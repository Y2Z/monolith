//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::url;

    #[test]
    fn parse_text_html_base64() {
        let (media_type, data) = url::data_url_to_data("data:text/html;base64,V29yayBleHBhbmRzIHNvIGFzIHRvIGZpbGwgdGhlIHRpbWUgYXZhaWxhYmxlIGZvciBpdHMgY29tcGxldGlvbg==");

        assert_eq!(media_type, "text/html");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_html_utf8() {
        let (media_type, data) = url::data_url_to_data(
            "data:text/html;utf8,Work expands so as to fill the time available for its completion",
        );

        assert_eq!(media_type, "text/html");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_html_plaintext() {
        let (media_type, data) = url::data_url_to_data(
            "data:text/html,Work expands so as to fill the time available for its completion",
        );

        assert_eq!(media_type, "text/html");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_html_charset_utf_8_between_two_whitespaces() {
        let (media_type, data) = url::data_url_to_data(" data:text/html;charset=utf-8,Work expands so as to fill the time available for its completion ");

        assert_eq!(media_type, "text/html");
        assert_eq!(
            String::from_utf8_lossy(&data),
            "Work expands so as to fill the time available for its completion"
        );
    }

    #[test]
    fn parse_text_css_url_encoded() {
        let (media_type, data) =
            url::data_url_to_data("data:text/css,div{background-color:%23000}");

        assert_eq!(media_type, "text/css");
        assert_eq!(String::from_utf8_lossy(&data), "div{background-color:#000}");
    }

    #[test]
    fn parse_no_media_type_base64() {
        let (media_type, data) = url::data_url_to_data("data:;base64,dGVzdA==");

        assert_eq!(media_type, "");
        assert_eq!(String::from_utf8_lossy(&data), "test");
    }

    #[test]
    fn parse_no_media_type_no_encoding() {
        let (media_type, data) = url::data_url_to_data("data:;,test%20test");

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
    use crate::url;

    #[test]
    fn just_word_data() {
        let (media_type, data) = url::data_url_to_data("data");

        assert_eq!(media_type, "");
        assert_eq!(String::from_utf8_lossy(&data), "");
    }
}
