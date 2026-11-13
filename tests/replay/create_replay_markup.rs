//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::replay::{CapturedAsset, HttpDetails, create_replay_markup, rewrite_stylesheets};

    fn asset(url: &str, destination: &str) -> CapturedAsset {
        CapturedAsset {
            content_type: "application/json; charset=utf-8".to_string(),
            data: b"{}".to_vec(),
            destination: destination.to_string(),
            is_reusable: true,
            method: "GET".to_string(),
            status: 200,
            urls: vec![url.to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn stores_responses_ahead_of_the_emulator() {
        let markup = create_replay_markup(
            &[asset("https://a.b/c?d=1&e=2", "")],
            &[],
            "https://a.b/",
            7,
        );

        assert!(markup.contains("data-monolith-url=\"https://a.b/c?d=1&amp;e=2\""));
        assert!(markup.contains("data-monolith-type=\"application/json\""));
        assert!(markup.contains(">e30=</script>"));
        assert!(markup.find("e30=").unwrap() < markup.find("CAPTURED_AT=7").unwrap());
    }

    #[test]
    fn stores_things_once() {
        let assets = [asset("https://a.b/c", ""), asset("https://a.b/c", "")];
        let markup = create_replay_markup(&assets, &[], "https://a.b/", 7);

        assert_eq!(markup.matches("data-monolith-url=\"").count(), 1);
    }

    #[test]
    fn stores_modules_separately() {
        let imports = [(
            "https://a.b/c".to_string(),
            "text/javascript".to_string(),
            "e30=".to_string(),
        )];
        let markup = create_replay_markup(
            &[asset("https://a.b/c", "script")],
            &imports,
            "https://a.b/",
            7,
        );

        assert!(!markup.contains("data-monolith-url=\""));
        assert!(markup.contains("data-monolith-modules>[[\"https://a.b/c\""));
    }

    #[test]
    fn page_url_can_not_close_the_script() {
        let markup = create_replay_markup(&[], &[], "https://a.b/</script>", 7);

        assert!(markup.contains("https://a.b/\\u003c/script>"));
    }

    #[test]
    fn leaves_out_what_scripts_never_ask_for() {
        let assets = [
            asset("https://a.b/", "document"),
            asset("https://a.b/f", "font"),
        ];

        assert!(
            !create_replay_markup(&assets, &[], "https://a.b/", 7).contains("data-monolith-url=\"")
        );
    }

    #[test]
    fn remembers_where_redirected_things_were_asked_for() {
        let mut moved = asset("https://a.test/new", "fetch");
        moved.urls = vec![
            "https://a.test/old".to_string(),
            "https://a.test/new".to_string(),
        ];
        let markup = create_replay_markup(&[moved], &[], "https://a.test/", 0);

        assert!(markup.contains(
            "data-monolith-url=\"https://a.test/new\" data-monolith-aliases=\"https://a.test/old\""
        ));
    }

    #[test]
    fn merges_where_identical_responses_were_asked_for() {
        let mut first = asset("https://a.test/shared", "fetch");
        first.urls.insert(0, "https://a.test/old-a".to_string());
        let mut second = asset("https://a.test/shared", "fetch");
        second.urls.insert(0, "https://a.test/old-b".to_string());
        let markup = create_replay_markup(&[first, second], &[], "https://a.test/", 0);

        assert_eq!(markup.matches("data-monolith-url=").count(), 1);
        assert!(
            markup.contains("data-monolith-aliases=\"https://a.test/old-a https://a.test/old-b\"")
        );
    }

    /// What has come from another origin in CORS mode, with given response headers.
    fn from_other_origin(url: &str, destination: &str, headers: &[(&str, &str)]) -> CapturedAsset {
        CapturedAsset {
            content_type: "font/woff2".to_string(),
            data: format!("FONT {}", url).into_bytes(),
            destination: destination.to_string(),
            http: HttpDetails {
                request_headers: vec![("Origin".to_string(), "https://a.b".to_string())],
                response_headers: headers
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect(),
                ..Default::default()
            },
            ..asset(url, destination)
        }
    }

    #[test]
    fn tells_emulator_what_other_origins_allowed() {
        let markup = create_replay_markup(
            &[
                from_other_origin(
                    "https://c.d/allowed",
                    "",
                    &[("Access-Control-Allow-Origin", "*")],
                ),
                from_other_origin("https://c.d/denied", "", &[]),
            ],
            &[],
            "https://a.b/",
            7,
        );

        assert!(markup.contains(
            "data-monolith-url=\"https://c.d/allowed\" data-monolith-allow-origin=\"*\" \
             data-monolith-allow-credentials=\"\""
        ));
        assert!(markup.contains(
            "data-monolith-url=\"https://c.d/denied\" data-monolith-allow-origin=\"\" \
             data-monolith-allow-credentials=\"\""
        ));
    }

    #[test]
    fn stylesheets_get_only_fonts_that_were_allowed() {
        let mut stylesheet = asset("https://a.b/style.css", "style");
        stylesheet.content_type = "text/css".to_string();
        stylesheet.data = b"@font-face{font-family:a;src:url(https://c.d/allowed)}\
            @font-face{font-family:b;src:url(https://c.d/denied)}"
            .to_vec();
        let mut assets = vec![
            stylesheet,
            from_other_origin(
                "https://c.d/allowed",
                "font",
                &[("Access-Control-Allow-Origin", "*")],
            ),
            from_other_origin("https://c.d/denied", "font", &[]),
        ];

        rewrite_stylesheets(&mut assets, None);

        let stylesheet = String::from_utf8(assets[0].data.clone()).unwrap();
        assert!(
            stylesheet.contains("url(\"data:font/woff2;base64,"),
            "{}",
            stylesheet
        );
        // What browsers didn't let the page use stays where it was
        assert!(
            stylesheet.contains("url(https://c.d/denied)"),
            "{}",
            stylesheet
        );
    }

    #[test]
    fn stylesheets_lose_references_to_source_maps() {
        let mut stylesheet = asset("https://a.b/style.css", "style");
        stylesheet.content_type = "text/css".to_string();
        stylesheet.data = b"a{}\n/*# sourceMappingURL=style.css.map */\n".to_vec();
        let mut assets = vec![stylesheet];

        rewrite_stylesheets(&mut assets, None);

        assert_eq!(String::from_utf8(assets[0].data.clone()).unwrap(), "a{}\n");
    }
}
