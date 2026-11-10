#[cfg(test)]
mod helpers {
    use serde_json::{Value, json};

    pub fn entry(url: &str, status: u64, mime_type: &str, text: Option<&str>) -> Value {
        json!({
            "request": {"method": "GET", "url": url},
            "response": {
                "status": status,
                "headers": [],
                "content": {"mimeType": mime_type, "text": text},
            },
        })
    }

    pub fn har(entries: Vec<Value>) -> Vec<u8> {
        json!({"log": {"entries": entries}})
            .to_string()
            .into_bytes()
    }
}

#[cfg(test)]
mod passing {
    use serde_json::json;

    use monolith::har::parse_har;

    use super::helpers::{entry, har};

    #[test]
    fn first_document_is_the_page() {
        let (page, _) = parse_har(
            &har(vec![
                entry("https://a.test/a.css", 200, "text/css", Some("a{}")),
                entry("https://a.test/", 200, "text/html", Some("<p>one")),
                entry("https://a.test/b", 200, "text/html", Some("<p>two")),
            ]),
            None,
        )
        .unwrap();

        assert_eq!(page.url, "https://a.test/");
        assert_eq!(page.html, "<p>one");
        assert_eq!(page.assets[0].destination, "style");
        assert_eq!(page.assets[1].destination, "document");
        assert_eq!(page.assets[2].destination, "iframe");
    }

    #[test]
    fn page_can_be_picked() {
        let (page, _) = parse_har(
            &har(vec![
                entry("https://a.test/", 200, "text/html", Some("<p>one")),
                entry("https://a.test/b", 200, "text/html", Some("<p>two")),
            ]),
            Some("https://a.test/b"),
        )
        .unwrap();

        assert_eq!(page.html, "<p>two");
    }

    #[test]
    fn stays_within_the_page() {
        let mut one = entry("https://a.test/", 200, "text/html", Some("<p>one"));
        one["pageref"] = json!("page_1");
        let mut one_style = entry("https://a.test/a.css", 200, "text/css", Some("a{}"));
        one_style["pageref"] = json!("page_1");
        let mut two = entry("https://a.test/b", 200, "text/html", Some("<p>two"));
        two["pageref"] = json!("page_2");
        let har = json!({"log": {
            "pages": [{"id": "page_1"}, {"id": "page_2"}],
            "entries": [one, one_style, two],
        }})
        .to_string();

        let (page, has_other_pages) = parse_har(har.as_bytes(), None).unwrap();
        assert_eq!(page.assets.len(), 2);
        assert!(has_other_pages);

        let (page, _) = parse_har(har.as_bytes(), Some("https://a.test/b")).unwrap();
        assert_eq!(page.assets.len(), 1);
        assert_eq!(page.html, "<p>two");
    }

    #[test]
    fn decodes_base64() {
        let mut image = entry("https://a.test/i.png", 200, "image/png", Some("AAEC"));
        image["response"]["content"]["encoding"] = json!("base64");

        let (page, _) = parse_har(
            &har(vec![
                entry("https://a.test/", 200, "text/html", Some("<p>one")),
                image,
            ]),
            None,
        )
        .unwrap();

        assert_eq!(page.assets[1].data, vec![0, 1, 2]);
    }

    #[test]
    fn text_is_utf8_no_matter_what_it_used_to_be() {
        let (page, _) = parse_har(
            &har(vec![entry(
                "https://a.test/",
                200,
                "text/html; charset=gb2312",
                Some("<p>one"),
            )]),
            None,
        )
        .unwrap();

        assert_eq!(page.assets[0].content_type, "text/html; charset=utf-8");
    }

    #[test]
    fn follows_redirects() {
        let mut redirect = entry("http://a.test/", 301, "", None);
        redirect["response"]["redirectURL"] = json!("https://a.test/");

        let (page, _) = parse_har(
            &har(vec![
                redirect,
                entry("https://a.test/", 200, "text/html", Some("<p>one")),
            ]),
            Some("http://a.test/"),
        )
        .unwrap();

        assert_eq!(page.url, "https://a.test/");
        assert_eq!(page.assets[0].urls[0], "http://a.test/");
    }

    #[test]
    fn headers_say_what_text_has_become() {
        let mut page = entry(
            "https://a.test/",
            200,
            "text/html; charset=gb2312",
            Some("x"),
        );
        page["response"]["headers"] =
            json!([{"name": "Content-Type", "value": "text/html; charset=gb2312"}]);

        let (page, _) = parse_har(&har(vec![page]), None).unwrap();

        assert_eq!(
            page.assets[0].http.response_headers,
            vec![(
                "Content-Type".to_string(),
                "text/html; charset=utf-8".to_string()
            )]
        );
    }

    #[test]
    fn redirects_say_where_they_lead() {
        let mut redirect = entry("http://a.test/", 301, "", None);
        redirect["response"]["redirectURL"] = json!("https://a.test/");

        let (page, _) = parse_har(
            &har(vec![
                redirect,
                entry("https://a.test/", 200, "text/html", Some("<p>one")),
            ]),
            None,
        )
        .unwrap();

        assert_eq!(
            page.assets[0].http.redirects[0].response_headers,
            vec![("Location".to_string(), "https://a.test/".to_string())]
        );
    }

    #[test]
    fn keeps_responses_that_have_nothing_in_them() {
        let (page, _) = parse_har(
            &har(vec![
                entry("https://a.test/", 200, "text/html", Some("<p>one")),
                entry("https://a.test/ping", 204, "", None),
            ]),
            None,
        )
        .unwrap();

        assert_eq!(page.assets.len(), 2);
        assert_eq!(page.assets[1].status, 204);
    }

    #[test]
    fn keeps_responses_to_head_requests() {
        let mut head = entry("https://a.test/check", 200, "text/plain", None);
        head["request"]["method"] = json!("HEAD");

        let (page, _) = parse_har(
            &har(vec![
                entry("https://a.test/", 200, "text/html", Some("<p>one")),
                head,
            ]),
            None,
        )
        .unwrap();

        assert_eq!(page.assets[1].method, "HEAD");
    }
}

#[cfg(test)]
mod failing {
    use monolith::har::parse_har;

    use super::helpers::{entry, har};

    #[test]
    fn exported_without_contents() {
        let error = parse_har(
            &har(vec![entry("https://a.test/", 200, "text/html", None)]),
            None,
        )
        .err()
        .unwrap();

        assert!(error.contains("no contents"));
    }

    #[test]
    fn page_that_is_not_there() {
        let archive = har(vec![entry("https://a.test/", 200, "text/html", Some("x"))]);

        assert!(parse_har(&archive, Some("https://b.test/")).is_err());
    }

    #[test]
    fn no_pages() {
        assert!(parse_har(&har(vec![]), None).is_err());
    }
}
