//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use html5ever::serialize::{serialize, SerializeOpts};
    use markup5ever_rcdom::SerializableHandle;
    use reqwest::blocking::Client;
    use url::Url;

    use monolith::cache::Cache;
    use monolith::core::Options;
    use monolith::html;
    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn basic() {
        let cache = &mut Cache::new(0, None);

        let html: &str = "<div><P></P></div>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();

        let mut options = Options::default();
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
    fn no_css() {
        let html = "\
            <link rel=\"stylesheet\" href=\"main.css\">\
            <link rel=\"alternate stylesheet\" href=\"main.css\">\
            <style>html{background-color: #000;}</style>\
            <div style=\"display: none;\"></div>\
        ";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_css = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
    fn no_images() {
        let html = "<link rel=\"icon\" href=\"favicon.ico\">\
                    <div><img src=\"http://localhost/assets/mono_lisa.png\" /></div>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_frames = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
                    <frame src=\"\">\
                </frameset>\
            </html>\
            "
        );
    }

    #[test]
    fn no_iframes() {
        let html = "<iframe src=\"http://trackbook.com\"></iframe>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_frames = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
                    <iframe src=\"\"></iframe>\
                </body>\
            </html>\
            "
        );
    }

    #[test]
    fn no_js() {
        let html = "\
            <div onClick=\"void(0)\">\
                <script src=\"http://localhost/assets/some.js\"></script>\
                <script>alert(1)</script>\
            </div>\
        ";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_js = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_css = true;
        options.no_js = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_css = true;
        options.no_js = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_css = true;
        options.no_frames = true;
        options.no_js = true;
        options.no_images = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
    fn preserves_script_type_json() {
        let html = "<script id=\"data\" type=\"application/json\">{\"mono\":\"lith\"}</script>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut Cache::new(0, None);

        let mut options = Options::default();
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options);

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
}
