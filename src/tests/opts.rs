//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::opts::Options;

    #[test]
    fn defaults() {
        let options: Options = Options::default();

        assert_eq!(options.target, str!());
        assert_eq!(options.no_css, false);
        assert_eq!(options.no_frames, false);
        assert_eq!(options.no_fonts, false);
        assert_eq!(options.no_images, false);
        assert_eq!(options.isolate, false);
        assert_eq!(options.no_js, false);
        assert_eq!(options.insecure, false);
        assert_eq!(options.no_metadata, false);
        assert_eq!(options.output, str!());
        assert_eq!(options.silent, false);
        assert_eq!(options.timeout, 0);
        assert_eq!(options.user_agent, "");
    }
}
