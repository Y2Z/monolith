//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::{MonolithOptions, create_monolithic_document_from_data};
    use monolith::session::Session;
    use monolith::url::Url;

    fn session() -> Session {
        let mut options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        // Keeps favicons from being requested
        options.remove_images = true;

        Session::new(None, None, options)
    }

    #[test]
    fn retrieves_assets_without_requesting_them() {
        let mut session = session();
        let url = Url::parse("https://example.com/image.png#fragment").unwrap();
        let parent_url = Url::parse("https://example.com/").unwrap();
        session.add_asset(
            &url,
            &vec![1, 2, 3],
            "image/png".to_string(),
            "".to_string(),
        );

        let (data, _, media_type, _) = session.retrieve_asset(&parent_url, &url).unwrap();

        assert_eq!(data, vec![1, 2, 3]);
        assert_eq!(media_type, "image/png");
    }

    #[test]
    fn keeps_scripts_that_have_to_wait_as_data_urls() {
        let mut session = session();
        session.add_asset(
            &Url::parse("https://example.com/a.js").unwrap(),
            &b"run();".to_vec(),
            "text/javascript".to_string(),
            "utf-8".to_string(),
        );

        let (data, _) = create_monolithic_document_from_data(
            session,
            b"<html><head><script defer src=\"a.js\"></script></head><body></body></html>".to_vec(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();
        let html = String::from_utf8_lossy(&data);

        assert!(html.contains("src=\"data:text/javascript"));
        assert!(!html.contains(">run();<"));
    }

    #[test]
    fn modules_of_documents_that_replay_responses_stay_single_instances() {
        let mut session = session();
        session.options.replay = true;

        let (data, _) = create_monolithic_document_from_data(
            session,
            b"<html><head><script type=\"module\" src=\"a.js\"></script></head><body></body></html>"
                .to_vec(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();

        assert!(String::from_utf8_lossy(&data).contains(
            "src=\"data:text/javascript;charset=utf-8,import%20%22https://example.com/a.js%22\""
        ));
    }

    fn replayed_module(integrity: &str) -> String {
        let mut session = session();
        session.options.replay = true;
        session.add_original_data(
            &Url::parse("https://example.com/a.js").unwrap(),
            b"run();".to_vec(),
        );
        let document = format!(
            "<html><head><script type=\"module\" src=\"a.js\" integrity=\"{}\"></script></head></html>",
            integrity
        );

        let (data, _) = create_monolithic_document_from_data(
            session,
            document.into_bytes(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();

        String::from_utf8_lossy(&data).to_string()
    }

    #[test]
    fn integrity_gets_checked_against_what_scripts_used_to_be() {
        // Printed by: printf 'run();' | openssl dgst -sha256 -binary | base64
        let html = replayed_module("sha256-vUOElbTHLp1UxxGV/I6teJfpCV5vgaasYjEhPKN2fqQ=");

        assert!(html.contains("import%20%22https://example.com/a.js%22"));
        assert!(!html.contains("integrity"));
    }

    #[test]
    fn scripts_that_fail_integrity_checks_are_left_out() {
        let html = replayed_module("sha256-AAAA");

        assert!(!html.contains("a.js"));
        assert!(!html.contains("integrity"));
    }

    #[test]
    fn documents_that_replay_responses_are_not_allowed_to_reach_out() {
        let mut session = session();
        session.options.replay = true;

        let (data, _) = create_monolithic_document_from_data(
            session,
            b"<html><head></head><body></body></html>".to_vec(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();

        assert!(
            String::from_utf8_lossy(&data)
                .contains("content=\"default-src 'unsafe-eval' 'unsafe-inline' data: blob:;")
        );
    }

    #[test]
    fn documents_that_replay_responses_do_not_reach_out() {
        let mut session = session();
        session.options.replay = true;

        let (data, _) = create_monolithic_document_from_data(
            session,
            b"<html><head><link rel=\"modulepreload\" href=\"a.js\"></head><body></body></html>"
                .to_vec(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();

        assert!(!String::from_utf8_lossy(&data).contains("a.js"));
    }

    #[test]
    fn embeds_assets_into_saved_shadow_roots() {
        let mut session = session();
        session.options.remove_images = false;
        session.add_asset(
            &Url::parse("https://example.com/a.png").unwrap(),
            &vec![0x89, 0x50, 0x4E, 0x47],
            "image/png".to_string(),
            "".to_string(),
        );

        let (data, _) = create_monolithic_document_from_data(
            session,
            b"<html><body><x-a><template shadowrootmode=\"open\"><img src=\"a.png\"></template></x-a></body></html>"
                .to_vec(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();
        let html = String::from_utf8_lossy(&data);

        assert!(html.contains("shadowrootmode=\"open\""));
        assert!(html.contains("src=\"data:image/png"));
    }

    #[test]
    fn keeps_contents_of_templates() {
        let (data, _) = create_monolithic_document_from_data(
            session(),
            b"<html><body><template><p>Kept</p></template></body></html>".to_vec(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();

        assert!(String::from_utf8_lossy(&data).contains("<template><p>Kept</p></template>"));
    }

    #[test]
    fn removes_scripts_from_templates() {
        let mut session = session();
        session.options.remove_js = true;

        let (data, _) = create_monolithic_document_from_data(
            session,
            b"<html><body><template><script>run();</script></template></body></html>".to_vec(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();

        assert!(!String::from_utf8_lossy(&data).contains("run();"));
    }

    #[test]
    fn embeds_assets_into_rendered_documents() {
        let mut session = session();
        session.add_asset(
            &Url::parse("https://example.com/a.css").unwrap(),
            &b"p { color: red; }".to_vec(),
            "text/css".to_string(),
            "utf-8".to_string(),
        );

        let (data, _) = create_monolithic_document_from_data(
            session,
            b"<html><head><link rel=\"stylesheet\" href=\"a.css\"></head><body></body></html>"
                .to_vec(),
            Some("utf-8".to_string()),
            Some("https://example.com/page".to_string()),
        )
        .unwrap();

        assert!(String::from_utf8_lossy(&data).contains("<style>"));
    }
}
