//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use html5ever::serialize::{SerializeOpts, serialize};
    use markup5ever_rcdom::SerializableHandle;
    use url::Url;

    use monolith::core::MonolithOptions;
    use monolith::html;
    use monolith::session::Session;
    use monolith::url::EMPTY_IMAGE_DATA_URL;

    fn serialized(dom: &markup5ever_rcdom::RcDom) -> String {
        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn iframe_data_urls(html: &str) -> Vec<String> {
        let mut urls: Vec<String> = vec![];
        let mut rest = html;

        while let Some(start) = rest.find("<iframe src=\"") {
            rest = &rest[start + "<iframe src=\"".len()..];
            let end = rest.find('"').unwrap();
            urls.push(rest[..end].to_string());
            rest = &rest[end..];
        }

        urls
    }

    fn decoded_data_url(url: &str) -> String {
        let (_, _, data) = monolith::url::parse_data_url(&Url::parse(url).unwrap());
        String::from_utf8(data).unwrap()
    }

    #[test]
    fn linked_stylesheet_encoding() {
        let url = Url::parse("https://example.com/").unwrap();

        for (charset, bytes, expected) in [
            (
                "windows-1252",
                b"p::before{content:'caf\xe9'}".as_slice(),
                "p::before{content:\"café\"}",
            ),
            (
                "utf-8",
                "p::before{content:'café'}".as_bytes(),
                "p::before{content:\"café\"}",
            ),
            (
                "",
                "p::before{content:'café'}".as_bytes(),
                "p::before{content:\"café\"}",
            ),
            (
                "",
                b"@charset \"windows-1252\";p::before{content:'caf\xe9'}".as_slice(),
                "@charset \"UTF-8\";p::before{content:\"café\"}",
            ),
        ] {
            let asset = monolith::url::create_data_url("text/css", charset, bytes, &url, None);
            let source = format!("<link rel=\"stylesheet\" href=\"{}\">", asset);
            let dom = html::html_to_dom(&source.into_bytes(), "utf-8".to_string());
            let mut session = Session::new(None, None, MonolithOptions::default());

            html::walk(&mut session, &url, &dom.document);

            let nodes = html::find_nodes(&dom.document, vec!["html", "head", "link"]);
            let href = html::get_node_attr(&nodes[0], "href").unwrap();
            let (media_type, embedded_charset, data) =
                monolith::url::parse_data_url(&Url::parse(&href).unwrap());
            assert_eq!(media_type, "text/css");
            assert_eq!(embedded_charset, "utf-8");
            assert_eq!(String::from_utf8(data).unwrap(), expected);
        }
    }

    /// Finds the first data URL of the given media type in serialized markup
    /// (or in a bare attribute value) and decodes it
    fn find_data_url(markup: &str, media_type: &str) -> (String, String) {
        let start = markup
            .find(&format!("data:{}", media_type))
            .expect("no data URL found");
        let end = start + markup[start..].find('"').unwrap_or(markup.len() - start);
        let (_, charset, data) =
            monolith::url::parse_data_url(&Url::parse(&markup[start..end]).unwrap());
        (
            charset,
            String::from_utf8(data).expect("embedded data is not UTF-8"),
        )
    }

    // "p{font-family:'メイリオ'}" in Shift_JIS, without any charset declared
    const SHIFT_JIS_CSS: &[u8] = b"p{font-family:'\x83\x81\x83\x43\x83\x8a\x83\x49'}";

    #[test]
    fn linked_stylesheet_environment_encoding() {
        let url = Url::parse("https://example.com/").unwrap();
        let asset = monolith::url::create_data_url("text/css", "", SHIFT_JIS_CSS, &url, None);
        let source = format!("<link rel=\"stylesheet\" href=\"{}\">", asset);
        let dom = html::html_to_dom(&source.into_bytes(), "utf-8".to_string());
        let mut session = Session::new(None, None, MonolithOptions::default());
        session.environment_encoding = Some(encoding_rs::SHIFT_JIS);

        html::walk(&mut session, &url, &dom.document);

        let nodes = html::find_nodes(&dom.document, vec!["html", "head", "link"]);
        let href = html::get_node_attr(&nodes[0], "href").unwrap();
        assert_eq!(
            find_data_url(&href, "text/css"),
            (
                "utf-8".to_string(),
                "p{font-family:\"メイリオ\"}".to_string()
            )
        );
    }

    #[test]
    fn frame_encoding() {
        let url = Url::parse("https://example.com/").unwrap();
        let css = monolith::url::create_data_url("text/css", "", SHIFT_JIS_CSS, &url, None);
        let mut frame_html: Vec<u8> = format!(
            "<html><head><meta charset=\"Shift_JIS\"><link rel=\"stylesheet\" href=\"{}\"></head><body>",
            css
        )
        .into_bytes();
        frame_html.extend_from_slice(b"\x83\x81\x83\x43\x83\x8a\x83\x49</body></html>");
        let frame = monolith::url::create_data_url("text/html", "", frame_html, &url, None);
        let source = format!("<iframe src=\"{}\"></iframe>", frame);
        let dom = html::html_to_dom(&source.into_bytes(), "utf-8".to_string());
        let mut session = Session::new(None, None, MonolithOptions::default());

        html::walk(&mut session, &url, &dom.document);

        let nodes = html::find_nodes(&dom.document, vec!["html", "body", "iframe"]);
        let src = html::get_node_attr(&nodes[0], "src").unwrap();
        let (frame_charset, frame_markup) = find_data_url(&src, "text/html");
        assert_eq!(frame_charset, "utf-8");
        assert!(frame_markup.contains("<body>メイリオ</body>"));
        assert_eq!(
            find_data_url(&frame_markup, "text/css"),
            (
                "utf-8".to_string(),
                "p{font-family:\"メイリオ\"}".to_string()
            )
        );
        assert_eq!(session.environment_encoding, None);
    }

    #[test]
    fn sources_that_only_scripts_know_about_get_left_alone() {
        let url = Url::from_file_path(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/_data_/svg/index.html"),
        )
        .unwrap();
        for remove_images in [false, true] {
            let source = "<img src=\"data:,placeholder\" data-src=\"image.svg\">".to_string();
            let dom = html::html_to_dom(&source.into_bytes(), "".to_string());
            let options = MonolithOptions {
                remove_images,
                silent: true,
                ..Default::default()
            };
            let mut session = Session::new(None, None, options);

            html::walk(&mut session, &url, &dom.document);

            let nodes = html::find_nodes(&dom.document, vec!["html", "body", "img"]);
            assert_eq!(
                html::get_node_attr(&nodes[0], "data-src"),
                Some("image.svg".to_string())
            );
            let src = html::get_node_attr(&nodes[0], "src").unwrap();
            assert!(!src.contains("svg"), "{}", src);
        }
    }

    #[test]
    fn basic() {
        let html: &str = "<div><P></P></div>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><div><p></p></div></body></html>"
        );
    }

    #[test]
    fn ensure_no_recursive_iframe() {
        let html = "<div><P></P><iframe src=\"\"></iframe></div>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><div><p></p><iframe src=\"\"></iframe></div></body></html>"
        );
    }

    #[test]
    fn ensure_no_recursive_frame() {
        let html = "<frameset><frame src=\"\"></frameset>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><frameset><frame src=\"\"></frameset></html>"
        );
    }

    #[test]
    fn remove_styles() {
        let html = "\
            <link rel=\"stylesheet\" href=\"main.css\">\
            <link rel=\"alternate stylesheet\" href=\"main.css\">\
            <style>html{background-color: #000;}</style>\
            <div style=\"display: none;\"></div>\
        ";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_styles: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head>\
                    <link rel=\"stylesheet\">\
                    <link rel=\"alternate stylesheet\">\
                    <style></style>\
                </head>\
                <body>\
                    <div></div>\
                </body>\
            </html>\
            "
        );
    }

    #[test]
    fn remove_images() {
        let html = "<link rel=\"icon\" href=\"favicon.ico\">\
                    <div><img src=\"http://localhost/assets/mono_lisa.png\" /></div>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_images: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            format!(
                "<html>\
                    <head>\
                        <link rel=\"icon\">\
                    </head>\
                    <body>\
                        <div>\
                            <img src=\"{empty_image}\">\
                        </div>\
                    </body>\
                </html>",
                empty_image = EMPTY_IMAGE_DATA_URL
            )
        );
    }

    #[test]
    fn no_body_background_images() {
        let html =
            "<body background=\"no/such/image.png\" background=\"no/such/image2.png\"></body>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_images: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body></body></html>"
        );
    }

    #[test]
    fn no_frames() {
        let html = "<frameset><frame src=\"http://trackbook.com\"></frameset>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::strip_elements(&dom.document, &[], &["frame".to_string()]);
        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head>\
                </head>\
                <frameset>\
                    <frame>\
                </frameset>\
            </html>\
            "
        );
    }

    #[test]
    fn no_iframes() {
        let html = "<iframe src=\"http://trackbook.com\"></iframe>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::strip_elements(&dom.document, &[], &["iframe".to_string()]);
        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head></head>\
                <body>\
                    <iframe></iframe>\
                </body>\
            </html>\
            "
        );
    }

    #[test]
    fn remove_js() {
        let html = "\
            <div onClick=\"void(0)\">\
                <script src=\"http://localhost/assets/some.js\"></script>\
                <script>alert(1)</script>\
            </div>\
        ";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_js: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head></head>\
                <body>\
                    <div>\
                        <script></script>\
                        <script></script>\
                    </div>\
                </body>\
            </html>\
            "
        );
    }

    #[test]
    fn keeps_integrity_for_unfamiliar_links() {
        let html = "<title>Has integrity</title>\
                    <link integrity=\"sha384-12345\" rel=\"something\" href=\"https://some-site.com/some-file.ext\" />";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head>\
                    <title>Has integrity</title>\
                    <link integrity=\"sha384-12345\" rel=\"something\" href=\"https://some-site.com/some-file.ext\">\
                </head>\
                <body></body>\
            </html>\
            "
        );
    }

    #[test]
    fn discards_integrity_for_known_links_nojs_nocss() {
        let html = "\
            <title>No integrity</title>\
            <link integrity=\"\" rel=\"stylesheet\" href=\"data:;\"/>\
            <script integrity=\"\" src=\"some.js\"></script>\
        ";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_styles: true,
            remove_js: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head>\
                    <title>No integrity</title>\
                    <link rel=\"stylesheet\">\
                    <script></script>\
                </head>\
                <body></body>\
            </html>\
            "
        );
    }

    #[test]
    fn discards_integrity_for_embedded_assets() {
        let html = "\
            <title>No integrity</title>\
            <link integrity=\"sha384-123\" rel=\"something\" href=\"data:;\"/>\
            <script integrity=\"sha384-456\" src=\"some.js\"></script>\
        ";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_styles: true,
            remove_js: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head>\
                    <title>No integrity</title>\
                    <link integrity=\"sha384-123\" rel=\"something\" href=\"data:;\">\
                    <script></script>\
                </head>\
                <body>\
                </body>\
            </html>\
            "
        );
    }

    #[test]
    fn removes_unwanted_meta_tags() {
        let html = "\
            <html>\
                <head>\
                    <meta http-equiv=\"Refresh\" content=\"2\"/>\
                    <meta http-equiv=\"Location\" content=\"https://freebsd.org\"/>\
                </head>\
                <body>\
                </body>\
            </html>\
        ";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_styles: true,
            remove_js: true,
            remove_images: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head>\
                    <meta content=\"2\">\
                    <meta content=\"https://freebsd.org\">\
                </head>\
                <body>\
                </body>\
            </html>"
        );
    }

    #[test]
    fn processes_noscript_tags() {
        let html = "\
        <html>\
            <body>\
                <noscript>\
                    <img src=\"image.png\" />\
                </noscript>\
            </body>\
        </html>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_images: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            format!(
                "\
                <html>\
                    <head>\
                    </head>\
                    <body>\
                        <noscript>\
                            <img src=\"{}\">\
                        </noscript>\
                    </body>\
                </html>",
                EMPTY_IMAGE_DATA_URL,
            )
        );
    }

    #[test]
    fn self_referencing_iframe() {
        let url: Url = Url::parse("https://example.com/page.html").unwrap();
        let html = "<html><body><iframe src=\"page.html\"></iframe></body></html>";
        let mut session: Session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &url,
            &html.as_bytes().to_vec(),
            "text/html".to_string(),
            "".to_string(),
        );

        let dom = html::html_to_dom(html.as_bytes(), "utf-8".to_string());
        html::walk(&mut session, &url, &dom.document);

        let output = serialized(&dom);
        assert!(!output.contains("page.html"), "{output}");
        assert!(output.contains("data:text/html;charset=utf-8,"), "{output}");
    }

    #[test]
    fn mutually_referencing_iframes() {
        let url_a: Url = Url::parse("https://example.com/a.html").unwrap();
        let url_b: Url = Url::parse("https://example.com/b.html").unwrap();
        let html_a = "<html><body><p>a marker</p><iframe src=\"b.html\"></iframe></body></html>";
        let html_b = "<html><body><p>b marker</p><iframe src=\"a.html\"></iframe></body></html>";
        let mut session: Session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &url_b,
            &html_b.as_bytes().to_vec(),
            "text/html".to_string(),
            "".to_string(),
        );
        session.add_asset(
            &url_a,
            &html_a.as_bytes().to_vec(),
            "text/html".to_string(),
            "".to_string(),
        );

        let dom = html::html_to_dom(html_a.as_bytes(), "utf-8".to_string());
        html::walk(&mut session, &url_a, &dom.document);

        let output = serialized(&dom);
        assert!(output.contains("a marker"), "{output}");
        let frames = iframe_data_urls(&output);
        assert_eq!(frames.len(), 1, "{output}");
        assert!(
            decoded_data_url(&frames[0]).contains("b marker"),
            "{output}"
        );
    }

    #[test]
    fn sibling_iframes_with_same_src_both_embed() {
        let parent_url: Url = Url::parse("https://example.com/parent.html").unwrap();
        let child_url: Url = Url::parse("https://example.com/child.html").unwrap();
        let child = "<html><body><p>child marker</p></body></html>";
        let parent = "\
            <html><body>\
                <iframe src=\"child.html\"></iframe>\
                <iframe src=\"child.html\"></iframe>\
            </body></html>";
        let mut session: Session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &child_url,
            &child.as_bytes().to_vec(),
            "text/html".to_string(),
            "".to_string(),
        );

        let dom = html::html_to_dom(parent.as_bytes(), "utf-8".to_string());
        html::walk(&mut session, &parent_url, &dom.document);

        // Expanding the same document twice in a row is not a cycle
        let output = serialized(&dom);
        let frames = iframe_data_urls(&output);
        assert_eq!(frames.len(), 2, "{output}");
        for frame in frames {
            assert!(
                decoded_data_url(&frame).contains("child marker"),
                "{output}"
            );
        }
    }

    #[test]
    fn self_referencing_svg_use() {
        let url: Url = Url::parse("https://example.com/icon.svg").unwrap();
        let svg = "\
            <svg xmlns=\"http://www.w3.org/2000/svg\">\
                <use href=\"icon.svg\"></use>\
            </svg>";
        let mut session: Session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &url,
            &svg.as_bytes().to_vec(),
            "image/svg+xml".to_string(),
            "".to_string(),
        );

        let dom = html::html_to_dom(svg.as_bytes(), "utf-8".to_string());
        html::walk(&mut session, &url, &dom.document);

        let output = serialized(&dom);
        assert!(!output.contains("href=\"icon.svg\""), "{output}");
    }

    #[test]
    fn mutually_referencing_svg_use() {
        let page_url: Url = Url::parse("https://example.com/page.html").unwrap();
        let url_a: Url = Url::parse("https://example.com/a.svg").unwrap();
        let url_b: Url = Url::parse("https://example.com/b.svg").unwrap();
        let svg_a = "\
            <svg xmlns=\"http://www.w3.org/2000/svg\">\
                <use href=\"b.svg\"></use>\
                <text>a marker</text>\
            </svg>";
        let svg_b = "\
            <svg xmlns=\"http://www.w3.org/2000/svg\">\
                <use href=\"a.svg\"></use>\
                <text>b marker</text>\
            </svg>";
        let page = "<html><body><svg><use href=\"a.svg\"></use></svg></body></html>";
        let mut session: Session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &url_b,
            &svg_b.as_bytes().to_vec(),
            "image/svg+xml".to_string(),
            "".to_string(),
        );
        session.add_asset(
            &url_a,
            &svg_a.as_bytes().to_vec(),
            "image/svg+xml".to_string(),
            "".to_string(),
        );

        let dom = html::html_to_dom(page.as_bytes(), "utf-8".to_string());
        html::walk(&mut session, &page_url, &dom.document);

        let output = serialized(&dom);
        assert!(output.contains("a marker"), "{output}");
        assert!(output.contains("b marker"), "{output}");
        assert_eq!(output.matches("href=\"a.svg\"").count(), 1, "{output}");
        assert_eq!(output.matches("href=\"b.svg\"").count(), 1, "{output}");
    }

    #[test]
    fn svg_use_cycle_in_subdirectory_does_not_hang() {
        let page_url: Url = Url::parse("https://example.com/index.html").unwrap();
        let icon_url: Url = Url::parse("https://example.com/sub/icon.svg").unwrap();
        let icon = "\
            <svg xmlns=\"http://www.w3.org/2000/svg\">\
                <use href=\"icon.svg\"></use>\
                <text>icon marker</text>\
            </svg>";
        let page = "<html><body><svg><use href=\"sub/icon.svg\"></use></svg></body></html>";
        let mut session: Session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &icon_url,
            &icon.as_bytes().to_vec(),
            "image/svg+xml".to_string(),
            "".to_string(),
        );

        let dom = html::html_to_dom(page.as_bytes(), "utf-8".to_string());
        html::walk(&mut session, &page_url, &dom.document);

        // The SVG contents are relative to where the SVG is, so the self reference resolves
        // to the SVG itself and gets cut off
        let output = serialized(&dom);
        assert!(output.contains("icon marker"), "{output}");
        assert_eq!(output.matches("icon marker").count(), 1, "{output}");
        assert!(!output.contains("href=\"icon.svg\""), "{output}");
        // The self reference would resolve here if it were taken to be relative to the page
        assert!(!output.contains("https://example.com/icon.svg"), "{output}");
    }

    #[test]
    fn svg_use_with_both_hrefs_embeds_once() {
        let page_url: Url = Url::parse("https://example.com/page.html").unwrap();
        let icon_url: Url = Url::parse("https://example.com/icon.svg").unwrap();
        let icon = "\
            <svg xmlns=\"http://www.w3.org/2000/svg\">\
                <symbol id=\"sym\"><text>symbol marker</text></symbol>\
            </svg>";
        let page = "\
            <html><body>\
                <svg><use href=\"icon.svg#sym\" xlink:href=\"icon.svg#sym\"></use></svg>\
            </body></html>";
        let mut session: Session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &icon_url,
            &icon.as_bytes().to_vec(),
            "image/svg+xml".to_string(),
            "".to_string(),
        );

        let dom = html::html_to_dom(page.as_bytes(), "utf-8".to_string());
        html::walk(&mut session, &page_url, &dom.document);

        // Both attributes point at the same symbol, it only gets brought in once
        let output = serialized(&dom);
        assert_eq!(output.matches("symbol marker").count(), 1, "{output}");
        assert_eq!(output.matches("href=\"#sym\"").count(), 2, "{output}");
        assert!(!output.contains("icon.svg"), "{output}");
    }

    #[test]
    fn preserves_script_type_json() {
        let html = "<script id=\"data\" type=\"application/json\">{\"mono\":\"lith\"}</script>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "\
            <html>\
                <head>\
                    <script id=\"data\" type=\"application/json\">{\"mono\":\"lith\"}</script>\
                </head>\
                <body>\
                </body>\
            </html>"
        );
    }

    #[test]
    fn remove_js_from_srcdoc() {
        let html = "<iframe src=\"other.html\" srcdoc=\"\
                <p onclick='void(0)'>a</p><script>alert(1)</script>\
                <iframe srcdoc='&lt;script>alert(2)&lt;/script>'></iframe>\
            \"></iframe>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let options = MonolithOptions {
            remove_js: true,
            silent: true,
            ..Default::default()
        };

        let mut session: Session = Session::new(None, None, options);

        html::walk(&mut session, &url, &dom.document);

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "<html><head></head><body>\
                <iframe srcdoc=\"\
                    &lt;html&gt;&lt;head&gt;&lt;/head&gt;&lt;body&gt;\
                    &lt;p&gt;a&lt;/p&gt;&lt;script&gt;&lt;/script&gt;\
                    &lt;iframe srcdoc=&quot;\
                        &amp;lt;html&amp;gt;&amp;lt;head&amp;gt;\
                        &amp;lt;script&amp;gt;&amp;lt;/script&amp;gt;\
                        &amp;lt;/head&amp;gt;&amp;lt;body&amp;gt;\
                        &amp;lt;/body&amp;gt;&amp;lt;/html&amp;gt;\
                    &quot; sandbox=&quot;allow-forms&quot;&gt;&lt;/iframe&gt;\
                    &lt;/body&gt;&lt;/html&gt;\
                \" sandbox=\"allow-forms\"></iframe>\
            </body></html>"
        );
    }
}
