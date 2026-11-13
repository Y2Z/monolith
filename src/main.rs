use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Error as IoError, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use clap::Parser;
use tempfile::{Builder, NamedTempFile};

#[cfg(feature = "browser")]
use monolith::browser;
use monolith::cache::Cache;
use monolith::cookies::{Cookie, parse_cookie_file_contents};
use monolith::core::{
    MonolithOptions, MonolithOutputFormat, create_monolithic_document,
    create_monolithic_document_from_data, create_output_file, format_output_path,
    output_file_pattern, parse_target, print_error_message, probe_target, read_archives,
    untitled_name,
};
use monolith::net::Credentials;
use monolith::session::{Records, Session};
use url::Url;

const ASCII: &str = " \
 _____    _____________   __________     ___________________    ___
|     \\  /             \\ |          |   |                   |  |   |
|      \\/       __      \\|    __    |   |    ___     ___    |__|   |
|              |  |          |  |   |   |   |   |   |   |          |
|   |\\    /|   |__|          |__|   |___|   |   |   |   |    __    |
|   | \\__/ |          |\\                    |   |   |   |   |  |   |
|___|      |__________| \\___________________|   |___|   |___|  |___|
";
const CACHE_ASSET_FILE_SIZE_THRESHOLD: usize = 1024 * 10; // Minimum file size for on-disk caching (in bytes)
const INTERRUPTED_EXIT_CODE: i32 = 130; // What shells report for programs ended with Ctrl+C
const DEFAULT_NETWORK_TIMEOUT: u64 = 120; // Maximum time to retrieve each remote asset (in seconds)
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0";

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version)] // Read version from Cargo.toml
#[command(about = ASCII.to_owned() + "\n" + env!("CARGO_PKG_NAME") + " " + env!("CARGO_PKG_VERSION") + "\n\n" + env!("CARGO_PKG_DESCRIPTION"), long_about = None)]
struct Cli {
    /// Take everything from given archive (WARC, HAR, MHTML, WebArchive), never going online:
    /// targets are URLs of pages in it; can be given more than once
    #[arg(short = 'a', long = "archive", value_name = "crawl.warc")]
    archives: Vec<String>,

    /// Set custom base URL
    #[arg(short, long, value_name = "http://localhost/")]
    base_url: Option<String>,

    /// Let a browser render the page first, it runs on its own, with a clean temporary profile
    #[arg(short = 'B', long)]
    browser: bool,

    /// Specify cookie file
    #[arg(short = 'C', long, value_name = "cookies.txt")]
    cookie_file: Option<String>,

    /// Forbid retrieving assets from specified domains, see --whitelist
    #[arg(short = 'd', long = "domain", value_name = "example.com")]
    domains: Vec<String>,

    /// Ignore network errors
    #[arg(short = 'e', long)]
    ignore_errors: bool,

    /// Enforce custom charset
    #[arg(short = 'E', long, value_name = "UTF-8")]
    encoding: Option<String>,

    /// Set output format: HTML (default), MHTML, WARC, or WARCGZ (WARC compressed record by record)
    #[arg(short = 'F', long, value_name = "HTML")]
    format: Option<MonolithOutputFormat>,

    /// Remove fonts
    #[arg(short = 'f', long)]
    remove_fonts: bool,

    /// Remove images
    #[arg(short = 'i', long)]
    remove_images: bool,

    /// Cut off document from the Internet
    #[arg(short = 'I', long)]
    isolate: bool,

    /// Remove JavaScript from the document, pages still get to run it in the browser with -B
    #[arg(short = 'j', long)]
    remove_js: bool,

    /// Allow invalid X.509 (TLS) certificates
    #[arg(short = 'k', long)]
    insecure: bool,

    /// Replace NOSCRIPT elements with their contents
    #[arg(short = 'n', long)]
    unwrap_noscript: bool,

    /// File to write to, use - for STDOUT; %title%, %url%, %timestamp% and %extension% get
    /// replaced; directories (ending with a slash, or with no file extension) get files named
    /// %title%.%extension% [default: %title%.%extension%]
    #[arg(short, long, value_name = "result.html")]
    output: Option<String>,

    /// Suppress verbosity
    #[arg(short, long)]
    quiet: bool,

    /// Remove elements from the document, e.g. "audio,video" or "main div iframe"
    #[arg(short = 'r', long = "remove", value_name = "selector")]
    remove_elements: Vec<String>,

    /// Save a simplified version of the page, just the article and nothing around it
    #[arg(short = 'S', long)]
    simplify: bool,

    /// Remove styles
    #[arg(short = 's', long)]
    remove_styles: bool,

    /// Adjust network request timeout
    #[arg(short, long, value_name = "60")]
    timeout: Option<u64>,

    /// Set custom User-Agent string
    #[arg(short, long, value_name = "Firefox")]
    user_agent: Option<String>,

    /// Wipe elements, e.g. "a" turns links into text, "img,iframe" leaves them without sources
    #[arg(short = 'w', long = "wipe", value_name = "selector")]
    wipe_elements: Vec<String>,

    /// Only retrieve assets from domains specified with --domain
    #[arg(short = 'W', long, requires = "domains")]
    whitelist: bool,

    /// URLs or file paths, use - for STDIN; links between them lead to files they get saved in
    #[arg(required = true, value_name = "TARGETS")]
    targets: Vec<String>,
}

const DEFAULT_OUTPUT: &str = "%title%.%extension%";

/// Whether Ctrl+C has been pressed: what fails after that fails because of it, and it's up to
/// what handles it to clean up and exit.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Exits with given code, unless interrupted (the interruption gets to exit the way it does).
fn exit_with(code: i32) -> ! {
    if INTERRUPTED.load(Ordering::SeqCst) {
        loop {
            thread::park();
        }
    }
    process::exit(code);
}

/// Whether results are meant to go to STDOUT, rather than into files.
fn is_stdout(destination: &str) -> bool {
    destination.is_empty() || destination == "-"
}

/// Creates the file for a document to be saved in, named after it (never replacing anything).
/// Provides the file, and where it is.
fn create_file(
    destination: &str,
    name: &str,
    url: Option<&Url>,
    format: MonolithOutputFormat,
) -> Result<(fs::File, String), IoError> {
    // Nothing gets replaced, files that would be get saved under names of their own, and
    // directories that aren't there get created
    create_output_file(&format_output_path(destination, name, url, format))
}

/// What a file gets named after: the document's title, or its URL if it has none.
fn name_of(title: Option<&str>, url: Option<&Url>) -> String {
    match title.filter(|title| !title.trim().is_empty()) {
        Some(title) => title.to_string(),
        None => url.map_or("Untitled".to_string(), untitled_name),
    }
}

/// Output that several sessions write into, one after another.
#[derive(Clone, Default)]
struct SharedOutput(Arc<Mutex<Option<Box<dyn Write + Send>>>>);

impl SharedOutput {
    fn is_open(&self) -> bool {
        self.0.lock().unwrap().is_some()
    }
}

impl Write for SharedOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.0.lock().unwrap().as_mut() {
            Some(output) => output.write(buf),
            None => Ok(buf.len()),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self.0.lock().unwrap().as_mut() {
            Some(output) => output.write_all(buf),
            None => Ok(()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.0.lock().unwrap().as_mut() {
            Some(output) => output.flush(),
            None => Ok(()),
        }
    }
}

/// A document to be saved, and the file it's going into.
struct Claim {
    /// What was given
    target: String,
    /// Where it has turned out to be (none for STDIN)
    url: Option<Url>,
    /// Where it was said to be, if that's different from where it's turned out to be
    original_url: Option<Url>,
    /// What it is, as who: the same page can be something else for someone who's logged in
    identity: Option<String>,
    path: String,
}

pub fn read_stdin() -> Vec<u8> {
    let mut buffer: Vec<u8> = vec![];

    match io::stdin().lock().read_to_end(&mut buffer) {
        Ok(_) => buffer,
        Err(_) => buffer,
    }
}

fn main() {
    let cli = Cli::parse();
    let cookie_file_path;
    let mut exit_code = 0;
    let mut options: MonolithOptions = MonolithOptions::default();
    let destination;

    // Process the command
    {
        options.base_url = cli.base_url;
        #[cfg(feature = "browser")]
        {
            options.browser = cli.browser;
            options.browser_user_agent = cli.user_agent.clone();
        }
        options.encoding = cli.encoding;
        if !cli.domains.is_empty() {
            options.domains = Some(cli.domains);
        }
        options.whitelist_domains = cli.whitelist;
        options.ignore_errors = cli.ignore_errors;
        options.insecure = cli.insecure;
        options.isolate = cli.isolate;
        options.remove_styles = cli.remove_styles;
        options.remove_fonts = cli.remove_fonts;
        options.remove_images = cli.remove_images;
        options.remove_js = cli.remove_js;
        options.output_format = cli.format.unwrap_or_default();
        options.simplify = cli.simplify;
        options.remove_elements = cli.remove_elements;
        options.wipe_elements = cli.wipe_elements;
        options.silent = cli.quiet;
        options.timeout = cli.timeout.unwrap_or(DEFAULT_NETWORK_TIMEOUT);
        options.threads = 30;
        options.unwrap_noscript = cli.unwrap_noscript;
        if cli.user_agent.is_none() {
            options.user_agent = Some(DEFAULT_USER_AGENT.to_string());
        } else {
            options.user_agent = cli.user_agent;
        }

        cookie_file_path = cli.cookie_file;
        destination = cli.output.clone();
    }

    // Set up cache (attempt to create temporary file)
    let temp_cache_file: Option<NamedTempFile> = Builder::new().prefix("monolith-").tempfile().ok();
    let cache = Some(Cache::new(
        CACHE_ASSET_FILE_SIZE_THRESHOLD,
        temp_cache_file
            .as_ref()
            .map(|temp_cache_file| temp_cache_file.path().display().to_string()),
    ));

    // Clean up if interrupted, none of what's been retrieved is supposed to stay behind
    let cache_file_path = temp_cache_file
        .as_ref()
        .map(|file| file.path().to_path_buf());
    let _ = ctrlc::set_handler(move || {
        // Shutting the browser down makes whatever it's doing fail, that's not what to report
        INTERRUPTED.store(true, Ordering::SeqCst);
        #[cfg(feature = "browser")]
        browser::shut_down();
        if let Some(path) = &cache_file_path {
            let _ = fs::remove_file(path);
        }
        process::exit(INTERRUPTED_EXIT_CODE);
    });

    // Read and parse cookie file
    let mut cookies: Option<Vec<Cookie>> = None;
    if let Some(opt_cookie_file) = cookie_file_path.clone() {
        match fs::read_to_string(&opt_cookie_file) {
            Ok(str) => match parse_cookie_file_contents(&str) {
                Ok(parsed_cookies_from_file) => {
                    cookies = Some(parsed_cookies_from_file);
                }
                Err(_) => {
                    if !options.silent {
                        print_error_message(&format!(
                            "could not parse specified cookie file \"{}\"",
                            opt_cookie_file
                        ));
                    }
                    drop(temp_cache_file);
                    exit_with(1);
                }
            },
            Err(_) => {
                if !options.silent {
                    print_error_message(&format!(
                        "could not read specified cookie file \"{}\"",
                        opt_cookie_file
                    ));
                }
                drop(temp_cache_file);
                exit_with(1);
            }
        }
    }

    let silent = options.silent;
    let report = |target: &str, error: &dyn std::fmt::Display| {
        if !silent && !INTERRUPTED.load(Ordering::SeqCst) {
            match cli.targets.len() {
                1 => print_error_message(&format!("Error: {}", error)),
                _ => print_error_message(&format!("Error: {}: {}", target, error)),
            }
        }
    };
    let exit = |message: &str, temp_cache_file: Option<NamedTempFile>| -> ! {
        if !silent {
            print_error_message(&format!("Error: {}", message));
        }
        drop(temp_cache_file);
        exit_with(1);
    };

    // The option is always there, builds that can't do what it asks for say so
    if cli.browser && cfg!(not(feature = "browser")) {
        exit(
            "this build of monolith has no browser support",
            temp_cache_file,
        );
    }
    let stdin_count = cli.targets.iter().filter(|target| *target == "-").count();
    if cli.browser && stdin_count > 0 {
        exit("browser can't be used with STDIN", temp_cache_file);
    }
    if stdin_count > 1 {
        exit("STDIN can only be read once", temp_cache_file);
    }

    // Archives are all there is to work with, if there are any
    let mut archives: Vec<Vec<u8>> = vec![];
    for path in &cli.archives {
        match fs::read(path) {
            Ok(data) => archives.push(data),
            Err(error) => exit(
                &format!("could not read archive {}: {}", path, error),
                temp_cache_file,
            ),
        }
    }
    let archives = match read_archives(archives) {
        Ok(archives) => archives,
        Err(error) => exit(&error.to_string(), temp_cache_file),
    };
    if !archives.is_empty() {
        if cli.browser {
            exit(
                "browser would go online, archives are all there is with -a",
                temp_cache_file,
            );
        }
        if stdin_count > 0 {
            exit(
                "targets are URLs of pages in archives with -a, STDIN can't be one",
                temp_cache_file,
            );
        }
    }

    // Saving the same thing twice wouldn't do anything
    let mut targets: Vec<String> = vec![];
    for target in &cli.targets {
        let url = parse_target(target).ok();
        let is_known = |known: &String| match &url {
            Some(url) => parse_target(known).ok().as_ref() == Some(url),
            None => known == target,
        };
        if target == "-" || !targets.iter().any(is_known) {
            targets.push(target.clone());
        }
    }

    let output_format = options.output_format.clone();
    // Directories get files named the default way
    let destination = match destination {
        Some(destination) if !is_stdout(&destination) => {
            output_file_pattern(&destination, DEFAULT_OUTPUT)
        }
        Some(destination) => destination,
        None => DEFAULT_OUTPUT.to_string(),
    };
    let new_session = || {
        let mut session = Session::new(cache.clone(), cookies.clone(), options.clone());
        if !archives.is_empty() {
            session.use_archives(&archives);
        }
        session
    };
    let save = |session: Session, target: &str| match target {
        "-" => create_monolithic_document_from_data(session, read_stdin(), None, None),
        _ => create_monolithic_document(session, target.to_string()),
    };

    if output_format.is_warc() || targets.len() == 1 {
        // One file for everything: it gets named after the first document that makes it in,
        // documents after it get added to it
        let output = SharedOutput::default();
        // What one target has put into the archive, others don't put in again
        let records = Records::default();
        for target in &targets {
            let mut session = new_session();
            session.share_records(&records);
            let shared = output.clone();
            let destination = destination.clone();
            let output_format = output_format.clone();
            let url = (target != "-").then(|| parse_target(target).ok()).flatten();
            session.set_destination(Box::new(move |title| {
                if !shared.is_open() {
                    let opened: Box<dyn Write + Send> = match is_stdout(&destination) {
                        true => Box::new(BufWriter::new(io::stdout())),
                        false => {
                            let name = name_of(title, url.as_ref());
                            let (file, _) =
                                create_file(&destination, &name, url.as_ref(), output_format)?;
                            Box::new(BufWriter::new(file))
                        }
                    };
                    *shared.0.lock().unwrap() = Some(opened);
                }
                Ok(Box::new(shared) as Box<dyn Write + Send>)
            }));

            if let Err(error) = save(session, target) {
                report(target, &error);
                exit_code = 1;
            }
        }
    } else if is_stdout(&destination) {
        exit(
            "several documents can't all go to STDOUT, they can as WARC",
            temp_cache_file,
        );
    } else {
        // Files get claimed before anything gets saved, documents have to know what files
        // they link to are called
        let mut claims: Vec<Claim> = vec![];
        for target in &targets {
            let (title, url, original_url) = match target.as_str() {
                "-" => (None, None, None),
                _ => match probe_target(new_session(), target) {
                    Ok(probed) => {
                        let original_url =
                            parse_target(target).ok().filter(|url| *url != probed.url);
                        (probed.title, Some(probed.url), original_url)
                    }
                    Err(error) => {
                        report(target, &error);
                        exit_code = 1;
                        continue;
                    }
                },
            };
            let mut credentials = Credentials::default();
            if let Ok(original_url) = parse_target(target) {
                credentials.take_from(&original_url);
            }
            let identity = url.as_ref().map(|url| credentials.cache_key(url));
            // Same document given twice (or redirected to) gets saved once
            if identity.is_some() && claims.iter().any(|claim| claim.identity == identity) {
                continue;
            }

            // Named after where it was said to be, the way it was given
            let given_url = parse_target(target).ok();
            match create_file(
                &destination,
                &name_of(title.as_deref(), url.as_ref()),
                given_url.as_ref(),
                output_format.clone(),
            ) {
                Ok((_, path)) => claims.push(Claim {
                    target: target.clone(),
                    url,
                    original_url,
                    identity,
                    path,
                }),
                Err(error) => {
                    report(target, &error);
                    exit_code = 1;
                }
            }
        }

        let files: Vec<(Url, PathBuf)> = claims
            .iter()
            .flat_map(|claim| {
                [&claim.url, &claim.original_url]
                    .into_iter()
                    .flatten()
                    .map(|url| (url.clone(), PathBuf::from(&claim.path)))
            })
            .collect();
        for claim in &claims {
            let mut session = new_session();
            // It gets saved from where it has turned out to be, as whoever it was asked for as
            if let Ok(original_url) = parse_target(&claim.target) {
                session.take_credentials(&original_url);
            }
            session.link_files(Path::new(&claim.path), &files);
            let path = claim.path.clone();
            session.set_destination(Box::new(move |_| {
                let file = OpenOptions::new().write(true).truncate(true).open(&path)?;
                Ok(Box::new(BufWriter::new(file)) as Box<dyn Write + Send>)
            }));

            // Where it has turned out to be is where it gets saved from
            let target = claim
                .url
                .as_ref()
                .map_or(claim.target.clone(), |url| url.to_string());
            if let Err(error) = save(session, &target) {
                report(&claim.target, &error);
                exit_code = 1;
            }
        }
    }

    // TODO: bring this back
    // Clean up (shred database file)
    //cache.unwrap().destroy_database_file();

    // Exiting doesn't run destructors
    drop(temp_cache_file);

    exit_with(exit_code);
}
