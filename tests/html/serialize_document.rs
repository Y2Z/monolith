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

    #[test]
    fn div_as_root_element() {
        let html = "<div><script src=\"some.js\"></script></div>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let options = MonolithOptions::default();

        assert_eq!(
            String::from_utf8_lossy(&html::serialize_document(dom, "".to_string(), &options)),
            "<html><head></head><body><div><script src=\"some.js\"></script></div></body></html>"
        );
    }

    #[test]
    fn full_page_with_no_html_head_or_body() {
        let html = "<title>Isolated document</title>\
                    <link rel=\"something\" href=\"some.css\" />\
                    <div><script src=\"some.js\"></script></div>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let options = MonolithOptions {
            isolate: true,
            ..Default::default()
        };

        // What Monolith keeps documents from doing goes first (see csp.rs)
        html::set_content_security_policies(&dom, &[html::compose_csp(&options)]);

        assert_eq!(
            String::from_utf8_lossy(&html::serialize_document(dom, "".to_string(), &options)),
            "<html>\
                <head>\
                    <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-eval' 'unsafe-inline' data:;\">\
                    <title>Isolated document</title>\
                    <link rel=\"something\" href=\"some.css\">\
                </head>\
                <body>\
                    <div>\
                        <script src=\"some.js\"></script>\
                    </div>\
                </body>\
            </html>"
        );
    }

    #[test]
    fn doctype_and_the_rest_no_html_head_or_body() {
        let html = "<!doctype html>\
                    <title>Unstyled document</title>\
                    <link rel=\"stylesheet\" href=\"main.css\"/>\
                    <div style=\"display: none;\"></div>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let options = MonolithOptions {
            remove_styles: true,
            ..Default::default()
        };

        // What Monolith keeps documents from doing goes first (see csp.rs)
        html::set_content_security_policies(&dom, &[html::compose_csp(&options)]);

        assert_eq!(
            String::from_utf8_lossy(&html::serialize_document(dom, "".to_string(), &options)),
            "<!DOCTYPE html>\
            <html>\
            <head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none';\">\
            <title>Unstyled document</title>\
            <link rel=\"stylesheet\" href=\"main.css\">\
            </head>\
            <body><div style=\"display: none;\"></div></body>\
            </html>"
        );
    }

    #[test]
    fn doctype_and_the_rest_no_html_head_or_body_forbid_frames() {
        let html = "<!doctype html>\
                    <title>Frameless document</title>\
                    <link rel=\"something\"/>\
                    <div><script src=\"some.js\"></script></div>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let options = MonolithOptions {
            wipe_elements: vec!["frame,iframe".to_string()],
            ..Default::default()
        };

        // What Monolith keeps documents from doing goes first (see csp.rs)
        html::set_content_security_policies(&dom, &[html::compose_csp(&options)]);

        assert_eq!(
            String::from_utf8_lossy(&html::serialize_document(dom, "".to_string(), &options)),
            "<!DOCTYPE html>\
                <html>\
                <head>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"frame-src 'none'; child-src 'none';\">\
                <title>Frameless document</title>\
                <link rel=\"something\">\
                </head>\
                <body><div><script src=\"some.js\"></script></div></body>\
                </html>"
        );
    }

    #[test]
    fn doctype_and_the_rest_all_forbidden() {
        let html = "<!doctype html>\
                    <title>no-frame no-css no-js no-image isolated document</title>\
                    <link rel=\"stylesheet\" href=\"some.css\">\
                    <div>\
                        <script src=\"some.js\"></script>\
                        <img style=\"width: 100%;\" src=\"some.png\" />\
                        <iframe src=\"some.html\"></iframe>\
                    </div>";
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        let options = MonolithOptions {
            isolate: true,
            remove_styles: true,
            remove_fonts: true,
            wipe_elements: vec!["frame,iframe".to_string()],
            remove_js: true,
            remove_images: true,
            ..Default::default()
        };

        // What Monolith keeps documents from doing goes first (see csp.rs)
        html::set_content_security_policies(&dom, &[html::compose_csp(&options)]);

        assert_eq!(
            String::from_utf8_lossy(&html::serialize_document(dom, "".to_string(), &options)),
            "<!DOCTYPE html>\
                <html>\
                    <head>\
                        <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-eval' 'unsafe-inline' data:; style-src 'none'; font-src 'none'; frame-src 'none'; child-src 'none'; script-src 'none'; object-src 'none'; img-src data:;\">\
                        <title>no-frame no-css no-js no-image isolated document</title>\
                            <link rel=\"stylesheet\" href=\"some.css\">\
                    </head>\
                    <body>\
                        <div>\
                            <script src=\"some.js\"></script>\
                            <img style=\"width: 100%;\" src=\"some.png\">\
                            <iframe src=\"some.html\"></iframe>\
                        </div>\
                    </body>\
                </html>"
        );
    }
}

#[cfg(test)]
mod streaming {
    use monolith::core::MonolithOptions;
    use monolith::html;
    use monolith::replay::insert_replay_markup;

    /// Accepts a byte at a time, to make sure nothing depends on how things are written.
    struct Trickle(Vec<u8>);

    impl std::io::Write for Trickle {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            match buf.first() {
                Some(byte) => {
                    self.0.push(*byte);
                    Ok(1)
                }
                None => Ok(0),
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn serialize_both_ways(html: &str, encoding: &str, markup: &str) -> (Vec<u8>, Vec<u8>) {
        let options = MonolithOptions::default();
        let dom = html::html_to_dom(html.as_bytes(), "utf-8".to_string());
        let whole = insert_replay_markup(
            html::serialize_document(dom, encoding.to_string(), &options),
            markup,
        );
        let dom = html::html_to_dom(html.as_bytes(), "utf-8".to_string());
        let mut streamed = Trickle(vec![]);
        let last_byte = html::serialize_document_into(
            dom,
            encoding.to_string(),
            &options,
            Some(markup.as_bytes()),
            &mut streamed,
        )
        .unwrap();
        assert_eq!(last_byte, streamed.0.last().copied());

        (whole, streamed.0)
    }

    #[test]
    fn markup_goes_right_after_head_tag() {
        let (whole, streamed) = serialize_both_ways(
            "<!-- <hea --><HEAD data-x=\"1\"><title>T</title></HEAD><body>Hi</body>",
            "utf-8",
            "<script>replay()</script>",
        );

        assert_eq!(streamed, whole);
        assert!(
            String::from_utf8_lossy(&streamed)
                .contains("<head data-x=\"1\"><script>replay()</script><title>")
        );
    }

    #[test]
    fn characters_get_encoded_whole() {
        let text = "メイリオ — ü ✓ 🙂 ".repeat(2000);
        for encoding in [
            "Shift_JIS",
            "windows-1252",
            "ISO-2022-JP",
            "UTF-16LE",
            "utf-8",
        ] {
            let (whole, streamed) = serialize_both_ways(
                &format!("<html><head></head><body><p>{}</p></body></html>", text),
                encoding,
                "",
            );

            assert_eq!(streamed, whole, "{}", encoding);
        }
    }

    #[test]
    fn unencodable_characters_become_references() {
        let (_, streamed) = serialize_both_ways("<body>✓</body>", "windows-1252", "");

        assert!(String::from_utf8_lossy(&streamed).contains("&#10003;"));
    }
}
