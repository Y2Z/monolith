#[cfg(test)]
mod helpers {
    use monolith::cache::Exchange;
    use monolith::core::{MonolithOptions, MonolithOutputFormat};
    use monolith::session::Session;

    pub fn session(compress: bool) -> Session {
        let options = MonolithOptions {
            output_format: match compress {
                true => MonolithOutputFormat::WARCGZ,
                false => MonolithOutputFormat::WARC,
            },
            ..Default::default()
        };

        Session::new(None, None, options)
    }

    pub fn response(url: &str, status: u16, headers: &[(&str, &str)], body: &[u8]) -> Exchange {
        Exchange {
            body: body.to_vec(),
            http_version: "HTTP/2".to_string(),
            method: "GET".to_string(),
            request_headers: vec![
                ("Cookie".to_string(), "secret=1".to_string()),
                ("Accept".to_string(), "*/*".to_string()),
            ],
            response_headers: headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            status: Some(status),
            status_text: "OK".to_string(),
            url: url.to_string(),
            ..Default::default()
        }
    }

    pub fn page() -> Exchange {
        response(
            "https://user:password@a.test/#top",
            200,
            &[
                ("Content-Type", "text/html"),
                ("Content-Encoding", "gzip"),
                ("Content-Length", "999"),
                ("Set-Cookie", "session=secret"),
                (":status", "200"),
            ],
            b"<title>t</title>",
        )
    }
}

#[cfg(test)]
mod passing {
    use monolith::cache::Exchange;
    use monolith::warc::{create_warc_document, is_warc, parse_warc};

    use super::helpers::{page, response, session};

    #[test]
    fn can_be_read_back() {
        let mut session = session(false);
        session.record_exchange(page());
        session.record_exchange(response(
            "https://a.test/a.css",
            200,
            &[("Content-Type", "text/css")],
            b"a{}",
        ));

        let warc = create_warc_document(&session);
        let (page, _) = parse_warc(&warc, None).unwrap();

        assert!(is_warc(&warc));
        assert_eq!(page.url, "https://a.test/");
        assert_eq!(page.html, "<title>t</title>");
        assert_eq!(page.assets[1].data, b"a{}");
    }

    #[test]
    fn starts_with_information_about_itself() {
        let warc = String::from_utf8(create_warc_document(&session(false))).unwrap();

        assert!(warc.starts_with("WARC/1.1\r\nWARC-Type: warcinfo\r\n"));
        assert!(warc.contains(&format!("software: monolith/{}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn keeps_responses_whole_and_pairs_them_with_requests() {
        let mut session = session(false);
        session.record_exchange(page());
        let warc = String::from_utf8(create_warc_document(&session)).unwrap();

        assert!(warc.contains("WARC-Target-URI: https://a.test/\r\n"));
        assert!(warc.contains("\r\nHTTP/1.1 200 OK\r\nContent-Type: text/html\r\n"));
        assert!(warc.contains("Content-Length: 16\r\n\r\n<title>t</title>"));
        assert!(warc.contains("GET / HTTP/1.1\r\nHost: a.test\r\nAccept: */*\r\n"));
        assert!(warc.contains("WARC-Concurrent-To: <urn:uuid:"));
        assert!(warc.contains("WARC-Payload-Digest: sha256:"));
    }

    #[test]
    fn gives_away_nothing_about_who_saved_it() {
        let mut session = session(false);
        session.record_exchange(page());
        let warc = String::from_utf8(create_warc_document(&session)).unwrap();

        assert!(!warc.contains("secret"));
        assert!(!warc.contains("password"));
        assert!(!warc.contains("user"));
    }

    #[test]
    fn leaves_out_what_no_longer_applies() {
        let mut session = session(false);
        session.record_exchange(page());
        let warc = String::from_utf8(create_warc_document(&session)).unwrap();

        assert!(!warc.contains("Content-Encoding"));
        assert!(!warc.contains("Content-Length: 999"));
        assert!(!warc.contains(":status"));
    }

    #[test]
    fn what_did_not_come_over_http_becomes_a_resource() {
        let mut session = session(false);
        session.record_exchange(Exchange {
            body: b"<p>local".to_vec(),
            content_type: "text/html".to_string(),
            url: "file:///page.html".to_string(),
            ..Default::default()
        });
        let warc = String::from_utf8(create_warc_document(&session)).unwrap();

        assert!(warc.contains("WARC-Type: resource\r\n"));
        assert!(warc.contains("Content-Type: text/html\r\nContent-Length: 8\r\n\r\n<p>local"));
        assert!(!warc.contains("WARC-Type: request"));
    }

    #[test]
    fn redirects_are_records_of_their_own() {
        let mut session = session(false);
        session.record_exchange(response(
            "http://a.test/",
            301,
            &[("Location", "https://a.test/")],
            b"",
        ));
        session.record_exchange(page());
        let warc = create_warc_document(&session);

        let (page, _) = parse_warc(&warc, Some("http://a.test/")).unwrap();

        assert_eq!(page.url, "https://a.test/");
    }

    #[test]
    fn compressed_record_by_record() {
        let mut session = session(true);
        session.record_exchange(page());
        let warc = create_warc_document(&session);

        let members = warc
            .windows(3)
            .filter(|bytes| bytes == &[0x1f, 0x8b, 0x08])
            .count();
        assert_eq!(members, 3);
        assert_eq!(parse_warc(&warc, None).unwrap().0.html, "<title>t</title>");
    }

    #[test]
    fn digests_are_base32() {
        let mut session = session(false);
        session.record_exchange(response("https://a.test/", 200, &[], b""));
        let warc = String::from_utf8(create_warc_document(&session)).unwrap();

        // SHA-256 of nothing
        assert!(warc.contains(
            "WARC-Payload-Digest: sha256:4OYMIQUY7QOBJGX36TEJS35ZEQT24QPEMSNZGTFESWMRW6CSXBKQ====\r\n"
        ));
    }

    #[test]
    fn requests_do_not_claim_to_have_bodies() {
        let mut session = session(false);
        let mut exchange = page();
        exchange.method = "POST".to_string();
        exchange
            .request_headers
            .push(("Content-Length".to_string(), "5".to_string()));
        exchange
            .request_headers
            .push(("Transfer-Encoding".to_string(), "chunked".to_string()));
        session.record_exchange(exchange);
        let warc = String::from_utf8(create_warc_document(&session)).unwrap();
        let request = &warc[warc.find("POST / HTTP/1.1").unwrap()..];
        let request = &request[..request.find("\r\n\r\n").unwrap()];

        assert!(!request.contains("Content-Length"));
        assert!(!request.contains("Transfer-Encoding"));
    }

    #[test]
    fn status_lines_without_reason_phrases_stay_valid() {
        let mut session = session(false);
        let mut exchange = page();
        exchange.status_text = String::new();
        session.record_exchange(exchange);
        let warc = String::from_utf8(create_warc_document(&session)).unwrap();

        assert!(warc.contains("\r\nHTTP/1.1 200 \r\n"));
    }
}

#[cfg(test)]
mod failing {
    use monolith::session::Session;

    #[test]
    fn records_nothing_for_other_formats() {
        let mut session = Session::new(None, None, Default::default());
        session.record_exchange(super::helpers::page());

        assert!(session.cache().is_none());
    }
}

#[cfg(test)]
mod passing_imported {
    use monolith::cache::Exchange;
    use monolith::warc::{create_warc_document, parse_warc};

    use super::helpers::{page, session};

    #[test]
    fn what_has_been_imported_can_not_start_lines_of_its_own() {
        let mut session = session(false);
        let mut exchange = page();
        exchange.status_text = "OK\r\nWARC/1.1\r\nWARC-Type: response".to_string();
        exchange.method = "GET /\r\nInjected: 1".to_string();
        exchange
            .response_headers
            .push(("X-Bad\r\nInjected".to_string(), "2".to_string()));
        exchange
            .response_headers
            .push(("X-Value".to_string(), "a\r\nInjected: 3\u{0}".to_string()));
        session.record_exchange(exchange);
        session.record_exchange(Exchange {
            body: b"x".to_vec(),
            content_type: "text/plain\r\nInjected: 4".to_string(),
            url: "file:///a.txt".to_string(),
            ..Default::default()
        });
        let warc = String::from_utf8(create_warc_document(&session)).unwrap();

        assert!(!warc.contains("\nInjected"));
        assert_eq!(warc.matches("\nWARC-Type: response").count(), 1);
        assert_eq!(warc.matches("\nWARC/1.1").count(), 3);
        assert!(warc.contains("\r\nHTTP/1.1 200 OK  WARC/1.1  WARC-Type: response\r\n"));
        assert!(warc.contains("\r\nGET / HTTP/1.1\r\n"));
        assert!(warc.contains("X-Value: a  Injected: 3\r\n"));
        assert_eq!(
            parse_warc(warc.as_bytes(), None).unwrap().0.html,
            "<title>t</title>"
        );
    }
}
