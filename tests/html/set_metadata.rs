#[cfg(test)]
mod helpers {
    use monolith::core::MonolithOptions;
    use monolith::html::{html_to_dom, serialize_document, set_metadata};
    use monolith::url::Url;

    pub fn save(html: &str, source: Option<&str>) -> String {
        let dom = html_to_dom(html.as_bytes(), "utf-8".to_string());
        let source = source.map(|source| Url::parse(source).unwrap());
        set_metadata(&dom, source.as_ref());

        String::from_utf8(serialize_document(
            dom,
            "utf-8".to_string(),
            &MonolithOptions::default(),
        ))
        .unwrap()
    }

    /// Content of the META element with given name, and how many of those there are.
    pub fn meta(html: &str, name: &str) -> (Option<String>, usize) {
        let start = format!("<meta name=\"{}\" content=\"", name);
        let content = html.find(&start).map(|position| {
            let rest = &html[position + start.len()..];
            rest[..rest.find('"').unwrap()].to_string()
        });

        (content, html.matches(&start).count())
    }
}

#[cfg(test)]
mod passing {
    use chrono::DateTime;

    use super::helpers::{meta, save};

    #[test]
    fn says_what_saved_it_when_and_where_from() {
        let html = save("<title>t</title>", Some("https://example.com/page?a=1"));

        assert_eq!(
            meta(&html, "generator"),
            (Some(format!("monolith {}", env!("CARGO_PKG_VERSION"))), 1)
        );
        assert_eq!(
            meta(&html, "dcterms.source"),
            (Some("https://example.com/page?a=1".to_string()), 1)
        );
        let (created, count) = meta(&html, "dcterms.created");
        assert_eq!(count, 1);
        assert!(DateTime::parse_from_rfc3339(&created.unwrap()).is_ok());
        assert!(html.contains("</title><meta name=\"generator\""));
    }

    #[test]
    fn replaces_what_was_there() {
        let html = save(
            "<head><meta name=Generator content=\"WordPress 6\">\
             <meta name=dcterms.source content=https://other.test/>\
             <meta name=DCTERMS.CREATED content=1999></head>\
             <body><meta name=generator content=\"Hugo\"></body>",
            Some("https://example.com/"),
        );

        assert!(!html.contains("WordPress"));
        assert!(!html.contains("Hugo"));
        assert!(!html.contains("other.test"));
        assert!(!html.contains("1999"));
        assert_eq!(meta(&html, "generator").1, 1);
        assert_eq!(meta(&html, "dcterms.source").1, 1);
        assert_eq!(meta(&html, "dcterms.created").1, 1);
    }

    #[test]
    fn leaves_credentials_and_fragments_out() {
        let html = save("", Some("https://user:password@example.com/page#part"));

        assert_eq!(
            meta(&html, "dcterms.source").0,
            Some("https://example.com/page".to_string())
        );
        assert!(!html.contains("password"));
    }

    #[test]
    fn keeps_where_saved_pages_came_from_when_there_is_nothing_better() {
        for source in [
            None,
            Some("file:///home/user/page.html"),
            Some("data:text/html,"),
        ] {
            let html = save(
                "<meta name=dcterms.source content=https://original.test/>",
                source,
            );

            assert_eq!(
                meta(&html, "dcterms.source"),
                (Some("https://original.test/".to_string()), 1)
            );
            assert!(!html.contains("/home/user"));
        }
    }
}

#[cfg(test)]
mod failing {
    use super::helpers::{meta, save};

    #[test]
    fn local_sources_are_not_given_away() {
        for source in [
            None,
            Some("file:///home/user/page.html"),
            Some("data:text/html,x"),
        ] {
            let html = save("<p>x", source);

            assert_eq!(meta(&html, "dcterms.source").1, 0);
            assert!(!html.contains("/home/user"));
            assert_eq!(meta(&html, "generator").1, 1);
        }
    }
}
