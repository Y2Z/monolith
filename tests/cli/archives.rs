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
            String::from_utf8_lossy(&output.stdout).into_owned(),
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
