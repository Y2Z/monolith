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
    fn parse_text_html_base64() {
        let (media_type, data) = url::parse_data_url(&Url::parse("data:text/html;base64,V29yayBleHBhbmRzIHNvIGFzIHRvIGZpbGwgdGhlIHRpbWUgYXZhaWxhYmxlIGZvciBpdHMgY29tcGxldGlvbg==").unwrap());

        assert_eq!(media_type, "text/html");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_html_utf8() {
        let (media_type, data) = url::parse_data_url(
            &Url::parse("data:text/html;utf8,Work expands so as to fill the time available for its completion").unwrap(),
        );

        assert_eq!(media_type, "text/html");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_html_plaintext() {
        let (media_type, data) = url::parse_data_url(
            &Url::parse(
                "data:text/html,Work expands so as to fill the time available for its completion",
            )
            .unwrap(),
        );

        assert_eq!(media_type, "text/html");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_css_url_encoded() {
        let (media_type, data) =
            url::parse_data_url(&Url::parse("data:text/css,div{background-color:%23000}").unwrap());

        assert_eq!(media_type, "text/css");
        assert_eq!(String::from_utf8_lossy(&data), "div{background-color:#000}");
    }

    #[test]
    fn parse_no_media_type_base64() {
        let (media_type, data) = url::parse_data_url(&Url::parse("data:;base64,dGVzdA==").unwrap());

        assert_eq!(media_type, "");
        assert_eq!(String::from_utf8_lossy(&data), "test");
    }

    #[test]
    fn parse_no_media_type_no_encoding() {
        let (media_type, data) = url::parse_data_url(&Url::parse("data:;,test%20test").unwrap());

        assert_eq!(media_type, "");
        assert_eq!(String::from_utf8_lossy(&data), "test test");
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

    use crate::url;

    #[test]
    fn empty_data_url() {
        let (media_type, data) = url::parse_data_url(&Url::parse("data:,").unwrap());

        assert_eq!(media_type, "");
        assert_eq!(String::from_utf8_lossy(&data), "");
    }
}
