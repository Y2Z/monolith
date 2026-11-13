//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::replay::CapturedAsset;
    use monolith::replay::rewrite_scripts;

    fn asset(url: &str, data: &str) -> CapturedAsset {
        CapturedAsset {
            content_type: "text/javascript".to_string(),
            data: data.as_bytes().to_vec(),
            destination: "script".to_string(),
            is_reusable: true,
            method: "GET".to_string(),
            status: 200,
            urls: vec![url.to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn provides_all_scripts_and_what_they_import() {
        let mut data = asset("https://a.b/data.json", "{}");
        data.destination = "json".to_string();
        data.content_type = "application/json".to_string();
        let mut unused = asset("https://a.b/other.json", "{}");
        unused.destination = "json".to_string();
        let mut assets = [
            asset("https://a.b/main.js", "import './data.json';"),
            asset("https://a.b/side.js", "window.a = 1;"),
            data,
            unused,
        ];
        let imports = rewrite_scripts(&mut assets, None);
        let urls: Vec<&str> = imports.iter().map(|(url, _, _)| url.as_str()).collect();

        assert_eq!(
            urls,
            vec![
                "https://a.b/main.js",
                "https://a.b/side.js",
                "https://a.b/data.json"
            ]
        );
        assert_eq!(imports[0].1, "text/javascript");
        assert_eq!(imports[2].1, "application/json");
    }

    #[test]
    fn covers_scripts_of_workers() {
        let mut worker_script = asset("https://a.b/a.js", "import './b.js';");
        worker_script.destination = "worker".to_string();
        let mut assets = [worker_script];

        assert_eq!(rewrite_scripts(&mut assets, None).len(), 1);
        assert_eq!(assets[0].data, br#"import "https://a.b/b.js";"#.to_vec());
    }

    #[test]
    fn first_copy_of_a_module_wins() {
        let mut assets = [
            asset("https://a.b/a.js", "export const a = 1;"),
            asset("https://a.b/a.js", "export const a = 1;"),
        ];

        assert_eq!(rewrite_scripts(&mut assets, None).len(), 1);
    }

    #[test]
    fn modifies_scripts_only() {
        let mut stylesheet = asset("https://a.b/a.css", "import './b.css';");
        stylesheet.destination = "style".to_string();
        let mut assets = [stylesheet];

        assert!(rewrite_scripts(&mut assets, None).is_empty());
        assert_eq!(assets[0].data, b"import './b.css';".to_vec());
    }

    #[test]
    fn scripts_in_legacy_encodings_become_utf8() {
        let mut script = asset("https://a.b/c.js", "");
        script.content_type = "text/javascript; charset=windows-1252".to_string();
        script.data = b"document.title = '\xe9t\xe9';".to_vec();
        script.http.response_headers = vec![(
            "Content-Type".to_string(),
            "text/javascript; charset=windows-1252".to_string(),
        )];
        let mut assets = vec![script];

        rewrite_scripts(&mut assets, None);

        assert_eq!(
            String::from_utf8(assets[0].data.clone()).unwrap(),
            "document.title = '\u{e9}t\u{e9}';"
        );
        assert_eq!(assets[0].content_type, "text/javascript; charset=utf-8");
        assert_eq!(
            assets[0].http.response_headers,
            vec![(
                "Content-Type".to_string(),
                "text/javascript; charset=utf-8".to_string()
            )]
        );
    }

    #[test]
    fn byte_order_marks_beat_what_has_been_declared() {
        let mut script = asset("https://a.b/c.js", "");
        script.content_type = "text/javascript; charset=windows-1252".to_string();
        script.data = vec![0xff, 0xfe, b'x', 0, b'=', 0, b'1', 0];
        let mut assets = vec![script];

        rewrite_scripts(&mut assets, None);

        assert_eq!(assets[0].data, b"x=1");
    }

    #[test]
    fn other_parameters_of_content_types_stay() {
        let mut script = asset("https://a.b/c.js", "");
        script.content_type = "text/javascript; foo=bar; charset=windows-1252".to_string();
        let mut assets = vec![script];

        rewrite_scripts(&mut assets, None);

        assert_eq!(
            assets[0].content_type,
            "text/javascript; foo=bar; charset=utf-8"
        );
    }

    #[test]
    fn unlabeled_scripts_that_are_not_utf8_go_by_the_document() {
        let mut legacy = asset("https://a.b/c.js", "");
        legacy.content_type = "text/javascript".to_string();
        legacy.data = b"x = '\xe9';".to_vec();
        let mut utf8 = asset("https://a.b/d.js", "");
        utf8.content_type = "text/javascript".to_string();
        utf8.data = "y = '\u{e9}';".as_bytes().to_vec();
        let mut assets = vec![legacy, utf8];

        rewrite_scripts(&mut assets, Some(encoding_rs::WINDOWS_1252));

        assert_eq!(
            String::from_utf8(assets[0].data.clone()).unwrap(),
            "x = '\u{e9}';"
        );
        assert_eq!(
            String::from_utf8(assets[1].data.clone()).unwrap(),
            "y = '\u{e9}';"
        );
    }

    #[test]
    fn references_to_source_maps_go() {
        let mut assets = [asset(
            "https://a.b/a.js",
            "a();\n//# sourceMappingURL=a.js.map\n",
        )];
        rewrite_scripts(&mut assets, None);

        assert_eq!(String::from_utf8(assets[0].data.clone()).unwrap(), "a();\n");
    }
}
