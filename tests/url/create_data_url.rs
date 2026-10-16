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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
        );

        assert_eq!(data_url.as_str(), "data:text/plain;charset=utf8;base64,");
    }

    #[test]
    fn create_data_url_round_trips_binary_data() {
        let data: Vec<u8> = (0..=255).collect();
        let data_url = url::create_data_url(
            "application/octet-stream",
            "",
            data.clone(),
            &Url::parse("data:,").unwrap(),
            None,
        );
        let (media_type, _charset, parsed_data) =
            url::parse_data_url(&Url::parse(&data_url).unwrap());

        assert_eq!(media_type, "application/octet-stream");
        assert_eq!(parsed_data, data);
    }

    #[test]
    fn create_data_url_with_lowercased_media_type() {
        let data_url =
            url::create_data_url("Image/PNG", "", b"", &Url::parse("data:,").unwrap(), None);

        assert_eq!(data_url.as_str(), "data:image/png;base64,");
    }

    #[test]
    fn create_data_url_with_quoted_charset() {
        let data_url = url::create_data_url(
            "text/css",
            "\"utf-8\"",
            b"",
            &Url::parse("data:,").unwrap(),
            None,
        );

        assert_eq!(data_url.as_str(), "data:text/css;charset=utf-8;base64,");
    }

    #[test]
    fn create_data_url_omits_invalid_media_type() {
        let data_url = url::create_data_url("wrong", "", b"", &Url::parse("data:,").unwrap(), None);

        assert_eq!(data_url.as_str(), "data:;base64,");
    }

    #[test]
    fn create_data_url_omits_charset_that_would_break_data_url() {
        let data_url = url::create_data_url(
            "text/css",
            "utf-8,x",
            b"",
            &Url::parse("data:,").unwrap(),
            None,
        );

        assert_eq!(data_url.as_str(), "data:text/css;base64,");
    }

    #[test]
    fn create_data_url_with_fragment() {
        let asset_url = Url::parse("https://example.com/icons.svg#a%20b").unwrap();
        let data_url = url::create_data_url(
            "image/svg+xml",
            "",
            b"<svg/>",
            &asset_url,
            asset_url.fragment(),
        );

        assert_eq!(data_url, "data:image/svg+xml;base64,PHN2Zy8+#a%20b");
    }

    #[test]
    fn create_data_url_encodes_fragment_like_url_does() {
        for fragment in ["", "a b", "<\"`>", "caf\u{e9}", "%41%", "x#y"] {
            let data_url = url::create_data_url(
                "text/plain",
                "",
                b"x",
                &Url::parse("data:,").unwrap(),
                Some(fragment),
            );
            let mut expected = Url::parse("data:text/plain;base64,eA==").unwrap();
            expected.set_fragment(Some(fragment));

            assert_eq!(data_url, expected.as_str(), "{fragment:?}");
        }
    }
}
