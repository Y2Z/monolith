//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::{ColorScheme, parse_macos_color_scheme};

    #[test]
    fn dark() {
        assert_eq!(parse_macos_color_scheme("Dark\n"), ColorScheme::Dark);
    }

    #[test]
    fn light_has_no_value() {
        assert_eq!(parse_macos_color_scheme(""), ColorScheme::Light);
    }
}
