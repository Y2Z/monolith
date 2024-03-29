//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::css;

    #[test]
    fn background() {
        assert!(css::is_image_url_prop("background"));
    }

    #[test]
    fn background_image() {
        assert!(css::is_image_url_prop("background-image"));
    }

    #[test]
    fn background_image_uppercase() {
        assert!(css::is_image_url_prop("BACKGROUND-IMAGE"));
    }

    #[test]
    fn border_image() {
        assert!(css::is_image_url_prop("border-image"));
    }

    #[test]
    fn content() {
        assert!(css::is_image_url_prop("content"));
    }

    #[test]
    fn cursor() {
        assert!(css::is_image_url_prop("cursor"));
    }

    #[test]
    fn list_style() {
        assert!(css::is_image_url_prop("list-style"));
    }

    #[test]
    fn list_style_image() {
        assert!(css::is_image_url_prop("list-style-image"));
    }

    #[test]
    fn mask_image() {
        assert!(css::is_image_url_prop("mask-image"));
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
    use monolith::css;

    #[test]
    fn empty() {
        assert!(!css::is_image_url_prop(""));
    }

    #[test]
    fn width() {
        assert!(!css::is_image_url_prop("width"));
    }

    #[test]
    fn color() {
        assert!(!css::is_image_url_prop("color"));
    }

    #[test]
    fn z_index() {
        assert!(!css::is_image_url_prop("z-index"));
    }
}
