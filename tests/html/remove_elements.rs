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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let selectors: Vec<String> = selectors.iter().map(|s| s.to_string()).collect();
        html::remove_elements(&dom.document, &selectors);

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
        let dom = html::html_to_dom(&page.as_bytes().to_vec(), "".to_string());

        html::remove_elements(
            &dom.document,
            &["".to_string(), ",,".to_string(), "p >>> [".to_string()],
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
