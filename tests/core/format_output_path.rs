//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use url::Url;

    use monolith::core::{MonolithOutputFormat, format_output_path};

    #[test]
    fn as_is() {
        let final_destination = format_output_path(
            "/home/username/Downloads/website.html",
            "",
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(final_destination, "/home/username/Downloads/website.html");
    }

    #[test]
    fn substitute_title() {
        let final_destination = format_output_path(
            "/home/username/Downloads/%title%.html",
            "Document Title",
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(
            final_destination,
            "/home/username/Downloads/Document Title.html"
        );
    }

    #[test]
    fn substitute_title_multi() {
        let final_destination = format_output_path(
            "/home/username/Downloads/%title%/%title%.html",
            "Document Title",
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(
            final_destination,
            "/home/username/Downloads/Document Title/Document Title.html"
        );
    }

    #[test]
    fn sanitize() {
        let final_destination = format_output_path(
            r#"/home/username/Downloads/<>:"|?/%title%.html"#,
            r#"/\<>:"|?"#,
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(
            final_destination,
            r#"/home/username/Downloads/<>:"|?/__[] - -.html"#
        );
    }

    #[test]
    fn sanitize_whitespace() {
        let final_destination = format_output_path(
            "%title%.%extension%",
            "\n      Reversing for dummies - x86 assembly and C code (Beginner/ADHD friendly)\n      \u{b7} 0x44.cc\n    ",
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(
            final_destination,
            "Reversing for dummies - x86 assembly and C code (Beginner_ADHD friendly) \u{b7} 0x44.cc.html"
        );
    }

    #[test]
    fn sanitize_control_characters() {
        let final_destination = format_output_path(
            "%title%.html",
            "Tab\there,\r\nnull\0here,\u{a0}nbsp\u{2028}separator",
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(
            final_destination,
            "Tab here, null here, nbsp separator.html"
        );
    }

    #[test]
    fn trim_spaces_and_dots() {
        let final_destination = format_output_path(
            "%title%.html",
            " . Title ... ",
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(final_destination, "Title.html");
    }

    #[test]
    fn level_up() {
        let final_destination = format_output_path(
            "../%title%.html",
            ".Title",
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(final_destination, r#"../Title.html"#);
    }

    #[test]
    fn file_name_extension() {
        let final_destination = format_output_path(
            "%title%.%extension%",
            "Title",
            None,
            MonolithOutputFormat::HTML,
        );

        assert_eq!(final_destination, r#"Title.html"#);
    }

    #[test]
    fn file_name_extension_mhtml() {
        let final_destination = format_output_path(
            "%title%.%extension%",
            "Title",
            None,
            MonolithOutputFormat::MHTML,
        );

        assert_eq!(final_destination, r#"Title.mhtml"#);
    }

    #[test]
    fn file_name_extension_short() {
        let final_destination =
            format_output_path("%title%.%ext%", "Title", None, MonolithOutputFormat::HTML);

        assert_eq!(final_destination, r#"Title.htm"#);
    }

    #[test]
    fn file_name_extension_short_mhtml() {
        let final_destination =
            format_output_path("%title%.%ext%", "Title", None, MonolithOutputFormat::MHTML);

        assert_eq!(final_destination, r#"Title.mht"#);
    }

    #[test]
    fn file_name_extension_compressed_warc() {
        for template in ["%title%.%extension%", "%title%.%ext%"] {
            assert_eq!(
                format_output_path(template, "Page", None, MonolithOutputFormat::WARCGZ),
                "Page.warc.gz"
            );
        }
    }

    #[test]
    fn nothing_left_of_title() {
        for title in ["", "  ", "...", " . "] {
            assert_eq!(
                format_output_path(
                    "%title%.%extension%",
                    title,
                    None,
                    MonolithOutputFormat::HTML
                ),
                "Untitled.html"
            );
        }
    }

    fn url(url: &str) -> Url {
        Url::parse(url).unwrap()
    }

    #[test]
    fn url_placeholder() {
        let name = |pattern: &str, address: &str| {
            format_output_path(
                pattern,
                "T",
                Some(&url(address)),
                MonolithOutputFormat::HTML,
            )
        };

        assert_eq!(
            name("%url%.%ext%", "https://github.com/Y2Z/monolith/issues/169"),
            "github.com--Y2Z--monolith--issues--169.htm"
        );
        assert_eq!(
            name("%url%.html", "https://example.com/"),
            "example.com.html"
        );
        assert_eq!(
            name("%url%.html", "https://example.com/docs/"),
            "example.com--docs.html"
        );
        assert_eq!(
            name("%url%.html", "http://localhost:8080/a?b=c&d=e/f#top"),
            "localhost_8080--a--b=c&d=e_f.html"
        );
        assert_eq!(
            name("%url%.html", "https://example.com/caf%C3%A9/a%20b"),
            "example.com--café--a b.html"
        );
        assert_eq!(
            name("%url%.html", "https://user:secret@example.com/a"),
            "example.com--a.html"
        );
        assert_eq!(name("%url%.html", "data:text/html,hi"), "data.html");
        assert_eq!(
            name("%title% (%url%).html", "https://example.com/a"),
            "T (example.com--a).html"
        );
    }

    #[test]
    fn url_placeholder_without_url() {
        assert_eq!(
            format_output_path("%url%.html", "T", None, MonolithOutputFormat::HTML),
            "Untitled.html"
        );
    }

    #[test]
    fn placeholders_in_titles_and_urls_stay() {
        assert_eq!(
            format_output_path(
                "%title% %url%.%ext%",
                "%url% and %ext%",
                Some(&url("https://example.com/%25title%25")),
                MonolithOutputFormat::HTML
            ),
            "%url% and %ext% example.com--%title%.htm"
        );
        assert_eq!(
            format_output_path("100% %title%.html", "T", None, MonolithOutputFormat::HTML),
            "100% T.html"
        );
    }

    #[test]
    fn long_names_get_cut_short() {
        let title = "é".repeat(300);
        let path = format_output_path("%title%.html", &title, None, MonolithOutputFormat::HTML);

        assert_eq!(path, format!("{}.html", "é".repeat(100)));
        let path = format_output_path(
            "%url%.html",
            "T",
            Some(&url(&format!("https://example.com/{}", "a".repeat(300)))),
            MonolithOutputFormat::HTML,
        );
        assert_eq!(path.len(), 200 + ".html".len());
    }
}
