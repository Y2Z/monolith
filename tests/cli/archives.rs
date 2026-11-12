#[cfg(test)]
mod helpers {
    use assert_cmd::Command;
    use serde_json::json;

    pub fn monolith(archive: &[u8], args: &[&str]) -> (String, String, bool) {
        let output = Command::new(assert_cmd::cargo_bin!("monolith"))
            .args(args)
            .arg("-")
            .write_stdin(archive)
            .output()
            .unwrap();

        (
            crate::cli::without_metadata(&output.stdout),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.success(),
        )
    }

    pub fn warc() -> Vec<u8> {
        let mut warc = vec![];
        for (url, content_type, body) in [
            (
                "https://a.test/",
                "text/html",
                "<title>One</title><link rel=stylesheet href=a.css>\
                 <img src=missing.png><script src=a.js></script>",
            ),
            ("https://a.test/a.css", "text/css", "a{color:red}"),
            (
                "https://a.test/a.js",
                "text/javascript",
                "document.title='Two'",
            ),
            ("https://a.test/b", "text/html", "<title>Other</title>"),
        ] {
            let block = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\n\r\n{}",
                content_type, body
            );
            warc.extend_from_slice(
                format!(
                    "WARC/1.1\r\nWARC-Type: response\r\nWARC-Target-URI: {}\r\n\
                     Content-Length: {}\r\n\r\n{}\r\n\r\n",
                    url,
                    block.len(),
                    block
                )
                .as_bytes(),
            );
        }
        warc
    }

    pub fn har() -> Vec<u8> {
        json!({"log": {"entries": [
            {
                "request": {"method": "GET", "url": "https://a.test/"},
                "response": {"status": 200, "headers": [], "content": {
                    "mimeType": "text/html",
                    "text": "<title>One</title><script src=a.js></script>",
                }},
            },
            {
                "request": {"method": "GET", "url": "https://a.test/a.js"},
                "response": {"status": 200, "headers": [], "content": {
                    "mimeType": "text/javascript",
                    "text": "document.title='Two'",
                }},
            },
        ]}})
        .to_string()
        .into_bytes()
    }
}

#[cfg(test)]
mod passing {
    use super::helpers::{har, monolith, warc};

    #[test]
    fn warc_gets_replayed() {
        let (output, _, success) = monolith(&warc(), &["-q"]);

        assert!(success);
        assert!(output.contains("data-monolith-url=\"https://a.test/a.css\""));
        assert!(output.contains("PAGE_URL=\"https://a.test/\""));
    }

    #[test]
    fn har_gets_replayed() {
        let (output, _, success) = monolith(&har(), &["-q"]);

        assert!(success);
        assert!(output.contains("PAGE_URL=\"https://a.test/\""));
    }

    #[test]
    fn other_pages_get_mentioned() {
        let (_, errors, _) = monolith(&warc(), &[]);

        assert!(errors.contains("-b picks"));
    }

    #[test]
    fn page_can_be_picked_with_base_url() {
        let (output, errors, success) = monolith(&warc(), &["-b", "https://a.test/b"]);

        assert!(success);
        assert!(output.contains("<title>Other</title>"));
        assert!(!errors.contains("-b picks"));
    }

    #[test]
    fn stays_offline_without_scripts() {
        let (output, errors, success) = monolith(&warc(), &["-j"]);

        assert!(success);
        assert!(output.contains("color:red"));
        assert!(!output.contains("monolith-url"));
        assert!(!output.contains("document.title"));
        assert!(errors.contains("https://a.test/missing.png (not found in archive)"));
    }

    #[test]
    fn can_be_turned_into_mhtml() {
        let (output, _, success) = monolith(&warc(), &["-q", "-F", "MHTML"]);

        assert!(success);
        assert!(output.contains("Content-Location: https://a.test/a.js"));
    }

    #[test]
    fn can_be_simplified() {
        let (output, _, success) = monolith(&warc(), &["-q", "-S"]);

        assert!(success);
        assert!(!output.contains("monolith-url"));
    }

    #[test]
    fn integrity_of_stylesheets_is_about_what_they_used_to_be() {
        use base64::Engine;
        use sha2::{Digest, Sha384};

        let stylesheet: &[u8] = b"@charset \"windows-1252\"; p { color: red }";
        let integrity = format!(
            "sha384-{}",
            base64::prelude::BASE64_STANDARD.encode(Sha384::digest(stylesheet))
        );
        let mut warc = vec![];
        for (url, content_type, body) in [
            (
                "https://a.test/",
                "text/html",
                format!(
                    "<link rel=stylesheet href=a.css integrity=\"{}\"><script>1</script>",
                    integrity
                )
                .into_bytes(),
            ),
            ("https://a.test/a.css", "text/css", stylesheet.to_vec()),
        ] {
            let mut block =
                format!("HTTP/1.1 200 OK\r\nContent-Type: {}\r\n\r\n", content_type).into_bytes();
            block.extend(body);
            warc.extend(
                format!(
                    "WARC/1.1\r\nWARC-Type: response\r\nWARC-Target-URI: {}\r\n\
                     Content-Length: {}\r\n\r\n",
                    url,
                    block.len()
                )
                .into_bytes(),
            );
            warc.extend(block);
            warc.extend(b"\r\n\r\n");
        }

        let (output, _, success) = monolith(&warc, &["-q"]);

        assert!(success);
        assert!(output.contains("color: red"), "{}", output);
    }
}

#[cfg(test)]
mod failing {
    use super::helpers::{monolith, warc};

    #[test]
    fn page_that_is_not_in_the_archive() {
        let (_, errors, success) = monolith(&warc(), &["-b", "https://b.test/"]);

        assert!(!success);
        assert!(errors.contains("archive has no page at https://b.test/"));
    }
}

#[cfg(test)]
mod warc_output {
    use std::env;

    use assert_cmd::Command;
    use monolith::warc::parse_warc;

    use super::helpers::{har, monolith, warc};

    fn bytes(args: &[&str], input: &[u8]) -> (Vec<u8>, String, bool) {
        let output = Command::new(assert_cmd::cargo_bin!("monolith"))
            .args(args)
            .write_stdin(input)
            .output()
            .unwrap();

        (
            output.stdout,
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.success(),
        )
    }

    #[test]
    fn local_files_become_resources() {
        let path = env::current_dir()
            .unwrap()
            .join("tests/_data_/basic/local-file.html");
        let (output, _, success) = bytes(&["-q", "-F", "WARC", path.to_str().unwrap()], b"");
        let output = String::from_utf8_lossy(&output);

        assert!(success);
        assert!(output.contains("WARC-Type: resource"));
        assert!(output.contains("local-file.html"));
        assert!(output.contains("local-style.css"));
    }

    #[test]
    fn documents_from_stdin_need_a_url() {
        let (_, errors, success) = bytes(&["-F", "WARC", "-"], b"<p>hi");

        assert!(!success);
        assert!(errors.contains("-b can provide one"));

        let (output, _, success) = bytes(
            &["-q", "-F", "WARC", "-b", "https://a.test/", "-"],
            b"<p>hi",
        );
        assert!(success);
        assert_eq!(parse_warc(&output, None).unwrap().0.html, "<p>hi");
    }

    #[test]
    fn archives_turn_into_warc() {
        for archive in [warc(), har()] {
            let (output, _, success) = bytes(&["-q", "-F", "WARC", "-"], &archive);
            let (page, _) = parse_warc(&output, None).unwrap();

            assert!(success);
            assert_eq!(page.url, "https://a.test/");
            assert!(
                page.assets
                    .iter()
                    .any(|asset| asset.urls[0] == "https://a.test/a.js")
            );
        }
    }

    #[test]
    fn webarchives_turn_into_warc() {
        let (output, _, success) = monolith(
            include_bytes!("../_data_/webarchive/page.webarchive"),
            &["-q", "-F", "WARC"],
        );

        assert!(success);
        assert!(output.contains("WARC-Target-URI: https://example.com/page.html"));
        assert!(output.contains("WARC-Target-URI: https://example.com/style.css"));
    }

    #[test]
    fn documents_given_as_data_urls_can_get_their_url_from_base_url() {
        let (output, _, success) = bytes(
            &[
                "-q",
                "-F",
                "WARC",
                "-b",
                "https://a.test/",
                "data:text/html,<p>hi",
            ],
            b"",
        );

        assert!(success);
        assert_eq!(parse_warc(&output, None).unwrap().0.url, "https://a.test/");
    }

    #[test]
    fn local_files_that_are_not_documents_get_archived_too() {
        let path = env::current_dir()
            .unwrap()
            .join("tests/_data_/basic/local-style.css");
        let (output, _, success) = bytes(&["-q", "-F", "WARC", path.to_str().unwrap()], b"");
        let output = String::from_utf8_lossy(&output);

        assert!(success);
        assert!(output.contains("WARC-Type: resource"));
        assert!(output.contains("local-style.css"));
    }

    #[test]
    fn archives_can_not_reach_into_local_files() {
        let directory = env::current_dir().unwrap().join("tests/_data_/basic");
        let page = format!(
            "<link rel=stylesheet href=\"{}\">",
            monolith::url::Url::from_file_path(directory.join("local-style.css")).unwrap()
        );
        let page_url = monolith::url::Url::from_file_path(directory.join("page.html")).unwrap();
        let warc = format!(
            "WARC/1.1\r\nWARC-Type: resource\r\nWARC-Target-URI: {}\r\n\
             Content-Type: text/html\r\nContent-Length: {}\r\n\r\n{}\r\n\r\n",
            page_url,
            page.len(),
            page
        );
        let stylesheet = std::fs::read_to_string(directory.join("local-style.css")).unwrap();

        for args in [&["-j"][..], &[][..]] {
            let (output, errors, success) = monolith(warc.as_bytes(), args);

            assert!(success);
            assert!(!output.contains(stylesheet.trim()));
            assert!(errors.contains("local-style.css (not found in archive)"));
        }
    }

    #[test]
    fn data_urls_that_are_not_documents_get_archived_too() {
        let (output, _, success) = bytes(
            &[
                "-q",
                "-F",
                "WARC",
                "-b",
                "https://a.test/style.css",
                "data:text/css,a{}",
            ],
            b"",
        );
        let output = String::from_utf8_lossy(&output);

        assert!(success);
        assert!(output.contains("WARC-Target-URI: https://a.test/style.css"));
        assert!(output.contains("a{}"));

        let (_, errors, success) = bytes(&["-F", "WARC", "data:text/css,a{}"], b"");
        assert!(!success);
        assert!(errors.contains("-b can provide one"));
    }

    #[test]
    fn compression_is_a_format_of_its_own() {
        let directory = tempfile::tempdir().unwrap();
        let named_like_compressed = directory.path().join("page.warc.gz");
        let (_, _, success) = bytes(
            &[
                "-q",
                "-F",
                "WARC",
                "-b",
                "https://a.test/",
                "-o",
                named_like_compressed.to_str().unwrap(),
                "-",
            ],
            b"<p>hi",
        );
        assert!(success);
        // File names don't decide anything
        assert!(
            std::fs::read(&named_like_compressed)
                .unwrap()
                .starts_with(b"WARC/1.1")
        );

        let (output, _, success) = bytes(
            &["-q", "-F", "WARCGZ", "-b", "https://a.test/", "-"],
            b"<p>hi",
        );
        assert!(success);
        assert!(output.starts_with(&[0x1f, 0x8b]));
        assert_eq!(parse_warc(&output, None).unwrap().0.html, "<p>hi");
    }
}
