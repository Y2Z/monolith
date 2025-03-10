//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::Options;

    #[test]
    fn defaults() {
        let options: Options = Options::default();

        assert!(!options.no_audio);
        assert_eq!(options.base_url, None);
        assert!(!options.no_css);
        assert_eq!(options.encoding, None);
        assert!(!options.no_frames);
        assert!(!options.no_fonts);
        assert!(!options.no_images);
        assert!(!options.isolate);
        assert!(!options.no_js);
        assert!(!options.insecure);
        assert!(!options.no_metadata);
        assert!(!options.silent);
        assert_eq!(options.timeout, 0);
        assert_eq!(options.user_agent, None);
        assert!(!options.no_video);
    }
}
