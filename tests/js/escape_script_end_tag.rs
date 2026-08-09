//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::js;

    #[test]
    fn plain_code() {
        assert_eq!(js::escape_script_end_tag("var a = 1;"), "var a = 1;");
    }

    #[test]
    fn lowercase_end_tag() {
        assert_eq!(
            js::escape_script_end_tag(r#"s = "<script></script>";"#),
            r#"s = "<script><\/script>";"#,
        );
    }

    #[test]
    fn uppercase_end_tag() {
        assert_eq!(
            js::escape_script_end_tag(r#"s = "</SCRIPT>";"#),
            r#"s = "<\/SCRIPT>";"#,
        );
    }

    #[test]
    fn mixed_case_end_tag() {
        assert_eq!(
            js::escape_script_end_tag(r#"s = "</ScRiPt>";"#),
            r#"s = "<\/ScRiPt>";"#,
        );
    }

    #[test]
    fn end_tag_with_trailing_whitespace() {
        assert_eq!(
            js::escape_script_end_tag(
                "s = \"</script >\"; s = \"</script\t>\"; s = \"</script\n>\";"
            ),
            "s = \"<\\/script >\"; s = \"<\\/script\t>\"; s = \"<\\/script\n>\";",
        );
    }

    #[test]
    fn end_tag_with_solidus() {
        assert_eq!(
            js::escape_script_end_tag(r#"s = "</script/>";"#),
            r#"s = "<\/script/>";"#,
        );
    }

    #[test]
    fn end_tag_at_eof() {
        assert_eq!(js::escape_script_end_tag("a</script"), "a<\\/script");
    }

    #[test]
    fn multiple_end_tags() {
        assert_eq!(
            js::escape_script_end_tag("</script></SCRIPT>"),
            "<\\/script><\\/SCRIPT>",
        );
    }

    #[test]
    fn multibyte_chars_before_end_tag() {
        assert_eq!(
            js::escape_script_end_tag("let s = \"\u{2715}</script>\";"),
            "let s = \"\u{2715}<\\/script>\";",
        );
    }

    #[test]
    fn longer_tag_name_stays_untouched() {
        assert_eq!(
            js::escape_script_end_tag(r#"s = "</scripts>"; t = "</scriptx>";"#),
            r#"s = "</scripts>"; t = "</scriptx>";"#,
        );
    }

    #[test]
    fn lone_less_than_and_open_tag_stay_untouched() {
        assert_eq!(
            js::escape_script_end_tag("a < b; s = \"<script>\"; t = \"</scr\";"),
            "a < b; s = \"<script>\"; t = \"</scr\";",
        );
    }
}
