//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::html;
    use monolith::opts::Options;

    #[test]
    fn isolated() {
        let mut options = Options::default();
        options.isolate = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "default-src 'unsafe-inline' data:;");
    }

    #[test]
    fn no_css() {
        let mut options = Options::default();
        options.no_css = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "style-src 'none';");
    }

    #[test]
    fn no_fonts() {
        let mut options = Options::default();
        options.no_fonts = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "font-src 'none';");
    }

    #[test]
    fn no_frames() {
        let mut options = Options::default();
        options.no_frames = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "frame-src 'none'; child-src 'none';");
    }

    #[test]
    fn no_js() {
        let mut options = Options::default();
        options.no_js = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "script-src 'none';");
    }

    #[test]
    fn no_images() {
        let mut options = Options::default();
        options.no_images = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "img-src data:;");
    }

    #[test]
    fn all() {
        let mut options = Options::default();
        options.isolate = true;
        options.no_css = true;
        options.no_fonts = true;
        options.no_frames = true;
        options.no_js = true;
        options.no_images = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "default-src 'unsafe-inline' data:; style-src 'none'; font-src 'none'; frame-src 'none'; child-src 'none'; script-src 'none'; img-src data:;");
    }
}
