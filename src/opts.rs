use clap::Parser;
use std::env;

// #[derive(Default)]
// pub struct Options {
//     pub base_url: Option<String>,
//     pub blacklist_domains: bool,
//     pub cookie_file: Option<String>,
//     pub cookies: Vec<Cookie>,
//     pub domains: Option<Vec<String>>,
//     pub encoding: Option<String>,
//     pub ignore_errors: bool,
//     pub insecure: bool,
//     pub isolate: bool,
//     pub no_audio: bool,
//     pub no_color: bool,
//     pub no_css: bool,
//     pub no_frames: bool,
//     pub no_fonts: bool,
//     pub no_images: bool,
//     pub no_js: bool,
//     pub no_metadata: bool,
//     pub no_video: bool,
//     pub output: String,
//     pub silent: bool,
//     pub target: String,
//     pub timeout: u64,
//     pub unwrap_noscript: bool,
//     pub user_agent: Option<String>,
// }

const ASCII: &'static str = " \
 _____     ______________    __________      ___________________    ___
|     \\   /              \\  |          |    |                   |  |   |
|      \\_/       __       \\_|    __    |    |    ___     ___    |__|   |
|               |  |            |  |   |    |   |   |   |   |          |
|   |\\     /|   |__|    _       |__|   |____|   |   |   |   |    __    |
|   | \\___/ |          | \\                      |   |   |   |   |  |   |
|___|       |__________|  \\_____________________|   |___|   |___|  |___|
";
const DEFAULT_NETWORK_TIMEOUT: u64 = 120;
const DEFAULT_USER_AGENT: &'static str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:73.0) Gecko/20100101 Firefox/73.0";

#[derive(Parser, Debug)]
#[command(
    version,
    // author = format!("\n{}\n\n", env!("CARGO_PKG_AUTHORS").replace(':', "\n")).as_str(),
    // about = format!("{}\n{}", ASCII, env!("CARGO_PKG_DESCRIPTION")).as_str()
)]
pub struct Options {
    /// Remove audio sources
    #[arg(short = 'a', long = "no-audio")]
    pub no_audio: bool,

    /// Set custom base URL
    #[arg(short = 'b', long = "base-url")]
    pub base_url: Option<String>,

    /// Treat list of specified domains as blacklist
    #[arg(short = 'B', long = "blacklist-domains")]
    pub blacklist_domains: bool,

    /// Remove CSS
    #[arg(short = 'c', long = "no-css")]
    pub no_css: bool,

    /// Specify cookie file
    #[arg(short = 'C', long = "cookies")]
    pub cookie_file: Option<String>,

    /// Specify domains to use for white/black-listing
    #[arg(short = 'd', long = "domains")]
    pub domains: Option<Vec<String>>,

    /// Ignore network errors
    #[arg(short = 'e', long = "ignore-errors")]
    pub ignore_errors: bool,

    /// Enforce custom charset
    #[arg(short = 'E', long = "encoding")]
    pub encoding: Option<String>,

    /// Remove frames and iframes
    #[arg(short = 'f', long = "no-frames")]
    pub no_frames: bool,

    /// Remove fonts
    #[arg(short = 'F', long = "no-fonts")]
    pub no_fonts: bool,

    /// Remove images
    #[arg(short = 'i', long = "no-images")]
    pub no_images: bool,

    /// Cut off document from the Internet
    #[arg(short = 'I', long = "isolate")]
    pub isolate: bool,

    /// Remove JavaScript
    #[arg(short = 'j', long = "no-js")]
    pub no_js: bool,

    /// Allow invalid X.509 (TLS) certificates
    #[arg(short = 'k', long = "insecure")]
    pub insecure: bool,

    /// Exclude timestamp and source information
    #[arg(short = 'M', long = "no-metadata")]
    pub no_metadata: bool,

    /// Replace NOSCRIPT elements with their contents
    #[arg(short = 'n', long = "unwrap-noscript")]
    pub unwrap_noscript: bool,

    /// Write output to <file>, use - for STDOUT
    #[arg(short = 'o', long = "output")]
    pub output: String,

    /// Suppress verbosity
    #[arg(short = 's', long = "silent")]
    pub silent: bool,

    /// Adjust network request timeout
    // #[arg(short = 't', long = "timeout", default_value = format!("{}", DEFAULT_NETWORK_TIMEOUT).as_str())]
    #[arg(short = 't', long = "timeout", default_value = "60")]
    pub timeout: u64,

    /// Set custom User-Agent string
    #[arg(short = 'u', long = "user-agent")]
    pub user_agent: Option<String>,

    /// Remove video sources
    #[arg(short = 'v', long = "no-video")]
    pub no_video: bool,

    /// URL or file path, use - for STDIN
    #[arg(name = "target", default_value = "-")]
    pub target: String,

/*
    /// Files to cat
    #[arg(name = "FILES", default_value = "-")]
    files: Vec<String>,
    /// Print line numbers
    #[arg(short, long = "number")]
    number_lines: bool,
    /// Print line numbers for nonblank lines
    #[arg(short = 'b', long = "number-nonblank", conflicts_with = "number_lines")]
    number_nonblank_lines: bool,
    /// Show $ at the end of each line
    #[arg(short = 'E', long = "show-ends")]
    show_ends: bool,
*/
}

/*
impl Options {
    pub fn from_args() -> Options {
        let app = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author(format!("\n{}\n\n", env!("CARGO_PKG_AUTHORS").replace(':', "\n")).as_str())
            .about(format!("{}\n{}", ASCII, env!("CARGO_PKG_DESCRIPTION")).as_str())
            .args_from_usage("-a, --no-audio 'Remove audio sources'")
            .args_from_usage("-b, --base-url=[http://localhost/] 'Set custom base URL'")
            .args_from_usage(
                "-B, --blacklist-domains 'Treat list of specified domains as blacklist'",
            )
            .args_from_usage("-c, --no-css 'Remove CSS'")
            .args_from_usage("-C, --cookies=[cookies.txt] 'Specify cookie file'")
            .arg(
                Arg::with_name("domains")
                    .short('d')
                    .long("domain")
                    .takes_value(true)
                    .value_name("example.com")
                    .action(ArgAction::Append)
                    .help("Specify domains to use for white/black-listing"),
            )
            .args_from_usage("-e, --ignore-errors 'Ignore network errors'")
            .args_from_usage("-E, --encoding=[UTF-8] 'Enforce custom charset'")
            .args_from_usage("-f, --no-frames 'Remove frames and iframes'")
            .args_from_usage("-F, --no-fonts 'Remove fonts'")
            .args_from_usage("-i, --no-images 'Remove images'")
            .args_from_usage("-I, --isolate 'Cut off document from the Internet'")
            .args_from_usage("-j, --no-js 'Remove JavaScript'")
            .args_from_usage("-k, --insecure 'Allow invalid X.509 (TLS) certificates'")
            .args_from_usage("-M, --no-metadata 'Exclude timestamp and source information'")
            .args_from_usage(
                "-n, --unwrap-noscript 'Replace NOSCRIPT elements with their contents'",
            )
            .args_from_usage(
                "-o, --output=[document.html] 'Write output to <file>, use - for STDOUT'",
            )
            .args_from_usage("-s, --silent 'Suppress verbosity'")
            .args_from_usage("-t, --timeout=[60] 'Adjust network request timeout'")
            .args_from_usage("-u, --user-agent=[Firefox] 'Set custom User-Agent string'")
            .args_from_usage("-v, --no-video 'Remove video sources'")
            .arg(
                Arg::with_name("target")
                    .required(true)
                    .takes_value(true)
                    .index(1)
                    .help("URL or file path, use - for STDIN"),
            )
            .get_matches();
        let mut options: Options = Options::default();

        // Process the command
        options.target = app
            .value_of("target")
            .expect("please set target")
            .to_string();
        options.no_audio = app.is_present("no-audio");
        if let Some(base_url) = app.value_of("base-url") {
            options.base_url = Some(base_url.to_string());
        }
        options.blacklist_domains = app.is_present("blacklist-domains");
        options.no_css = app.is_present("no-css");
        if let Some(cookie_file) = app.value_of("cookies") {
            options.cookie_file = Some(cookie_file.to_string());
        }
        if let Some(encoding) = app.value_of("encoding") {
            options.encoding = Some(encoding.to_string());
        }
        if let Some(domains) = app.get_many::<String>("domains") {
            let list_of_domains: Vec<String> = domains.map(|v| v.clone()).collect::<Vec<_>>();
            options.domains = Some(list_of_domains);
        }
        options.ignore_errors = app.is_present("ignore-errors");
        options.no_frames = app.is_present("no-frames");
        options.no_fonts = app.is_present("no-fonts");
        options.no_images = app.is_present("no-images");
        options.isolate = app.is_present("isolate");
        options.no_js = app.is_present("no-js");
        options.insecure = app.is_present("insecure");
        options.no_metadata = app.is_present("no-metadata");
        options.output = app.value_of("output").unwrap_or("").to_string();
        options.silent = app.is_present("silent");
        options.timeout = app
            .value_of("timeout")
            .unwrap_or(&DEFAULT_NETWORK_TIMEOUT.to_string())
            .parse::<u64>()
            .unwrap();
        if let Some(user_agent) = app.value_of("user-agent") {
            options.user_agent = Some(user_agent.to_string());
        } else {
            options.user_agent = Some(DEFAULT_USER_AGENT.to_string());
        }
        options.unwrap_noscript = app.is_present("unwrap-noscript");
        options.no_video = app.is_present("no-video");

        options
    }
}
*/
