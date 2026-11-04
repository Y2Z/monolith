//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::MonolithOptions;
    use monolith::html;

    #[test]
    fn isolated() {
        let mut options = MonolithOptions::default();
        options.isolate = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(
            csp_content,
            "default-src 'unsafe-eval' 'unsafe-inline' data:;"
        );
    }

    #[test]
    fn no_styles() {
        let mut options = MonolithOptions::default();
        options.no_styles = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "style-src 'none';");
    }

    #[test]
    fn no_fonts() {
        let mut options = MonolithOptions::default();
        options.no_fonts = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "font-src 'none';");
    }

    #[test]
    fn no_frames() {
        let mut options = MonolithOptions::default();
        options.wipe_elements = vec!["frame,iframe".to_string()];
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "frame-src 'none'; child-src 'none';");
    }

    #[test]
    fn frames_targeted_by_selectors() {
        let mut options = MonolithOptions::default();
        options.wipe_elements = vec!["iframe".to_string()];

        assert_eq!(
            html::compose_csp(&options),
            "frame-src 'none'; child-src 'none';"
        );
    }

    #[test]
    fn objects_targeted_by_selectors() {
        let mut options = MonolithOptions::default();
        options.remove_elements = vec!["object".to_string()];

        assert_eq!(html::compose_csp(&options), "object-src 'none';");
    }

    #[test]
    fn nested_selectors_do_not_affect_csp() {
        let mut options = MonolithOptions::default();
        options.remove_elements = vec!["main iframe".to_string()];

        assert_eq!(html::compose_csp(&options), "");
    }

    #[test]
    fn no_js() {
        let mut options = MonolithOptions::default();
        options.no_js = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "script-src 'none'; object-src 'none';");
    }

    #[test]
    fn no_images() {
        let mut options = MonolithOptions::default();
        options.no_images = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "img-src data:;");
    }

    #[test]
    fn all() {
        let mut options = MonolithOptions::default();
        options.isolate = true;
        options.no_styles = true;
        options.no_fonts = true;
        options.wipe_elements = vec!["frame,iframe".to_string()];
        options.no_js = true;
        options.no_images = true;
        let csp_content = html::compose_csp(&options);

        assert_eq!(
            csp_content,
            "default-src 'unsafe-eval' 'unsafe-inline' data:; style-src 'none'; font-src 'none'; frame-src 'none'; child-src 'none'; script-src 'none'; object-src 'none'; img-src data:;"
        );
    }
}
