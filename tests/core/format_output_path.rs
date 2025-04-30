//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::{format_output_path, MonolithOutputFormat};

    #[test]
    fn as_is() {
        let final_destination = format_output_path(
            "/home/username/Downloads/website.html",
            "",
            MonolithOutputFormat::HTML,
        );

        assert_eq!(final_destination, "/home/username/Downloads/website.html");
    }

    #[test]
    fn substitute_title() {
        let final_destination = format_output_path(
            "/home/username/Downloads/%title%.html",
            "Document Title",
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
            MonolithOutputFormat::HTML,
        );

        assert_eq!(
            final_destination,
            r#"/home/username/Downloads/<>:"|?/__[] - -.html"#
        );
    }

    #[test]
    fn level_up() {
        let final_destination =
            format_output_path("../%title%.html", ".Title", MonolithOutputFormat::HTML);

        assert_eq!(final_destination, r#"../Title.html"#);
    }

    #[test]
    fn file_name_extension() {
        let final_destination =
            format_output_path("%title%.%extension%", "Title", MonolithOutputFormat::HTML);

        assert_eq!(final_destination, r#"Title.html"#);
    }

    #[test]
    fn file_name_extension_mhtml() {
        let final_destination =
            format_output_path("%title%.%extension%", "Title", MonolithOutputFormat::MHTML);

        assert_eq!(final_destination, r#"Title.mhtml"#);
    }

    #[test]
    fn file_name_extension_short() {
        let final_destination =
            format_output_path("%title%.%ext%", "Title", MonolithOutputFormat::HTML);

        assert_eq!(final_destination, r#"Title.htm"#);
    }

    #[test]
    fn file_name_extension_short_mhtml() {
        let final_destination =
            format_output_path("%title%.%ext%", "Title", MonolithOutputFormat::MHTML);

        assert_eq!(final_destination, r#"Title.mht"#);
    }
}
