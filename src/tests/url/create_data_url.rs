//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use crate::url;

    #[test]
    fn encode_string_with_specific_media_type() {
        let media_type = "application/javascript";
        let data = "var word = 'hello';\nalert(word);\n";
        let data_url = url::create_data_url(
            media_type,
            "",
            data.as_bytes(),
            &Url::parse("data:,").unwrap(),
        );

        assert_eq!(
            data_url.as_str(),
            "data:application/javascript;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
        );
    }

    #[test]
    fn encode_append_fragment() {
        let data = "<svg></svg>\n";
        let data_url = url::create_data_url(
            "image/svg+xml",
            "",
            data.as_bytes(),
            &Url::parse("data:,").unwrap(),
        );

        assert_eq!(
            data_url.as_str(),
            "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4K"
        );
    }

    #[test]
    fn encode_string_with_specific_media_type_and_charset() {
        let media_type = "application/javascript";
        let charset = "utf8";
        let data = "var word = 'hello';\nalert(word);\n";
        let data_url = url::create_data_url(
            media_type,
            charset,
            data.as_bytes(),
            &Url::parse("data:,").unwrap(),
        );

        assert_eq!(
            data_url.as_str(),
            "data:application/javascript;charset=utf8;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
        );
    }

    #[test]
    fn create_data_url_with_us_ascii_charset() {
        let media_type = "";
        let charset = "us-ascii";
        let data = "";
        let data_url = url::create_data_url(
            media_type,
            charset,
            data.as_bytes(),
            &Url::parse("data:,").unwrap(),
        );

        assert_eq!(data_url.as_str(), "data:;base64,");
    }

    #[test]
    fn create_data_url_with_utf8_charset() {
        let media_type = "";
        let charset = "utf8";
        let data = "";
        let data_url = url::create_data_url(
            media_type,
            charset,
            data.as_bytes(),
            &Url::parse("data:,").unwrap(),
        );

        assert_eq!(data_url.as_str(), "data:;charset=utf8;base64,");
    }

    #[test]
    fn create_data_url_with_media_type_text_plain_and_utf8_charset() {
        let media_type = "text/plain";
        let charset = "utf8";
        let data = "";
        let data_url = url::create_data_url(
            media_type,
            charset,
            data.as_bytes(),
            &Url::parse("data:,").unwrap(),
        );

        assert_eq!(data_url.as_str(), "data:text/plain;charset=utf8;base64,");
    }
}
