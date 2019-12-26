use clap::{App, Arg};

#[derive(Default)]
pub struct AppArgs {
    pub url_target: String,
    pub no_css: bool,
    pub no_frames: bool,
    pub no_images: bool,
    pub no_js: bool,
    pub insecure: bool,
    pub isolate: bool,
    pub output: String,
    pub silent: bool,
    pub user_agent: String,
}

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.14; rv:66.0) Gecko/20100101 Firefox/66.0";

impl AppArgs {
    pub fn get() -> AppArgs {
        let app = App::new("monolith")
            .version(crate_version!())
            .author(crate_authors!("\n"))
            .about(crate_description!())
            .arg(
                Arg::with_name("url")
                    .required(true)
                    .takes_value(true)
                    .index(1)
                    .help("URL to download"),
            )
            // .args_from_usage("-a, --include-audio 'Embed audio sources'")
            .args_from_usage("-c, --no-css 'Ignore styles'")
            .args_from_usage("-f, --no-frames 'Exclude iframes'")
            .args_from_usage("-i, --no-images 'Remove images'")
            .args_from_usage("-I, --isolate 'Cut off from the Internet'")
            .args_from_usage("-j, --no-js 'Exclude JavaScript'")
            .args_from_usage("-k, --insecure 'Accept invalid X.509 (TLS) certificates'")
            .args_from_usage("-o, --output=[document.html] 'Write output to <file>'")
            .args_from_usage("-s, --silent 'Suppress verbosity'")
            .args_from_usage("-u, --user-agent=[Iceweasel] 'Custom User-Agent string'")
            // .args_from_usage("-v, --include-video 'Embed video sources'")
            .get_matches();
        let mut app_args = AppArgs::default();
        // Process the command
        app_args.url_target = app
            .value_of("url")
            .expect("please set target url")
            .to_string();
        app_args.no_css = app.is_present("no-css");
        app_args.no_frames = app.is_present("no-frames");
        app_args.no_images = app.is_present("no-images");
        app_args.no_js = app.is_present("no-js");
        app_args.insecure = app.is_present("insecure");
        app_args.isolate = app.is_present("isolate");
        app_args.silent = app.is_present("silent");
        app_args.output = app.value_of("output").unwrap_or("").to_string();
        app_args.user_agent = app
            .value_of("user-agent")
            .unwrap_or_else(|| DEFAULT_USER_AGENT)
            .to_string();
        app_args
    }
}
