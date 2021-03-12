//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use html5ever::serialize::{serialize, SerializeOpts};
    use reqwest::blocking::Client;
    use std::collections::HashMap;
    use url::Url;

    use crate::html;
    use crate::opts::Options;

    #[test]
    fn basic() {
        let cache = &mut HashMap::new();

        let html: &str = "<div><P></P></div>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();

        let mut options = Options::default();
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><div><p></p></div></body></html>"
        );
    }

    #[test]
    fn ensure_no_recursive_iframe() {
        let html = "<div><P></P><iframe src=\"\"></iframe></div>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><div><p></p><iframe src=\"\"></iframe></div></body></html>"
        );
    }

    #[test]
    fn ensure_no_recursive_frame() {
        let html = "<frameset><frame src=\"\"></frameset>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><frameset><frame src=\"\"></frameset></html>"
        );
    }

    #[test]
    fn no_css() {
        let html = "<link rel=\"stylesheet\" href=\"main.css\">\
                    <link rel=\"alternate stylesheet\" href=\"main.css\">\
                    <style>html{background-color: #000;}</style>\
                    <div style=\"display: none;\"></div>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.no_css = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html>\
            <head>\
            <link rel=\"stylesheet\">\
            <link rel=\"alternate stylesheet\">\
            <style></style>\
            </head>\
            <body>\
            <div></div>\
            </body>\
            </html>"
        );
    }

    #[test]
    fn no_images() {
        let html = "<link rel=\"icon\" href=\"favicon.ico\">\
                    <div><img src=\"http://localhost/assets/mono_lisa.png\" /></div>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

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
                empty_image = empty_image!()
            )
        );
    }

    #[test]
    fn no_body_background_images() {
        let html =
            "<body background=\"no/such/image.png\" background=\"no/such/image2.png\"></body>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body></body></html>"
        );
    }

    #[test]
    fn no_frames() {
        let html = "<frameset><frame src=\"http://trackbook.com\"></frameset>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.no_frames = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><frameset><frame src=\"\"></frameset></html>"
        );
    }

    #[test]
    fn no_iframes() {
        let html = "<iframe src=\"http://trackbook.com\"></iframe>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.no_frames = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><iframe src=\"\"></iframe></body></html>"
        );
    }

    #[test]
    fn no_js() {
        let html = "<div onClick=\"void(0)\">\
                        <script src=\"http://localhost/assets/some.js\"></script>\
                        <script>alert(1)</script>\
                    </div>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.no_js = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><div><script></script>\
            <script></script></div></body></html>"
        );
    }

    // #[test]
    // fn discards_integrity() {
    //     let html = "<title>No integrity</title>\
    //                 <link integrity=\"sha384-...\" rel=\"something\"/>\
    //                 <script integrity=\"sha384-...\" src=\"some.js\"></script>";
    //     let dom = html::html_to_dom(&html);
    //     let url: Url = Url::parse("http://localhost").unwrap();
    //     let cache = &mut HashMap::new();

    //     let mut options = Options::default();
    //     options.no_css = true;
    //     options.no_frames = true;
    //     options.no_js = true;
    //     options.no_images = true;
    //     options.silent = true;

    //     let client = Client::new();

    //     html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

    //     let mut buf: Vec<u8> = Vec::new();
    //     serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

    //     assert_eq!(
    //         buf.iter().map(|&c| c as char).collect::<String>(),
    //         "<html>\
    //             <head><title>No integrity</title><link rel=\"something\"><script></script></head>\
    //             <body></body>\
    //         </html>"
    //     );
    // }

    #[test]
    fn removes_unwanted_meta_tags() {
        let html = "<html>\
            <head>\
                <meta http-equiv=\"Refresh\" value=\"20\"/>\
                <meta http-equiv=\"Location\" value=\"https://freebsd.org\"/>\
            </head>\
            <body></body>\
        </html>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.no_css = true;
        options.no_frames = true;
        options.no_js = true;
        options.no_images = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html>\
                <head>\
                    <meta http-equiv=\"disabled by monolith (Refresh)\" value=\"20\">\
                    <meta http-equiv=\"disabled by monolith (Location)\" value=\"https://freebsd.org\">\
                </head>\
                <body></body>\
            </html>"
        );
    }

    #[test]
    fn processes_noscript_tags() {
        let html = "<html>\
            <body>\
                <noscript>\
                    <img src=\"image.png\" />\
                </noscript>\
            </body>\
        </html>";
        let dom = html::html_to_dom(&html);
        let url: Url = Url::parse("http://localhost").unwrap();
        let cache = &mut HashMap::new();

        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;

        let client = Client::new();

        html::walk_and_embed_assets(cache, &client, &url, &dom.document, &options, 0);

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            format!(
                "<html>\
                    <head>\
                    </head>\
                    <body>\
                        <noscript>\
                            <img src=\"{}\">\
                        </noscript>\
                    </body>\
                </html>",
                empty_image!(),
            )
        );
    }
}
