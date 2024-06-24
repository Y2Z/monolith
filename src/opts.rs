const DEFAULT_NETWORK_TIMEOUT: u64 = 60;
const DEFAULT_USER_AGENT: &'static str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:73.0) Gecko/20100101 Firefox/73.0";

// #[derive(Default)]
pub struct Options {
    pub base_url: Option<String>,
    pub blacklist_domains: bool,
    pub cookie_file: Option<String>,
    pub domains: Option<Vec<String>>,
    pub encoding: Option<String>,
    pub ignore_errors: bool,
    pub insecure: bool,
    pub isolate: bool,
    pub no_audio: bool,
    pub no_css: bool,
    pub no_frames: bool,
    pub no_fonts: bool,
    pub no_images: bool,
    pub no_js: bool,
    pub no_metadata: bool,
    pub no_video: bool,
    pub output: String,
    pub silent: bool,
    pub target: String,
    pub timeout: u64,
    pub unwrap_noscript: bool,
    pub user_agent: Option<String>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            base_url: None,
            blacklist_domains: false,
            cookie_file: None,
            domains: None,
            encoding: None,
            ignore_errors: false,
            insecure: false,
            isolate: false,
            no_audio: false,
            no_css: false,
            no_frames: false,
            no_fonts: false,
            no_images: false,
            no_js: false,
            no_metadata: false,
            no_video: false,
            output: String::from("-"),
            silent: false,
            target: String::from("-"),
            timeout: DEFAULT_NETWORK_TIMEOUT,
            unwrap_noscript: false,
            user_agent: DEFAULT_USER_AGENT,
        }
    }
}
