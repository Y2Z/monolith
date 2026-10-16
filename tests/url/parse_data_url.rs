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
    fn parse_text_html_base64() {
        let (media_type, charset, data) = url::parse_data_url(&Url::parse("data:text/html;base64,V29yayBleHBhbmRzIHNvIGFzIHRvIGZpbGwgdGhlIHRpbWUgYXZhaWxhYmxlIGZvciBpdHMgY29tcGxldGlvbg==").unwrap());

        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "US-ASCII");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_html_utf8() {
        let (media_type, charset, data) = url::parse_data_url(
            &Url::parse("data:text/html;charset=utf8,Work expands so as to fill the time available for its completion").unwrap(),
        );

        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "utf8");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_html_plaintext() {
        let (media_type, charset, data) = url::parse_data_url(
            &Url::parse(
                "data:text/html,Work expands so as to fill the time available for its completion",
            )
            .unwrap(),
        );

        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "US-ASCII");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_css_url_encoded() {
        let (media_type, charset, data) =
            url::parse_data_url(&Url::parse("data:text/css,div{background-color:%23000}").unwrap());

        assert_eq!(media_type, "text/css");
        assert_eq!(charset, "US-ASCII");
        assert_eq!(String::from_utf8_lossy(&data), "div{background-color:#000}");
    }

    #[test]
    fn parse_no_media_type_base64() {
        let (media_type, charset, data) =
            url::parse_data_url(&Url::parse("data:;base64,dGVzdA==").unwrap());

        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "US-ASCII");
        assert_eq!(String::from_utf8_lossy(&data), "test");
    }

    #[test]
    fn parse_no_media_type_no_encoding() {
        let (media_type, charset, data) =
            url::parse_data_url(&Url::parse("data:;,test%20test").unwrap());

        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "US-ASCII");
        assert_eq!(String::from_utf8_lossy(&data), "test test");
    }

    #[test]
    fn parse_percent_encoded_binary_data() {
        // Bytes that aren't valid UTF-8 used to get replaced with U+FFFD, corrupting the image
        let (media_type, _charset, data) =
            url::parse_data_url(&Url::parse(url::EMPTY_IMAGE_DATA_URL).unwrap());

        assert_eq!(media_type, "image/png");
        assert!(data.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(data.ends_with(b"IEND\xAEB\x60\x82"));
        assert_eq!(data.len(), 74);
    }

    #[test]
    fn parse_base64_without_padding() {
        let (_media_type, _charset, data) =
            url::parse_data_url(&Url::parse("data:text/plain;base64,dGVzdA").unwrap());

        assert_eq!(String::from_utf8_lossy(&data), "test");
    }

    #[test]
    fn parse_base64_with_spaces() {
        let (_media_type, _charset, data) =
            url::parse_data_url(&Url::parse("data:text/plain;base64,dGVz dA==").unwrap());

        assert_eq!(String::from_utf8_lossy(&data), "test");
    }

    #[test]
    fn parse_charset_after_whitespace() {
        let (media_type, charset, _data) =
            url::parse_data_url(&Url::parse("data:Text/HTML; charset=utf-8,x").unwrap());

        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "utf-8");
    }

    #[test]
    fn parse_ignores_fragment() {
        let (_media_type, _charset, data) = url::parse_data_url(&Url::parse("data:,x#y").unwrap());

        assert_eq!(data, b"x");
    }
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use reqwest::Url;

    use monolith::url;

    #[test]
    fn empty_data_url() {
        let (media_type, charset, data) = url::parse_data_url(&Url::parse("data:,").unwrap());

        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "US-ASCII");
        assert_eq!(String::from_utf8_lossy(&data), "");
    }

    #[test]
    fn invalid_base64() {
        let (media_type, charset, data) =
            url::parse_data_url(&Url::parse("data:text/plain;base64,a").unwrap());

        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "US-ASCII");
        assert_eq!(data, b"");
    }

    #[test]
    fn no_comma() {
        let (media_type, charset, data) =
            url::parse_data_url(&Url::parse("data:image/png").unwrap());

        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "US-ASCII");
        assert_eq!(data, b"");
    }
}
