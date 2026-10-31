//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use assert_cmd::cargo_bin_cmd;
    use std::env;
    use std::fs;
    use std::path::MAIN_SEPARATOR;
    use url::Url;

    use monolith::core::{
        MonolithOptions, MonolithOutputFormat, create_monolithic_document_from_data,
    };
    use monolith::mhtml::{MHTML_BOUNDARY, parse_mhtml};
    use monolith::session::Session;
    use monolith::url::parse_data_url;

    /// Returns what's between the given prefix and the closing quote.
    fn attr_value<'a>(text: &'a str, prefix: &str) -> &'a str {
        let start: usize = text.find(prefix).expect(prefix) + prefix.len();
        let end: usize = start + text[start..].find('"').unwrap();
        &text[start..end]
    }

    fn silent_session(output_format: MonolithOutputFormat) -> Session {
        let mut options = MonolithOptions::default();
        options.silent = true;
        options.no_metadata = true;
        options.output_format = output_format;
        Session::new(None, None, options)
    }

    #[test]
    fn mhtml_to_html() {
        let data = fs::read("tests/_data_/mhtml/chrome.mhtml").unwrap();
        let session = silent_session(MonolithOutputFormat::HTML);

        let (result, title) =
            create_monolithic_document_from_data(session, data, None, None).unwrap();
        let result = String::from_utf8_lossy(&result);

        assert_eq!(title, Some("Archived page".to_string()));

        // Assets get embedded straight from the archive
        let (media_type, _charset, data) =
            parse_data_url(&Url::parse(attr_value(&result, r#"<img src=""#)).unwrap());
        assert_eq!(media_type, "image/png");
        assert_eq!(data, b"\x89PNG\r\n\x1a\n");

        // ...including the ones referenced by stylesheets
        let (media_type, _charset, css) = parse_data_url(
            &Url::parse(attr_value(&result, r#"<link rel="stylesheet" href=""#)).unwrap(),
        );
        assert_eq!(media_type, "text/css");
        let css = String::from_utf8(css).unwrap();
        let (media_type, _charset, data) =
            parse_data_url(&Url::parse(attr_value(&css, r#"url(""#)).unwrap());
        assert_eq!(media_type, "image/gif");
        assert_eq!(data, b"GIF89a");

        // Frames are referenced by cid: URLs
        assert!(result.contains(r#"<iframe src="data:text/html"#));
        assert!(!result.contains("cid:"));
        // Assets that aren't in the archive remain remote references
        assert!(result.contains(r#"<img src="https://example.com/missing.png">"#));
    }

    #[test]
    fn mhtml_to_mhtml() {
        let data = fs::read("tests/_data_/mhtml/chrome.mhtml").unwrap();
        let session = silent_session(MonolithOutputFormat::MHTML);

        let (result, _title) =
            create_monolithic_document_from_data(session, data, None, None).unwrap();
        let archive = parse_mhtml(&result, None).unwrap();

        assert_eq!(
            archive.root().url.as_ref().unwrap().as_str(),
            "https://example.com/page/"
        );
        assert!(
            archive
                .get(&Url::parse("https://example.com/page/img.png").unwrap())
                .is_some()
        );
        assert!(
            archive
                .get(&Url::parse("https://example.com/page/bg.gif").unwrap())
                .is_some()
        );
        assert!(
            archive
                .get(&Url::parse("cid:frame-2@mhtml.blink").unwrap())
                .is_some()
        );
    }

    #[test]
    fn local_file_to_mhtml_and_back() {
        let cwd: String = env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("\\", "/");
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

        let out = cargo_bin_cmd!(env!("CARGO_PKG_NAME"))
            .arg("-M")
            .arg("-m")
            .arg(format!(
                "tests{s}_data_{s}basic{s}local-file.html",
                s = MAIN_SEPARATOR
            ))
            .output()
            .unwrap();
        let mhtml = out.stdout;

        let text = String::from_utf8_lossy(&mhtml);
        assert!(text.starts_with("Snapshot-Content-Location: "));
        assert!(text.contains(&format!("boundary=\"{}\"", MHTML_BOUNDARY)));
        assert!(text.ends_with(&format!("--{}--\r\n", MHTML_BOUNDARY)));

        let archive = parse_mhtml(&mhtml, None).unwrap();
        assert_eq!(
            archive.root().url.as_ref().unwrap().as_str(),
            format!(
                "{}{}/tests/_data_/basic/local-file.html",
                file_url_protocol, cwd
            )
        );
        assert_eq!(archive.root().charset, "utf-8");
        let script_url = Url::parse(&format!(
            "{}{}/tests/_data_/basic/local-script.js",
            file_url_protocol, cwd
        ))
        .unwrap();
        assert_eq!(
            archive.get(&script_url).unwrap().data,
            fs::read("tests/_data_/basic/local-script.js").unwrap()
        );

        // Turning it back into HTML gets the script embedded, without touching the filesystem
        let session = silent_session(MonolithOutputFormat::HTML);
        let (html, _title) =
            create_monolithic_document_from_data(session, mhtml, None, None).unwrap();
        let html = String::from_utf8_lossy(&html);
        assert!(html.contains(r#"document.body.style.backgroundColor = "green";"#));
        assert!(!html.contains("local-script.js"));
    }
}
