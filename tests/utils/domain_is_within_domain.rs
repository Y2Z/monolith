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
    fn sub_domain_is_within_domain() {
        assert!(utils::domain_is_within_domain(
            "news.ycombinator.com",
            "ycombinator.com"
        ));
    }

    #[test]
    fn sub_domain_is_within_dotted_domain() {
        assert!(utils::domain_is_within_domain(
            "news.ycombinator.com",
            ".ycombinator.com"
        ));
    }

    #[test]
    fn domain_is_within_top_level_domain() {
        assert!(utils::domain_is_within_domain("ycombinator.com", "com"));
    }

    #[test]
    fn domain_is_within_itself() {
        assert!(utils::domain_is_within_domain(
            "ycombinator.com",
            "ycombinator.com"
        ));
    }

    #[test]
    fn sub_domain_is_within_dotted_itself() {
        assert!(utils::domain_is_within_domain(
            "ycombinator.com",
            ".ycombinator.com"
        ));
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
    use monolith::utils;

    #[test]
    fn sub_domain_is_not_within_domain() {
        assert!(!utils::domain_is_within_domain(
            "news.ycombinator.com",
            "kernel.org"
        ));
    }

    #[test]
    fn sub_domain_is_not_within_top_level_domain() {
        assert!(!utils::domain_is_within_domain(
            "news.ycombinator.com",
            "org"
        ));
    }

    #[test]
    fn no_domain_is_not_within_dot() {
        assert!(!utils::domain_is_within_domain("news.ycombinator.com", "."));
    }

    #[test]
    fn no_domain_is_within_empty_domain() {
        assert!(!utils::domain_is_within_domain("news.ycombinator.com", ""));
    }
}
