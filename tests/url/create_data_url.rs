//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use monolith::url;

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

    #[test]
    fn create_data_url_round_trips_binary_data() {
        let data: Vec<u8> = (0..=255).collect();
        let data_url = url::create_data_url(
            "application/octet-stream",
            "",
            &data,
            &Url::parse("data:,").unwrap(),
        );
        let (media_type, _charset, parsed_data) = url::parse_data_url(&data_url);

        assert_eq!(media_type, "application/octet-stream");
        assert_eq!(parsed_data, data);
    }

    #[test]
    fn create_data_url_with_lowercased_media_type() {
        let data_url = url::create_data_url("Image/PNG", "", b"", &Url::parse("data:,").unwrap());

        assert_eq!(data_url.as_str(), "data:image/png;base64,");
    }

    #[test]
    fn create_data_url_with_quoted_charset() {
        let data_url =
            url::create_data_url("text/css", "\"utf-8\"", b"", &Url::parse("data:,").unwrap());

        assert_eq!(data_url.as_str(), "data:text/css;charset=utf-8;base64,");
    }

    #[test]
    fn create_data_url_omits_invalid_media_type() {
        let data_url = url::create_data_url("wrong", "", b"", &Url::parse("data:,").unwrap());

        assert_eq!(data_url.as_str(), "data:;base64,");
    }

    #[test]
    fn create_data_url_omits_charset_that_would_break_data_url() {
        let data_url =
            url::create_data_url("text/css", "utf-8,x", b"", &Url::parse("data:,").unwrap());

        assert_eq!(data_url.as_str(), "data:text/css;base64,");
    }
}
