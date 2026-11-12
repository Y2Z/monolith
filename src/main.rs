use std::fs;
use std::io::{self, Error as IoError, Read, Write};
use std::path::Path;
use std::process;

use clap::Parser;
use tempfile::{Builder, NamedTempFile};

#[cfg(feature = "browser")]
use monolith::browser;
use monolith::cache::Cache;
use monolith::cookies::{Cookie, parse_cookie_file_contents};
use monolith::core::{
    MonolithOptions, MonolithOutputFormat, create_monolithic_document,
    create_monolithic_document_from_data, create_output_file, format_output_path,
    print_error_message,
};
use monolith::session::Session;

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
    /// Set custom base URL, or pick the page to save out of a WARC or HAR archive
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

    /// Create missing directories in the output file path
    #[arg(short = 'D', long)]
    create_dirs: bool,

    /// Ignore network errors
    #[arg(short = 'e', long)]
    ignore_errors: bool,

    /// Enforce custom charset
    #[arg(short = 'E', long, value_name = "UTF-8")]
    encoding: Option<String>,

    /// Set output format: HTML (default), MHTML, or WARC (compressed if output file ends with .gz)
    #[arg(short = 'F', long, value_name = "HTML")]
    format: Option<MonolithOutputFormat>,

    /// Remove fonts
    #[arg(short = 'f', long)]
    no_fonts: bool,

    /// Remove images
    #[arg(short = 'i', long)]
    no_images: bool,

    /// Cut off document from the Internet
    #[arg(short = 'I', long)]
    isolate: bool,

    /// Remove JavaScript
    #[arg(short = 'j', long)]
    no_js: bool,

    /// Allow invalid X.509 (TLS) certificates
    #[arg(short = 'k', long)]
    insecure: bool,

    /// Replace NOSCRIPT elements with their contents
    #[arg(short = 'n', long)]
    unwrap_noscript: bool,

    /// File to write to, use - for STDOUT
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
    no_styles: bool,

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

    /// URL or file path, use - for STDIN
    target: String,
}

pub enum Output {
    Stdout(io::Stdout),
    File(fs::File),
}

impl Output {
    fn new(
        destination: &str,
        document_title: &str,
        format: MonolithOutputFormat,
        create_dirs: bool,
    ) -> Result<Output, IoError> {
        if destination.is_empty() || destination.eq("-") {
            Ok(Output::Stdout(io::stdout()))
        } else {
            let final_destination = format_output_path(destination, document_title, format);

            // Create directories in the path to output file if they are
            // missing and the user opted in via --create-dirs (like curl).
            // Without the flag a clear error is reported instead of creating
            // directory trees the user did not ask for.
            if let Some(parent) = Path::new(&final_destination).parent()
                && !parent.as_os_str().is_empty()
                && !parent.exists()
            {
                if create_dirs {
                    fs::create_dir_all(parent)?;
                } else {
                    return Err(IoError::new(
                        io::ErrorKind::NotFound,
                        format!(
                            "cannot write to '{}': directory '{}' does not exist (use --create-dirs to create it)",
                            final_destination,
                            parent.display()
                        ),
                    ));
                }
            }

            // Nothing gets replaced, files that would be get saved under names of their own
            let (file, _) = create_output_file(&final_destination)?;
            Ok(Output::File(file))
        }
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), IoError> {
        match self {
            Output::Stdout(stdout) => {
                stdout.write_all(bytes)?;
                stdout.flush()
            }
            Output::File(file) => {
                file.write_all(bytes)?;
                file.flush()
            }
        }
    }
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
        options.no_styles = cli.no_styles;
        options.no_fonts = cli.no_fonts;
        options.no_images = cli.no_images;
        options.no_js = cli.no_js;
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
        options.compress = options.output_format == MonolithOutputFormat::WARC
            && destination
                .as_deref()
                .is_some_and(|path| path.to_lowercase().ends_with(".gz"));
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
                    process::exit(1);
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
                process::exit(1);
            }
        }
    }

    // Initiate session
    let output_format = options.output_format.clone();
    let silent = options.silent;
    let session: Session = Session::new(cache, cookies, options);

    // The option is always there, builds that can't do what it asks for say so
    if cli.browser && cfg!(not(feature = "browser")) {
        if !silent {
            print_error_message("Error: this build of monolith has no browser support");
        }
        drop(temp_cache_file);
        process::exit(1);
    }

    if cli.browser && cli.target == "-" {
        if !silent {
            print_error_message("Error: browser can't be used with STDIN");
        }
        drop(temp_cache_file);
        process::exit(1);
    }

    // Retrieve target from source
    let result = if cli.target == "-" {
        // Read input from pipe (STDIN)
        create_monolithic_document_from_data(session, read_stdin(), None, None)
    } else {
        create_monolithic_document(session, cli.target)
    };

    // Output result
    match result {
        Ok((result, title)) => {
            // Define output
            match Output::new(
                &destination.unwrap_or(String::new()),
                &title.unwrap_or_default(),
                output_format,
                cli.create_dirs,
            ) {
                Ok(mut output) => {
                    // Write result into STDOUT or file
                    output.write(&result).expect("could not write output");
                }
                Err(error) => {
                    if !silent {
                        print_error_message(&format!("Error: {}", error));
                    }

                    exit_code = 1;
                }
            }
        }
        Err(error) => {
            if !silent {
                print_error_message(&format!("Error: {}", error));
            }

            exit_code = 1;
        }
    }

    // TODO: bring this back
    // Clean up (shred database file)
    //cache.unwrap().destroy_database_file();

    // Exiting doesn't run destructors
    drop(temp_cache_file);

    if exit_code > 0 {
        process::exit(exit_code);
    }
}
