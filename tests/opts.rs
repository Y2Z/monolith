//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use clap::Parser;

    use monolith::opts::Options;

    #[test]
    fn defaults() {
        let options: Options = Options::parse();

        assert_eq!(options.no_audio, false);
        assert_eq!(options.base_url, None);
        assert_eq!(options.no_css, false);
        assert_eq!(options.cookie_file, None);
        assert_eq!(options.encoding, None);
        assert_eq!(options.no_frames, false);
        assert_eq!(options.no_fonts, false);
        assert_eq!(options.no_images, false);
        assert_eq!(options.isolate, false);
        assert_eq!(options.no_js, false);
        assert_eq!(options.insecure, false);
        assert_eq!(options.no_metadata, false);
        assert_eq!(options.output, "-".to_string());
        assert_eq!(options.silent, false);
        assert_eq!(options.timeout, 60);
        assert_eq!(options.user_agent, "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:73.0) Gecko/20100101 Firefox/73.0");
        assert_eq!(options.no_video, false);

        assert_eq!(options.target, "-".to_string());
    }
}
