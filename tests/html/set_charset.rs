//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::MonolithOptions;
    use monolith::html;

    fn set_charset_and_serialize(html: &str, charset: &str) -> String {
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let dom = html::set_charset(dom, charset.to_string());
        let options = MonolithOptions::default();

        String::from_utf8_lossy(&html::serialize_document(dom, "".to_string(), &options))
            .to_string()
    }

    #[test]
    fn replace_meta_charset_in_place() {
        assert_eq!(
            set_charset_and_serialize(
                "<html><head>\n    <meta charset=\"Shift_JIS\">\n    <title>x</title></head><body></body></html>",
                "UTF-8"
            ),
            "<html><head>\n    <meta charset=\"UTF-8\">\n    <title>x</title></head><body></body></html>"
        );
    }

    #[test]
    fn replace_meta_content_type_in_place() {
        assert_eq!(
            set_charset_and_serialize(
                "<html><head><meta http-equiv=\"content-type\" content=\"text/html; charset=Shift_JIS\"><title>x</title></head><body></body></html>",
                "UTF-8"
            ),
            "<html><head><meta http-equiv=\"content-type\" content=\"text/html;charset=UTF-8\"><title>x</title></head><body></body></html>"
        );
    }

    #[test]
    fn add_meta_charset_as_first_element() {
        assert_eq!(
            set_charset_and_serialize(
                "<html><head><title>x</title></head><body></body></html>",
                "UTF-8"
            ),
            "<html><head><meta charset=\"UTF-8\"><title>x</title></head><body></body></html>"
        );
    }

    #[test]
    fn move_meta_charset_to_be_first_element() {
        assert_eq!(
            set_charset_and_serialize(
                "<html><head><title>x</title><link rel=\"stylesheet\" href=\"style.css\"><meta charset=\"Shift_JIS\"></head><body></body></html>",
                "UTF-8"
            ),
            "<html><head><meta charset=\"UTF-8\"><title>x</title><link rel=\"stylesheet\" href=\"style.css\"></head><body></body></html>"
        );
    }

    #[test]
    fn meta_charset_stays_within_first_1024_bytes() {
        // Embedded assets can make HEAD huge, while browsers only look for the charset within the first 1024 bytes
        let html = format!(
            "<html><head><style>{}</style><meta charset=\"Shift_JIS\"></head><body></body></html>",
            "p{color:red}".repeat(200)
        );
        let output = set_charset_and_serialize(&html, "UTF-8");

        assert!(output.find("<meta charset=\"UTF-8\">").unwrap() < 1024);
        assert_eq!(output.matches("<meta charset=").count(), 1);
    }

    #[test]
    fn ignore_meta_content_type_without_charset() {
        // A content type without a charset doesn't declare any encoding, so it's left alone
        assert_eq!(
            set_charset_and_serialize(
                "<html><head><meta http-equiv=\"content-type\" content=\"text/html\"></head><body></body></html>",
                "UTF-8"
            ),
            "<html><head><meta charset=\"UTF-8\"><meta http-equiv=\"content-type\" content=\"text/html\"></head><body></body></html>"
        );
    }

    #[test]
    fn get_charset_returns_what_was_set() {
        let html = "<html><head><title>x</title><meta http-equiv=\"content-type\" content=\"text/html\"><meta charset=\"Shift_JIS\"></head><body></body></html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let dom = html::set_charset(dom, "UTF-8".to_string());

        assert_eq!(html::get_charset(&dom.document), Some("UTF-8".to_string()));
    }
}
