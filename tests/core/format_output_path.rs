//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::format_output_path;

    #[test]
    fn as_is() {
        let final_destination = format_output_path("/home/username/Downloads/website.html", "");

        assert_eq!(final_destination, "/home/username/Downloads/website.html");
    }

    #[test]
    fn substitute_title() {
        let final_destination =
            format_output_path("/home/username/Downloads/%title%.html", "Document Title");

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
        );

        assert_eq!(
            final_destination,
            r#"/home/username/Downloads/[] - -/__[] - -.html"#
        );
    }
}
