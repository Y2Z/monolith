//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::parse_content_type;

    #[test]
    fn text_plain_utf8() {
        let (media_type, charset, is_base64) = parse_content_type("text/plain;charset=utf8");
        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "utf8");
        assert!(!is_base64);
    }

    #[test]
    fn text_plain_utf8_spaces() {
        let (media_type, charset, is_base64) = parse_content_type(" text/plain ; charset=utf8 ");
        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "utf8");
        assert!(!is_base64);
    }

    #[test]
    fn empty() {
        let (media_type, charset, is_base64) = parse_content_type("");
        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "US-ASCII");
        assert!(!is_base64);
    }

    #[test]
    fn base64() {
        let (media_type, charset, is_base64) = parse_content_type(";base64");
        assert_eq!(media_type, "text/plain");
        assert_eq!(charset, "US-ASCII");
        assert!(is_base64);
    }

    #[test]
    fn text_html_base64() {
        let (media_type, charset, is_base64) = parse_content_type("text/html;base64");
        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "US-ASCII");
        assert!(is_base64);
    }

    #[test]
    fn only_media_type() {
        let (media_type, charset, is_base64) = parse_content_type("text/html");
        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "US-ASCII");
        assert!(!is_base64);
    }

    #[test]
    fn only_media_type_colon() {
        let (media_type, charset, is_base64) = parse_content_type("text/html;");
        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "US-ASCII");
        assert!(!is_base64);
    }

    #[test]
    fn media_type_gb2312_filename() {
        let (media_type, charset, is_base64) =
            parse_content_type("text/html;charset=GB2312;filename=index.html");
        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "GB2312");
        assert!(!is_base64);
    }

    #[test]
    fn media_type_filename_gb2312() {
        let (media_type, charset, is_base64) =
            parse_content_type("text/html;filename=index.html;charset=GB2312");
        assert_eq!(media_type, "text/html");
        assert_eq!(charset, "GB2312");
        assert!(!is_base64);
    }
}
