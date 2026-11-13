//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::{MonolithOptions, MonolithOutputFormat};

    #[test]
    fn defaults() {
        let options: MonolithOptions = MonolithOptions::default();

        assert_eq!(options.base_url, None);
        assert!(!options.remove_styles);
        assert_eq!(options.encoding, None);
        assert!(!options.remove_fonts);
        assert!(!options.remove_images);
        assert!(!options.isolate);
        assert!(!options.remove_js);
        assert!(!options.insecure);
        assert_eq!(options.output_format, MonolithOutputFormat::HTML);
        assert!(!options.simplify);
        assert!(options.remove_elements.is_empty());
        assert!(options.wipe_elements.is_empty());
        assert!(!options.silent);
        assert_eq!(options.timeout, 0);
        assert_eq!(options.user_agent, None);
    }
}
