#[cfg(test)]
mod helpers {
    pub fn record(kind: &str, url: &str, extra: &str, block: &[u8]) -> Vec<u8> {
        let mut record = format!(
            "WARC/1.1\r\nWARC-Type: {}\r\nWARC-Target-URI: {}\r\n{}Content-Length: {}\r\n\r\n",
            kind,
            url,
            extra,
            block.len()
        )
        .into_bytes();
        record.extend_from_slice(block);
        record.extend_from_slice(b"\r\n\r\n");
        record
    }

    pub fn http(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
        let mut response = format!("HTTP/1.1 {}\r\n{}\r\n", status, headers).into_bytes();
        response.extend_from_slice(body);
        response
    }

    pub fn html(body: &str) -> Vec<u8> {
        http("200 OK", "Content-Type: text/html\r\n", body.as_bytes())
    }
}

#[cfg(test)]
mod passing {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use monolith::warc::parse_warc;

    use super::helpers::{html, http, record};

    #[test]
    fn first_document_is_the_page() {
        let mut warc = record(
            "response",
            "https://a.test/robots.txt",
            "",
            &http("200 OK", "Content-Type: text/plain\r\n", b"nothing"),
        );
        warc.extend(record("response", "https://a.test/", "", &html("<p>one")));
        warc.extend(record("response", "https://a.test/b", "", &html("<p>two")));

        let (page, has_other_pages) = parse_warc(&warc, None).unwrap();

        assert_eq!(page.url, "https://a.test/");
        assert_eq!(page.html, "<p>one");
        assert!(has_other_pages);
    }

    #[test]
    fn page_can_be_picked() {
        let mut warc = record("response", "https://a.test/", "", &html("<p>one"));
        warc.extend(record("response", "https://a.test/b", "", &html("<p>two")));

        let (page, _) = parse_warc(&warc, Some("https://a.test/b")).unwrap();

        assert_eq!(page.html, "<p>two");
    }

    #[test]
    fn urls_in_angle_brackets() {
        let warc = record("response", "<https://a.test/>", "", &html("<p>one"));

        assert_eq!(parse_warc(&warc, None).unwrap().0.url, "https://a.test/");
    }

    #[test]
    fn follows_redirects() {
        let mut warc = record(
            "response",
            "http://a.test/",
            "",
            &http("301 Moved", "Location: https://a.test/home\r\n", b""),
        );
        warc.extend(record(
            "response",
            "https://a.test/home",
            "",
            &html("<p>home"),
        ));

        let (page, _) = parse_warc(&warc, Some("http://a.test/")).unwrap();

        assert_eq!(page.url, "https://a.test/home");
        assert_eq!(
            page.assets[0].urls,
            vec!["http://a.test/", "https://a.test/home"]
        );
    }

    #[test]
    fn undoes_transfer_encodings() {
        let mut gzip = GzEncoder::new(vec![], Compression::default());
        gzip.write_all(b"<p>one").unwrap();
        let gzip = gzip.finish().unwrap();
        let mut chunked = format!("{:x}\r\n", gzip.len()).into_bytes();
        chunked.extend_from_slice(&gzip);
        chunked.extend_from_slice(b"\r\n0\r\n\r\n");
        let warc = record(
            "response",
            "https://a.test/",
            "",
            &http(
                "200 OK",
                "Content-Type: text/html\r\nContent-Encoding: gzip\r\nTransfer-Encoding: chunked\r\n",
                &chunked,
            ),
        );

        assert_eq!(parse_warc(&warc, None).unwrap().0.html, "<p>one");
    }

    #[test]
    fn archives_compressed_record_by_record() {
        let mut warc = vec![];
        for block in [
            record("warcinfo", "", "", b"software: test"),
            record("response", "https://a.test/", "", &html("<p>one")),
        ] {
            let mut encoder = GzEncoder::new(vec![], Compression::default());
            encoder.write_all(&block).unwrap();
            warc.extend(encoder.finish().unwrap());
        }

        assert_eq!(parse_warc(&warc, None).unwrap().0.html, "<p>one");
    }

    #[test]
    fn revisits_get_what_they_refer_to() {
        let digest = "WARC-Payload-Digest: sha1:ABC\r\n";
        let mut warc = record("response", "https://a.test/", "", &html("<p>one"));
        warc.extend(record(
            "response",
            "https://a.test/a.css",
            digest,
            &http("200 OK", "Content-Type: text/css\r\n", b"a{}"),
        ));
        warc.extend(record(
            "revisit",
            "https://a.test/b.css",
            digest,
            &http("200 OK", "Content-Type: text/css\r\n", b""),
        ));

        let (page, _) = parse_warc(&warc, None).unwrap();

        assert_eq!(page.assets[2].urls, vec!["https://a.test/b.css"]);
        assert_eq!(page.assets[2].data, b"a{}");
    }

    #[test]
    fn methods_come_from_requests() {
        let id = "<urn:uuid:1>";
        let mut warc = record("response", "https://a.test/", "", &html("<p>one"));
        warc.extend(record(
            "response",
            "https://a.test/api",
            &format!("WARC-Record-ID: {}\r\n", id),
            &http("200 OK", "Content-Type: application/json\r\n", b"{}"),
        ));
        warc.extend(record(
            "request",
            "https://a.test/api",
            &format!("WARC-Concurrent-To: {}\r\n", id),
            b"POST /api HTTP/1.1\r\n\r\n",
        ));

        let (page, _) = parse_warc(&warc, None).unwrap();

        assert_eq!(page.assets[1].method, "POST");
        assert!(!page.assets[1].is_reusable);
    }

    #[test]
    fn stays_with_what_was_stored_before_getting_cut_short() {
        let mut warc = record("response", "https://a.test/", "", &html("<p>one"));
        let next = record("response", "https://a.test/b", "", &html("<p>two"));
        warc.extend_from_slice(&next[..next.len() - 10]);

        let (page, has_other_pages) = parse_warc(&warc, None).unwrap();

        assert_eq!(page.html, "<p>one");
        assert!(!has_other_pages);
    }

    #[test]
    fn keeps_every_response_to_the_same_request() {
        let mut warc = record("response", "https://a.test/", "", &html("<p>one"));
        for body in [b"1", b"2"] {
            warc.extend(record(
                "response",
                "https://a.test/poll",
                "",
                &http("200 OK", "Content-Type: application/json\r\n", body),
            ));
        }

        let (page, _) = parse_warc(&warc, None).unwrap();

        assert_eq!(page.assets.len(), 3);
        assert_eq!(page.assets[1].data, b"1");
        assert_eq!(page.assets[2].data, b"2");
    }

    #[test]
    fn documents_get_decoded_the_way_they_say_they_are_encoded() {
        let warc = record(
            "response",
            "https://a.test/",
            "",
            &http(
                "200 OK",
                "Content-Type: text/html; charset=windows-1252\r\n",
                b"<p>caf\xe9",
            ),
        );

        assert_eq!(parse_warc(&warc, None).unwrap().0.html, "<p>caf\u{e9}");
    }

    #[test]
    fn undoes_encodings_applied_one_after_another() {
        use flate2::write::ZlibEncoder;

        let mut deflate = ZlibEncoder::new(vec![], Compression::default());
        deflate.write_all(b"<p>one").unwrap();
        let mut gzip = GzEncoder::new(vec![], Compression::default());
        gzip.write_all(&deflate.finish().unwrap()).unwrap();
        let warc = record(
            "response",
            "https://a.test/",
            "",
            &http(
                "200 OK",
                "Content-Type: text/html\r\nContent-Encoding: deflate, gzip\r\n",
                &gzip.finish().unwrap(),
            ),
        );

        assert_eq!(parse_warc(&warc, None).unwrap().0.html, "<p>one");
    }

    #[test]
    fn redirects_get_paired_with_what_they_led_to() {
        let mut warc = record("response", "https://a.test/", "", &html("<p>one"));
        for (version, body) in [("v1", b"1"), ("v2", b"2")] {
            warc.extend(record(
                "response",
                "https://a.test/api",
                "",
                &http("302 Found", &format!("Location: /{}\r\n", version), b""),
            ));
            warc.extend(record(
                "response",
                &format!("https://a.test/{}", version),
                "",
                &http("200 OK", "Content-Type: application/json\r\n", body),
            ));
        }

        let (page, _) = parse_warc(&warc, None).unwrap();

        assert_eq!(
            page.assets[1].urls,
            vec!["https://a.test/api", "https://a.test/v1"]
        );
        assert_eq!(
            page.assets[2].urls,
            vec!["https://a.test/api", "https://a.test/v2"]
        );
        assert_eq!(page.assets[1].http.redirects.len(), 1);
        assert_eq!(page.assets[2].http.redirects.len(), 1);
    }

    #[test]
    fn resources_stay_resources() {
        let warc = record(
            "resource",
            "file:///page.html",
            "Content-Type: text/html\r\n",
            b"<p>x",
        );
        let (page, _) = parse_warc(&warc, None).unwrap();
        let exchanges = monolith::replay::exchanges_of(&page.assets[0]);

        assert!(page.assets[0].http.is_resource);
        assert_eq!(exchanges[0].status, None);
    }

    #[test]
    fn stylesheets_follow_rules_of_their_own() {
        let mut warc = record(
            "response",
            "https://a.test/",
            "",
            &html("<link rel=stylesheet href=a.css><script></script>"),
        );
        warc.extend(record(
            "response",
            "https://a.test/a.css",
            "",
            &http(
                "200 OK",
                "Content-Type: text/css\r\n",
                b"@charset \"windows-1252\"; a::after { content: '\xe9' }",
            ),
        ));
        let (mut page, _) = parse_warc(&warc, None).unwrap();

        monolith::replay::rewrite_stylesheets(&mut page.assets, None);

        assert!(
            String::from_utf8(page.assets[1].data.clone())
                .unwrap()
                .contains("'\u{e9}'")
        );
    }

    #[test]
    fn unlabeled_stylesheets_go_by_the_document() {
        let mut warc = record("response", "https://a.test/", "", &html("<p>"));
        warc.extend(record(
            "response",
            "https://a.test/a.css",
            "",
            &http(
                "200 OK",
                "Content-Type: text/css\r\n",
                // "日本" in Shift_JIS
                b"a::after { content: '\x93\xfa\x96\x7b' }",
            ),
        ));
        let (mut page, _) = parse_warc(&warc, None).unwrap();

        monolith::replay::rewrite_stylesheets(&mut page.assets, Some(encoding_rs::SHIFT_JIS));

        assert!(
            String::from_utf8(page.assets[1].data.clone())
                .unwrap()
                .contains("'\u{65e5}\u{672c}'")
        );
    }

    #[test]
    fn redirects_keep_the_method_they_were_asked_with() {
        let id = "<urn:uuid:1>";
        let mut warc = record("response", "https://a.test/", "", &html("<p>one"));
        warc.extend(record(
            "response",
            "https://a.test/form",
            &format!("WARC-Record-ID: {}\r\n", id),
            &http("303 See Other", "Location: /done\r\n", b""),
        ));
        warc.extend(record(
            "request",
            "https://a.test/form",
            &format!("WARC-Concurrent-To: {}\r\n", id),
            b"POST /form HTTP/1.1\r\n\r\n",
        ));
        warc.extend(record(
            "response",
            "https://a.test/done",
            "",
            &html("<p>done"),
        ));

        let (page, _) = parse_warc(&warc, None).unwrap();
        let exchanges = monolith::replay::exchanges_of(&page.assets[1]);

        assert_eq!(exchanges[0].method, "POST");
        assert_eq!(exchanges[1].method, "GET");
    }

    #[test]
    fn undoes_every_transfer_coding() {
        let mut gzip = GzEncoder::new(vec![], Compression::default());
        gzip.write_all(b"<p>one").unwrap();
        let gzip = gzip.finish().unwrap();
        let mut chunked = format!("{:x}\r\n", gzip.len()).into_bytes();
        chunked.extend_from_slice(&gzip);
        chunked.extend_from_slice(b"\r\n0\r\n\r\n");
        let warc = record(
            "response",
            "https://a.test/",
            "",
            &http(
                "200 OK",
                "Content-Type: text/html\r\nTransfer-Encoding: gzip, chunked\r\n",
                &chunked,
            ),
        );

        assert_eq!(parse_warc(&warc, None).unwrap().0.html, "<p>one");
    }
}

#[cfg(test)]
mod failing {
    use monolith::warc::parse_warc;

    use super::helpers::{html, http, record};

    #[test]
    fn no_pages() {
        let warc = record(
            "response",
            "https://a.test/a.css",
            "",
            &http("200 OK", "Content-Type: text/css\r\n", b"a{}"),
        );

        assert!(parse_warc(&warc, None).is_err());
    }

    #[test]
    fn page_that_is_not_there() {
        let warc = record("response", "https://a.test/", "", &html("<p>one"));

        assert!(parse_warc(&warc, Some("https://b.test/")).is_err());
    }

    #[test]
    fn truncated_records_get_left_out() {
        let warc = record(
            "response",
            "https://a.test/",
            "WARC-Truncated: length\r\n",
            &html("<p>one"),
        );

        assert!(parse_warc(&warc, None).is_err());
    }
}
