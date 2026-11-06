//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::replay::embed_stylesheet_urls;
    use monolith::url::Url;
    use std::collections::HashMap;

    fn embed(stylesheet: &str) -> String {
        let url = Url::parse("https://a.b/css/main.css").unwrap();
        let data_urls = HashMap::from([
            (
                "https://a.b/css/i.png".to_string(),
                "data:image/png;base64,AA==".to_string(),
            ),
            (
                "https://c.d/f.woff2".to_string(),
                "data:font/woff2;base64,BB==".to_string(),
            ),
        ]);

        embed_stylesheet_urls(stylesheet, &url, &data_urls)
    }

    #[test]
    fn relative_urls() {
        assert_eq!(
            embed("a{background:url(i.png)}"),
            "a{background:url(\"data:image/png;base64,AA==\")}"
        );
    }

    #[test]
    fn quoted_urls_without_a_protocol() {
        assert_eq!(
            embed("@font-face{src:url('//c.d/f.woff2')format('woff2')}"),
            "@font-face{src:url(\"data:font/woff2;base64,BB==\")format('woff2')}"
        );
    }

    #[test]
    fn fragments() {
        assert_eq!(
            embed("a{background:url(/css/i.png#x)}"),
            "a{background:url(\"data:image/png;base64,AA==\")}"
        );
    }

    #[test]
    fn what_was_not_captured_stays_the_same() {
        let stylesheet = "a{background:url(other.png)}b{background:url(data:,x)}";

        assert_eq!(embed(stylesheet), stylesheet);
    }

    #[test]
    fn unfinished_stylesheets() {
        assert_eq!(embed("a{background:url(i.png"), "a{background:url(i.png");
    }
}
