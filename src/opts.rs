use clap::{App, Arg};

#[derive(Default)]
pub struct Options {
    pub target: String,
    pub no_css: bool,
    pub no_fonts: bool,
    pub no_frames: bool,
    pub no_images: bool,
    pub no_js: bool,
    pub insecure: bool,
    pub isolate: bool,
    pub output: String,
    pub silent: bool,
    pub timeout: u64,
    pub user_agent: String,
    pub no_metadata: bool,
}

const ASCII: &str = " \
 _____     ______________    __________      ___________________    ___
|     \\   /              \\  |          |    |                   |  |   |
|      \\_/       __       \\_|    __    |    |    ___     ___    |__|   |
|               |  |            |  |   |    |   |   |   |   |          |
|   |\\     /|   |__|    _       |__|   |____|   |   |   |   |    __    |
|   | \\___/ |          | \\                      |   |   |   |   |  |   |
|___|       |__________|  \\_____________________|   |___|   |___|  |___|
";
const DEFAULT_NETWORK_TIMEOUT: u64 = 120;
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:73.0) Gecko/20100101 Firefox/73.0";

impl Options {
    pub fn from_args() -> Options {
        let app = App::new(env!("CARGO_PKG_NAME"))
            .version(crate_version!())
            .author(crate_authors!("\n"))
            .about(format!("{}\n{}", ASCII, crate_description!()).as_str())
            .arg(
                Arg::with_name("target")
                    .required(true)
                    .takes_value(true)
                    .index(1)
                    .help("URL or file path"),
            )
            // .args_from_usage("-a, --no-audio 'Removes audio sources'")
            .args_from_usage("-c, --no-css 'Removes CSS'")
            .args_from_usage("-f, --no-frames 'Removes frames and iframes'")
            .args_from_usage("-F, --no-fonts 'Removes fonts'")
            .args_from_usage("-i, --no-images 'Removes images'")
            .args_from_usage("-I, --isolate 'Cuts off document from the Internet'")
            .args_from_usage("-j, --no-js 'Removes JavaScript'")
            .args_from_usage("-k, --insecure 'Allows invalid X.509 (TLS) certificates'")
            .args_from_usage("-M, --no-metadata 'Excludes metadata information from the document'")
            .args_from_usage("-o, --output=[document.html] 'Writes output to <file>'")
            .args_from_usage("-s, --silent 'Suppresses verbosity'")
            .args_from_usage("-t, --timeout=[60] 'Adjusts network request timeout'")
            .args_from_usage("-u, --user-agent=[Firefox] 'Sets custom User-Agent string'")
            // .args_from_usage("-v, --no-video 'Removes video sources'")
            .get_matches();
        let mut options: Options = Options::default();

        // Process the command
        options.target = app
            .value_of("target")
            .expect("please set target")
            .to_string();
        options.no_css = app.is_present("no-css");
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
        options.user_agent = app
            .value_of("user-agent")
            .unwrap_or(DEFAULT_USER_AGENT)
            .to_string();

        options
    }
}
