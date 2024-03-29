//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::utils;

    #[test]
    fn zero() {
        assert_eq!(utils::indent(0), "");
    }

    #[test]
    fn one() {
        assert_eq!(utils::indent(1), " ");
    }

    #[test]
    fn two() {
        assert_eq!(utils::indent(2), "  ");
    }

    #[test]
    fn three() {
        assert_eq!(utils::indent(3), "   ");
    }

    #[test]
    fn four() {
        assert_eq!(utils::indent(4), "    ");
    }
}
