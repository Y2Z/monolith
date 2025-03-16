use std::fs;
use std::io::{self, Error as IoError, Write};
use std::process;

use chrono::prelude::*;
use clap::Parser;
use tempfile::Builder;

use monolith::cache::Cache;
use monolith::cookies::parse_cookie_file_contents;
use monolith::core::{create_monolithic_document, print_error_message, Options};

const ASCII: &str = " \
 _____     ______________    __________      ___________________    ___
|     \\   /              \\  |          |    |                   |  |   |
|      \\_/       __       \\_|    __    |    |    ___     ___    |__|   |
|               |  |            |  |   |    |   |   |   |   |          |
|   |\\     /|   |__|    _       |__|   |____|   |   |   |   |    __    |
|   | \\___/ |          | \\                      |   |   |   |   |  |   |
|___|       |__________|  \\_____________________|   |___|   |___|  |___|
";

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version)] // Read version from Cargo.toml
#[command(about = ASCII.to_owned() + "\n" + env!("CARGO_PKG_NAME") + " " + env!("CARGO_PKG_VERSION") + "\n\n" + env!("CARGO_PKG_DESCRIPTION"), long_about = None)]
struct Cli {
    /// Remove audio sources
    #[arg(short = 'a', long)]
    no_audio: bool,

    /// Set custom base URL
    #[arg(short, long, value_name = "http://localhost/")]
    base_url: Option<String>,

    /// Treat specified domains as blacklist
    #[arg(short = 'B', long)]
    blacklist_domains: bool,

    /// Remove CSS
    #[arg(short = 'c', long)]
    no_css: bool,

    /// Specify cookie file
    #[arg(short = 'C', long, value_name = "cookies.txt")]
    cookie_file: Option<String>,

    /// Specify domains to use for white/black-listing
    #[arg(short = 'd', long = "domain", value_name = "example.com")]
    domains: Vec<String>,

    /// Ignore network errors
    #[arg(short = 'e', long)]
    ignore_errors: bool,

    /// Enforce custom charset
    #[arg(short = 'E', long, value_name = "UTF-8")]
    encoding: Option<String>,

    /// Remove frames and iframes
    #[arg(short = 'f', long)]
    no_frames: bool,

    /// Remove fonts
    #[arg(short = 'F', long)]
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

    /// Exclude timestamp and source information
    #[arg(short = 'M', long)]
    no_metadata: bool,

    /// Replace NOSCRIPT elements with their contents
    #[arg(short = 'n', long)]
    unwrap_noscript: bool,

    /// File to write to, use - for STDOUT
    #[arg(short, long, value_name = "result.html")]
    output: Option<String>,

    /// Suppress verbosity
    #[arg(short, long)]
    quiet: bool,

    /// Adjust network request timeout
    #[arg(short, long, value_name = "60")]
    timeout: Option<u64>,

    /// Set custom User-Agent string
    #[arg(short, long, value_name = "Firefox")]
    user_agent: Option<String>,

    /// Remove video sources
    #[arg(short = 'v', long)]
    no_video: bool,

    /// URL or file path, use - for STDIN
    target: String,
}

enum Output {
    Stdout(io::Stdout),
    File(fs::File),
}

impl Output {
    fn new(destination: &str, document_title: &str) -> Result<Output, IoError> {
        if destination.is_empty() || destination.eq("-") {
            Ok(Output::Stdout(io::stdout()))
        } else {
            let datetime: &str = &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
            let final_destination = &destination
                .replace("%timestamp%", &datetime.replace(':', "_"))
                .replace(
                    "%title%",
                    document_title
                        .to_string()
                        .replace('/', "∕")
                        .replace('\\', "⧵")
                        .replace('<', "[")
                        .replace('>', "]")
                        .replace(':', "_")
                        .replace('\"', "''")
                        .replace('|', "_")
                        .replace('?', "")
                        .trim_start_matches('.'),
                );
            Ok(Output::File(fs::File::create(final_destination)?))
        }
    }

    fn write(&mut self, bytes: &Vec<u8>) -> Result<(), IoError> {
        match self {
            Output::Stdout(stdout) => {
                stdout.write_all(bytes)?;
                // Ensure newline at end of output
                if bytes.last() != Some(&b"\n"[0]) {
                    stdout.write_all(b"\n")?;
                }
                stdout.flush()
            }
            Output::File(file) => {
                file.write_all(bytes)?;
                // Ensure newline at end of output
                if bytes.last() != Some(&b"\n"[0]) {
                    file.write_all(b"\n")?;
                }
                file.flush()
            }
        }
    }
}

const CACHE_ASSET_FILE_SIZE_THRESHOLD: usize = 1024 * 10; // Minimum file size for on-disk caching (in bytes)
const DEFAULT_NETWORK_TIMEOUT: u64 = 120;
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0";

fn main() {
    let cli = Cli::parse();
    let cookie_file_path;
    let mut exit_code = 0;
    let mut options: Options = Options::default();
    let destination;

    // Process the command
    {
        options.base_url = cli.base_url;
        options.blacklist_domains = cli.blacklist_domains;
        options.encoding = cli.encoding;
        if !cli.domains.is_empty() {
            options.domains = Some(cli.domains);
        }
        options.ignore_errors = cli.ignore_errors;
        options.insecure = cli.insecure;
        options.isolate = cli.isolate;
        options.no_audio = cli.no_audio;
        options.no_css = cli.no_css;
        options.no_fonts = cli.no_fonts;
        options.no_frames = cli.no_frames;
        options.no_images = cli.no_images;
        options.no_js = cli.no_js;
        options.no_metadata = cli.no_metadata;
        options.no_video = cli.no_video;
        options.silent = cli.quiet;
        options.timeout = cli.timeout.unwrap_or(DEFAULT_NETWORK_TIMEOUT);
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
    let temp_cache_file = match Builder::new().prefix("monolith-").tempfile() {
        Ok(tempfile) => Some(tempfile),
        Err(_) => None,
    };
    let mut cache = Some(Cache::new(
        CACHE_ASSET_FILE_SIZE_THRESHOLD,
        if temp_cache_file.is_some() {
            Some(
                temp_cache_file
                    .as_ref()
                    .unwrap()
                    .path()
                    .display()
                    .to_string(),
            )
        } else {
            None
        },
    ));

    // Read and parse cookie file
    if let Some(opt_cookie_file) = cookie_file_path.clone() {
        match fs::read_to_string(&opt_cookie_file) {
            Ok(str) => match parse_cookie_file_contents(&str) {
                Ok(parsed_cookies_from_file) => {
                    options.cookies = parsed_cookies_from_file;
                }
                Err(_) => {
                    print_error_message(
                        &format!(
                            "could not parse specified cookie file \"{}\"",
                            opt_cookie_file
                        ),
                        &options,
                    );
                    process::exit(1);
                }
            },
            Err(_) => {
                print_error_message(
                    &format!(
                        "could not read specified cookie file \"{}\"",
                        opt_cookie_file
                    ),
                    &options,
                );
                process::exit(1);
            }
        }
    }

    // Retrieve target from source and output result
    match create_monolithic_document(cli.target, &options, &mut cache) {
        Ok((result, title)) => {
            // Define output
            let mut output = Output::new(
                &destination.unwrap_or(String::new()),
                &title.unwrap_or_default(),
            )
            .expect("could not prepare output");

            // Write result into STDOUT or file
            output.write(&result).expect("could not write output");
        }
        Err(error) => {
            print_error_message(&format!("Error: {}", error), &options);

            exit_code = 1;
        }
    }

    // Clean up (shred database file)
    cache.unwrap().destroy_database_file();

    if exit_code > 0 {
        process::exit(exit_code);
    }
}
