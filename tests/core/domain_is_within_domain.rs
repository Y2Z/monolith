//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::domain_is_within_domain;

    #[test]
    fn sub_domain_is_within_dotted_sub_domain() {
        assert!(domain_is_within_domain(
            "news.ycombinator.com",
            ".news.ycombinator.com"
        ));
    }

    #[test]
    fn domain_is_within_dotted_domain() {
        assert!(domain_is_within_domain(
            "ycombinator.com",
            ".ycombinator.com"
        ));
    }

    #[test]
    fn sub_domain_is_within_dotted_domain() {
        assert!(domain_is_within_domain(
            "news.ycombinator.com",
            ".ycombinator.com"
        ));
    }

    #[test]
    fn sub_domain_is_within_dotted_top_level_domain() {
        assert!(domain_is_within_domain("news.ycombinator.com", ".com"));
    }

    #[test]
    fn domain_is_within_itself() {
        assert!(domain_is_within_domain(
            "ycombinator.com",
            "ycombinator.com"
        ));
    }

    #[test]
    fn domain_with_trailing_dot_is_within_itself() {
        assert!(domain_is_within_domain(
            "ycombinator.com.",
            "ycombinator.com"
        ));
    }

    #[test]
    fn domain_with_trailing_dot_is_within_single_dot() {
        assert!(domain_is_within_domain("ycombinator.com.", "."));
    }

    #[test]
    fn domain_matches_single_dot() {
        assert!(domain_is_within_domain("ycombinator.com", "."));
    }

    #[test]
    fn dotted_domain_must_be_within_dotted_domain() {
        assert!(domain_is_within_domain(
            ".ycombinator.com",
            ".ycombinator.com"
        ));
    }

    #[test]
    fn empty_is_within_dot() {
        assert!(domain_is_within_domain("", "."));
    }

    #[test]
    fn both_dots() {
        assert!(domain_is_within_domain(".", "."));
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
    use monolith::core::domain_is_within_domain;

    #[test]
    fn sub_domain_must_not_be_within_domain() {
        assert!(!domain_is_within_domain(
            "news.ycombinator.com",
            "ycombinator.com"
        ));
    }

    #[test]
    fn domain_must_not_be_within_top_level_domain() {
        assert!(!domain_is_within_domain("ycombinator.com", "com"));
    }

    #[test]
    fn different_domains_must_not_be_within_one_another() {
        assert!(!domain_is_within_domain(
            "news.ycombinator.com",
            "kernel.org"
        ));
    }

    #[test]
    fn sub_domain_is_not_within_wrong_top_level_domain() {
        assert!(!domain_is_within_domain("news.ycombinator.com", "org"));
    }

    #[test]
    fn dotted_domain_is_not_within_domain() {
        assert!(!domain_is_within_domain(
            ".ycombinator.com",
            "ycombinator.com"
        ));
    }

    #[test]
    fn different_domain_is_not_within_dotted_domain() {
        assert!(!domain_is_within_domain(
            "www.doodleoptimize.com",
            ".ycombinator.com"
        ));
    }

    #[test]
    fn no_domain_can_be_within_empty_domain() {
        assert!(!domain_is_within_domain("ycombinator.com", ""));
    }

    #[test]
    fn both_can_not_be_empty() {
        assert!(!domain_is_within_domain("", ""));
    }
}
