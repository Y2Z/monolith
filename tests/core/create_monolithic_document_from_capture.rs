//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::{
        MonolithOptions, MonolithOutputFormat, create_monolithic_document_from_capture,
    };
    use monolith::replay::{CapturedAsset, CapturedPage};
    use monolith::session::Session;

    fn session() -> Session {
        let options = MonolithOptions {
            silent: true,
            remove_images: true,
            ..Default::default()
        };

        Session::new(None, None, options)
    }

    fn asset(url: &str, destination: &str, content_type: &str, data: &str) -> CapturedAsset {
        CapturedAsset {
            content_type: content_type.to_string(),
            data: data.as_bytes().to_vec(),
            destination: destination.to_string(),
            is_reusable: true,
            method: "GET".to_string(),
            status: 200,
            urls: vec![url.to_string()],
            ..Default::default()
        }
    }

    fn page() -> CapturedPage {
        CapturedPage {
            assets: vec![
                asset(
                    "https://a.b/",
                    "document",
                    "text/html",
                    "<html><head><title>Before</title><script src=\"a.js\"></script></head></html>",
                ),
                asset(
                    "https://a.b/a.js",
                    "script",
                    "text/javascript",
                    "run(location.href);",
                ),
                asset("https://a.b/api", "", "application/json", "{}"),
            ],
            content_type: "text/html".to_string(),
            html: "<html><head><title>After</title></head><body>Rendered</body></html>".to_string(),
            adopted_style_marker: None,
            title: "After".to_string(),
            url: "https://a.b/".to_string(),
        }
    }

    #[test]
    fn replays_responses_if_scripts_are_kept() {
        let (data, title) = create_monolithic_document_from_capture(session(), page()).unwrap();
        let html = String::from_utf8_lossy(&data);

        assert_eq!(title, Some("After".to_string()));
        assert!(html.contains("data-monolith-url=\"https://a.b/api\""));
        assert!(html.contains("PAGE_URL=\"https://a.b/\""));
        assert!(html.contains("run(__monolithLocation.href);"));
        assert!(!html.contains("Rendered"));
    }

    #[test]
    fn freezes_the_page_without_scripts() {
        let mut session = session();
        session.options.remove_js = true;

        let (data, title) = create_monolithic_document_from_capture(session, page()).unwrap();
        let html = String::from_utf8_lossy(&data);

        assert_eq!(title, Some("After".to_string()));
        assert!(html.contains("Rendered"));
        assert!(!html.contains("data-monolith-url"));
    }

    #[test]
    fn freezes_the_page_for_mhtml() {
        let mut session = session();
        session.options.output_format = MonolithOutputFormat::MHTML;

        let (data, _) = create_monolithic_document_from_capture(session, page()).unwrap();

        assert!(!String::from_utf8_lossy(&data).contains("data-monolith-url"));
    }

    #[test]
    fn freezes_the_page_if_original_document_is_missing() {
        let mut page = page();
        page.assets.remove(0);

        let (data, _) = create_monolithic_document_from_capture(session(), page).unwrap();

        assert!(String::from_utf8_lossy(&data).contains("Rendered"));
    }

    #[test]
    fn embeds_captured_assets_into_frozen_pages() {
        let mut page = page();
        page.assets.remove(0);
        page.assets.push(asset(
            "https://a.b/a.css",
            "style",
            "text/css",
            "p { color: red; }",
        ));
        page.html =
            "<html><head><link rel=\"stylesheet\" href=\"a.css\"></head></html>".to_string();

        let (data, _) = create_monolithic_document_from_capture(session(), page).unwrap();

        assert!(String::from_utf8_lossy(&data).contains("<style>"));
    }
}
