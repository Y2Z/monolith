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

    fn remove(html: &str, selectors: &[&str]) -> String {
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let selectors: Vec<String> = selectors.iter().map(|s| s.to_string()).collect();
        html::strip_elements(&dom.document, &selectors, &[]);

        String::from_utf8_lossy(&html::serialize_document(
            dom,
            "".to_string(),
            &MonolithOptions::default(),
        ))
        .to_string()
    }

    fn wipe(html: &str, remove: &[&str], wipe: &[&str]) -> String {
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let remove: Vec<String> = remove.iter().map(|s| s.to_string()).collect();
        let wipe: Vec<String> = wipe.iter().map(|s| s.to_string()).collect();
        html::strip_elements(&dom.document, &remove, &wipe);

        String::from_utf8_lossy(&html::serialize_document(
            dom,
            "".to_string(),
            &MonolithOptions::default(),
        ))
        .to_string()
    }

    const PAGE: &str = "<html><head></head><body><main><div><iframe></iframe><p>a</p></div></main><iframe></iframe></body></html>";

    #[test]
    fn nothing_to_remove() {
        assert_eq!(remove(PAGE, &[]), PAGE);
    }

    #[test]
    fn by_name() {
        assert_eq!(
            remove(PAGE, &["iframe"]),
            "<html><head></head><body><main><div><p>a</p></div></main></body></html>"
        );
    }

    #[test]
    fn by_nested_path_keeps_everything_else() {
        assert_eq!(
            remove(PAGE, &["main div iframe"]),
            "<html><head></head><body><main><div><p>a</p></div></main><iframe></iframe></body></html>"
        );
    }

    #[test]
    fn path_can_skip_levels() {
        assert_eq!(
            remove(PAGE, &["main iframe"]),
            "<html><head></head><body><main><div><p>a</p></div></main><iframe></iframe></body></html>"
        );
    }

    #[test]
    fn wildcard() {
        assert_eq!(
            remove(PAGE, &["main *"]),
            "<html><head></head><body><main></main><iframe></iframe></body></html>"
        );
    }

    #[test]
    fn comma_separated() {
        assert_eq!(
            remove(PAGE, &["p, iframe"]),
            "<html><head></head><body><main><div></div></main></body></html>"
        );
    }

    #[test]
    fn wiping_turns_links_into_their_contents() {
        assert_eq!(
            wipe(
                "<html><head></head><body><p>a <a href=\"b\"><b>c</b> d</a> e</p></body></html>",
                &[],
                &["a"]
            ),
            "<html><head></head><body><p>a <b>c</b> d e</p></body></html>"
        );
    }

    #[test]
    fn wiping_leaves_empty_shells() {
        assert_eq!(
            wipe(
                "<html><head></head><body><iframe src=\"a\" srcdoc=\"b\"></iframe><audio src=\"c\"></audio><video src=\"d\" poster=\"e\"></video><object data=\"f\"></object><embed src=\"g\"></body></html>",
                &[],
                &["iframe,audio,video,object,embed"]
            ),
            "<html><head></head><body><iframe></iframe><audio></audio><video></video><object></object><embed></body></html>"
        );
    }

    #[test]
    fn wiping_images_replaces_their_source() {
        let result = wipe(
            "<html><head></head><body><img src=\"a.png\" srcset=\"b.png 2x\"></body></html>",
            &[],
            &["img"],
        );

        assert!(result.contains("src=\"data:image/png"));
        assert!(!result.contains(".png"));
    }

    #[test]
    fn wiping_leaves_what_only_scripts_know_the_meaning_of() {
        let result = wipe(
            "<html><head></head><body><img src=\"a.png\" data-src=\"c.png\"></body></html>",
            &[],
            &["img"],
        );

        assert!(result.contains("data-src=\"c.png\""));
    }

    #[test]
    fn wiping_scripts_never_exposes_their_code() {
        assert_eq!(
            wipe(
                "<html><head></head><body><script>a();</script><style>b{}</style></body></html>",
                &[],
                &["script,style"]
            ),
            "<html><head></head><body><script></script><style></style></body></html>"
        );
    }

    #[test]
    fn removing_beats_wiping() {
        assert_eq!(
            wipe(
                "<html><head></head><body><a href=\"b\">c</a></body></html>",
                &["a"],
                &["a"]
            ),
            "<html><head></head><body></body></html>"
        );
    }

    #[test]
    fn nested_paths_still_see_wiped_elements() {
        assert_eq!(
            wipe(
                "<html><head></head><body><a href=\"b\"><i>c</i>d</a></body></html>",
                &["a i"],
                &["a"]
            ),
            "<html><head></head><body>d</body></html>"
        );
    }

    #[test]
    fn wiping_inside_noscript() {
        assert_eq!(
            wipe(
                "<html><head></head><body><noscript><a href=\"b\">c</a></noscript></body></html>",
                &[],
                &["a"]
            ),
            "<html><head></head><body><noscript>c</noscript></body></html>"
        );
    }

    #[test]
    fn detects_targeted_elements() {
        let mut options = MonolithOptions::default();
        assert!(!html::targets_element_anywhere(&options, &["link"]));

        options.remove_elements = vec!["html link".to_string()];
        assert!(html::targets_element_anywhere(&options, &["link"]));

        options.remove_elements = vec!["form, link".to_string()];
        assert!(html::targets_element_anywhere(&options, &["link"]));

        options.remove_elements = vec!["main *".to_string()];
        assert!(!html::targets_element_anywhere(&options, &["link"]));

        options.remove_elements = vec![];
        options.wipe_elements = vec!["*".to_string()];
        assert!(html::targets_element_anywhere(&options, &["link"]));
    }

    #[test]
    fn inside_noscript() {
        assert_eq!(
            remove(
                "<html><head></head><body><noscript><img src=\"a.png\"><p>b</p></noscript></body></html>",
                &["noscript img"]
            ),
            "<html><head></head><body><noscript><p>b</p></noscript></body></html>"
        );
    }

    #[test]
    fn path_reaches_through_noscript() {
        assert_eq!(
            remove(
                "<html><head></head><body><main><noscript><img src=\"a.png\"></noscript></main><img src=\"b.png\"></body></html>",
                &["main img"]
            ),
            "<html><head></head><body><main><noscript></noscript></main><img src=\"b.png\"></body></html>"
        );
    }
}

#[cfg(test)]
mod failing {
    use monolith::core::MonolithOptions;
    use monolith::html;

    #[test]
    fn selectors_that_make_no_sense_get_ignored() {
        let page = "<html><head></head><body><p>a</p></body></html>";
        let dom = html::html_to_dom(page.as_bytes(), "".to_string());

        html::strip_elements(
            &dom.document,
            &["".to_string(), ",,".to_string(), "p >>> [".to_string()],
            &["".to_string()],
        );

        assert_eq!(
            String::from_utf8_lossy(&html::serialize_document(
                dom,
                "".to_string(),
                &MonolithOptions::default(),
            )),
            page
        );
    }
}
