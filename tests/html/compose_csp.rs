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
        let options = MonolithOptions {
            isolate: true,
            ..Default::default()
        };
        let csp_content = html::compose_csp(&options);

        assert_eq!(
            csp_content,
            "default-src 'unsafe-eval' 'unsafe-inline' data:;"
        );
    }

    #[test]
    fn remove_styles() {
        let options = MonolithOptions {
            remove_styles: true,
            ..Default::default()
        };
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "style-src 'none';");
    }

    #[test]
    fn remove_fonts() {
        let options = MonolithOptions {
            remove_fonts: true,
            ..Default::default()
        };
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "font-src 'none';");
    }

    #[test]
    fn no_frames() {
        let options = MonolithOptions {
            wipe_elements: vec!["frame,iframe".to_string()],
            ..Default::default()
        };
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "frame-src 'none'; child-src 'none';");
    }

    #[test]
    fn frames_targeted_by_selectors() {
        let options = MonolithOptions {
            wipe_elements: vec!["iframe".to_string()],
            ..Default::default()
        };

        assert_eq!(
            html::compose_csp(&options),
            "frame-src 'none'; child-src 'none';"
        );
    }

    #[test]
    fn objects_targeted_by_selectors() {
        let options = MonolithOptions {
            remove_elements: vec!["object".to_string()],
            ..Default::default()
        };

        assert_eq!(html::compose_csp(&options), "object-src 'none';");
    }

    #[test]
    fn nested_selectors_do_not_affect_csp() {
        let options = MonolithOptions {
            remove_elements: vec!["main iframe".to_string()],
            ..Default::default()
        };

        assert_eq!(html::compose_csp(&options), "");
    }

    #[test]
    fn remove_js() {
        let options = MonolithOptions {
            remove_js: true,
            ..Default::default()
        };
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "script-src 'none'; object-src 'none';");
    }

    #[test]
    fn remove_images() {
        let options = MonolithOptions {
            remove_images: true,
            ..Default::default()
        };
        let csp_content = html::compose_csp(&options);

        assert_eq!(csp_content, "img-src data:;");
    }

    #[test]
    fn all() {
        let options = MonolithOptions {
            isolate: true,
            remove_styles: true,
            remove_fonts: true,
            wipe_elements: vec!["frame,iframe".to_string()],
            remove_js: true,
            remove_images: true,
            ..Default::default()
        };
        let csp_content = html::compose_csp(&options);

        assert_eq!(
            csp_content,
            "default-src 'unsafe-eval' 'unsafe-inline' data:; style-src 'none'; font-src 'none'; frame-src 'none'; child-src 'none'; script-src 'none'; object-src 'none'; img-src data:;"
        );
    }
}
