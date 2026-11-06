//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::destination_of;

    #[test]
    fn documents_depend_on_the_frame() {
        assert_eq!(destination_of("Document", true), "document");
        assert_eq!(destination_of("Document", false), "iframe");
    }

    #[test]
    fn assets() {
        assert_eq!(destination_of("Script", true), "script");
        assert_eq!(destination_of("Stylesheet", true), "style");
        assert_eq!(destination_of("Image", true), "image");
        assert_eq!(destination_of("Font", true), "font");
    }

    #[test]
    fn what_scripts_ask_for() {
        assert_eq!(destination_of("Fetch", true), "");
        assert_eq!(destination_of("XHR", true), "");
    }
}
