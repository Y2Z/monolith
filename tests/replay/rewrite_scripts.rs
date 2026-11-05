//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::CapturedAsset;
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
        let imports = rewrite_scripts(&mut assets);
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

        assert_eq!(rewrite_scripts(&mut assets).len(), 1);
        assert_eq!(assets[0].data, br#"import "https://a.b/b.js";"#.to_vec());
    }

    #[test]
    fn first_copy_of_a_module_wins() {
        let mut assets = [
            asset("https://a.b/a.js", "export const a = 1;"),
            asset("https://a.b/a.js", "export const a = 1;"),
        ];

        assert_eq!(rewrite_scripts(&mut assets).len(), 1);
    }

    #[test]
    fn modifies_scripts_only() {
        let mut stylesheet = asset("https://a.b/a.css", "import './b.css';");
        stylesheet.destination = "style".to_string();
        let mut assets = [stylesheet];

        assert!(rewrite_scripts(&mut assets).is_empty());
        assert_eq!(assets[0].data, b"import './b.css';".to_vec());
    }
}
