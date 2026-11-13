#[cfg(test)]
mod helpers {
    use assert_cmd::Command;
    use serde_json::json;

    pub fn monolith(archive: &[u8], args: &[&str]) -> (String, String, bool) {
        let writes_to_file = args.iter().any(|arg| *arg == "-o" || *arg == "--output");
        let output = Command::new(assert_cmd::cargo_bin!("monolith"))
            .args(if writes_to_file {
                vec![]
            } else {
                vec!["-o", "-"]
            })
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
        warc_of(&[
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
        ])
    }

    /// WARC archive of given responses: URLs, media types, and bodies.
    pub fn warc_of(responses: &[(&str, &str, &str)]) -> Vec<u8> {
        let mut warc = vec![];
        for (url, content_type, body) in responses {
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

    /// Monolith taking everything from given archives (-a), saving given targets to STDOUT.
    pub fn from_archives(
        archives: &[Vec<u8>],
        args: &[&str],
        targets: &[&str],
    ) -> (String, String, bool) {
        let directory = tempfile::tempdir().unwrap();
        let mut command = Command::new(assert_cmd::cargo_bin!("monolith"));
        command.current_dir(directory.path()).args(["-o", "-"]);
        for (index, archive) in archives.iter().enumerate() {
            let path = directory.path().join(format!("archive-{}", index));
            std::fs::write(&path, archive).unwrap();
            command.arg("-a").arg(path);
        }
        let output = command.args(args).args(targets).output().unwrap();

        (
            crate::cli::without_metadata(&output.stdout),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.success(),
        )
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

        assert!(errors.contains("-a takes archives, for their URLs to be targets"));
    }

    #[test]
    fn base_url_does_not_pick_pages() {
        let (output, _, success) = monolith(&warc(), &["-q", "-j", "-b", "https://a.test/b"]);

        assert!(success);
        assert!(output.contains("<title>One</title>"));
        assert!(output.contains("<base href=\"https://a.test/b\">"));
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
    use super::helpers::{from_archives, warc};

    #[test]
    fn page_that_is_not_in_the_archive() {
        let (_, errors, success) = from_archives(&[warc()], &[], &["https://b.test/"]);

        assert!(!success);
        assert!(errors.contains("none of the archives have https://b.test/"));
    }

    #[test]
    fn what_is_not_an_archive() {
        let (_, errors, success) = from_archives(
            &[b"<title>Page</title>".to_vec()],
            &[],
            &["https://a.test/"],
        );

        assert!(!success);
        assert_eq!(
            errors,
            "Error: archive #1 is neither WARC, HAR, MHTML, nor WebArchive\n"
        );
    }

    #[test]
    fn archives_need_targets() {
        let (_, errors, success) = from_archives(&[warc()], &[], &[]);

        assert!(!success);
        assert!(errors.contains("required arguments were not provided"));
    }

    #[test]
    fn browsers_and_stdin_would_not_use_archives() {
        for (args, error) in [
            (
                ["-B", "https://a.test/"],
                match cfg!(feature = "browser") {
                    true => "browser would go online",
                    false => "no browser support",
                },
            ),
            (["-j", "-"], "STDIN can't be one"),
        ] {
            let (_, errors, success) = from_archives(&[warc()], &args[..1], &args[1..]);

            assert!(!success);
            assert!(errors.contains(error), "{}", errors);
        }
    }
}

#[cfg(test)]
mod taking_everything_from_archives {
    use std::io::Write;

    use super::helpers::{from_archives, har, warc, warc_of};

    #[test]
    fn pages_get_picked_by_their_urls() {
        for archive in [warc(), har()] {
            let (output, _, success) =
                from_archives(&[archive], &["-q", "-j"], &["https://a.test/"]);

            assert!(success);
            assert!(output.contains("<title>One</title>"));
        }

        let (output, _, success) = from_archives(&[warc()], &["-q", "-j"], &["https://a.test/b"]);
        assert!(success);
        assert!(output.contains("<title>Other</title>"));
    }

    #[test]
    fn pages_get_replayed() {
        let (output, _, success) = from_archives(&[warc()], &["-q"], &["https://a.test/"]);

        assert!(success);
        assert!(output.contains("PAGE_URL=\"https://a.test/\""));
    }

    #[test]
    fn assets_can_be_in_other_warc_archives() {
        let page = warc_of(&[(
            "https://c.test/",
            "text/html",
            "<title>C</title><link rel=stylesheet href=c.css>",
        )]);
        let mut style = vec![];
        flate2::write::GzEncoder::new(&mut style, flate2::Compression::default())
            .write_all(&warc_of(&[(
                "https://c.test/c.css",
                "text/css",
                "c{color:blue}",
            )]))
            .unwrap();

        let (output, _, success) =
            from_archives(&[page, style], &["-q", "-j"], &["https://c.test/"]);

        assert!(success);
        assert!(output.contains("<title>C</title>"));
        assert!(output.contains("c{color:blue}"));
    }

    #[test]
    fn nothing_comes_from_anywhere_else() {
        let page = warc_of(&[(
            "https://c.test/",
            "text/html",
            "<link rel=stylesheet href=https://127.0.0.1:9/x.css><img src=gone.png>",
        )]);

        let (output, errors, success) = from_archives(&[page], &["-j"], &["https://c.test/"]);

        assert!(success);
        assert!(
            errors.contains("https://127.0.0.1:9/x.css (not found in archive)"),
            "{}",
            errors
        );
        assert!(
            errors.contains("https://c.test/gone.png (not found in archive)"),
            "{}",
            errors
        );
        // What couldn't be found stays where it was, the way it does online
        assert!(output.contains("href=\"https://127.0.0.1:9/x.css\""));
    }

    #[test]
    fn several_pages_get_linked_to_each_other() {
        let site = warc_of(&[
            (
                "https://s.test/",
                "text/html",
                "<title>Home</title><a href=/about>About</a>",
            ),
            (
                "https://s.test/about",
                "text/html",
                "<title>About</title><a href=/>Home</a>",
            ),
        ]);
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("site.warc"), site).unwrap();

        let output = assert_cmd::Command::new(assert_cmd::cargo_bin!("monolith"))
            .current_dir(directory.path())
            .args(["-q", "-j", "-a", "site.warc", "-o", "saved/"])
            .args(["https://s.test/", "https://s.test/about"])
            .output()
            .unwrap();

        assert!(output.status.success());
        let read = |name: &str| std::fs::read_to_string(directory.path().join("saved").join(name));
        assert!(
            read("Home.html")
                .unwrap()
                .contains("<a href=\"About.html\">About</a>")
        );
        assert!(
            read("About.html")
                .unwrap()
                .contains("<a href=\"Home.html\">Home</a>")
        );
    }

    #[test]
    fn mhtml_archives_have_pages_at_their_urls() {
        let mhtml = "From: <Saved by Monolith>\r\nSnapshot-Content-Location: https://m.test/\r\n\
                     MIME-Version: 1.0\r\nContent-Type: multipart/related; type=\"text/html\"; \
                     boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/html\r\n\
                     Content-Location: https://m.test/\r\n\r\n<title>M</title>\r\n--b--\r\n";

        let (output, _, success) =
            from_archives(&[mhtml.as_bytes().to_vec()], &["-q"], &["https://m.test/"]);
        assert!(success);
        assert!(output.contains("<title>M</title>"));

        let (_, errors, success) =
            from_archives(&[mhtml.as_bytes().to_vec()], &[], &["https://m.test/other"]);
        assert!(!success);
        assert!(errors.contains("none of the archives have https://m.test/other"));
    }
}

#[cfg(test)]
mod warc_output {
    use std::env;

    use assert_cmd::Command;
    use monolith::warc::parse_warc;

    use super::helpers::{har, monolith, warc};

    fn bytes(args: &[&str], input: &[u8]) -> (Vec<u8>, String, bool) {
        let writes_to_file = args.iter().any(|arg| *arg == "-o" || *arg == "--output");
        let output = Command::new(assert_cmd::cargo_bin!("monolith"))
            .args(if writes_to_file {
                vec![]
            } else {
                vec!["-o", "-"]
            })
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

#[cfg(test)]
mod what_other_origins_allowed {
    use super::helpers::{monolith, warc_of};

    /// Page with fonts from its own origin, and another one, some allowed and some not.
    fn site() -> Vec<u8> {
        warc_of(&[
            (
                "https://a.test/",
                "text/html",
                "<title>Fonts</title><style>\
                 @font-face{font-family:a;src:url(https://cdn.test/allowed.woff)}\
                 @font-face{font-family:b;src:url(https://cdn.test/denied.woff)}\
                 @font-face{font-family:c;src:url(/own.woff)}</style>",
            ),
            // Headers other than Content-Type go along with it
            (
                "https://cdn.test/allowed.woff",
                "font/woff\r\nAccess-Control-Allow-Origin: *",
                "ALLOWEDFONT",
            ),
            ("https://cdn.test/denied.woff", "font/woff", "DENIEDFONT"),
            ("https://a.test/own.woff", "font/woff", "OWNFONT"),
        ])
    }

    #[test]
    fn fonts_out_of_archives() {
        for args in [&["-j"][..], &[][..]] {
            let (output, errors, success) = monolith(&site(), args);

            assert!(success, "{:?}", args);
            let style = &output[output.find("<style>").unwrap()..output.find("</style>").unwrap()];
            assert!(style.contains("ALLOWEDFONT"), "{:?}: {}", args, style);
            assert!(style.contains("OWNFONT"), "{:?}: {}", args, style);
            assert!(!style.contains("DENIEDFONT"), "{:?}: {}", args, style);
            assert!(style.contains("url(\"\")"), "{:?}: {}", args, style);
            assert!(
                errors.contains("https://cdn.test/denied.woff (blocked by CORS)"),
                "{:?}",
                args
            );
        }
    }
}
