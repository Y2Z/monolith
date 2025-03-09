//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::html;

    #[test]
    fn icon() {
        assert!(html::parse_link_type("icon").contains(&html::LinkType::Favicon));
    }

    #[test]
    fn shortcut_icon_capitalized() {
        assert!(html::parse_link_type("Shortcut Icon").contains(&html::LinkType::Favicon));
    }

    #[test]
    fn stylesheet() {
        assert!(html::parse_link_type("stylesheet").contains(&html::LinkType::Stylesheet));
    }

    #[test]
    fn preload_stylesheet() {
        assert!(html::parse_link_type("preload stylesheet").contains(&html::LinkType::Stylesheet));
    }

    #[test]
    fn apple_touch_icon() {
        assert!(html::parse_link_type("apple-touch-icon").contains(&html::LinkType::AppleTouchIcon));
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
    use monolith::html;

    #[test]
    fn mask_icon() {
        assert!(html::parse_link_type("mask-icon").is_empty());
    }

    #[test]
    fn fluid_icon() {
        assert!(html::parse_link_type("fluid-icon").is_empty());
    }

    #[test]
    fn empty_string() {
        assert!(html::parse_link_type("").is_empty());
    }
}
