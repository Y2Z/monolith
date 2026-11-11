//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::MonolithOptions;
    use monolith::net::is_domain_allowed;
    use monolith::url::Url;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn options(whitelist_domains: bool) -> MonolithOptions {
        MonolithOptions {
            domains: Some(vec![".example.com".to_string()]),
            whitelist_domains,
            ..Default::default()
        }
    }

    #[test]
    fn everything_is_allowed_without_domains() {
        let options = MonolithOptions::default();

        assert!(is_domain_allowed(&url("https://example.com/"), &options));
    }

    #[test]
    fn domains_are_blacklisted_by_default() {
        let options = options(false);

        assert!(!is_domain_allowed(
            &url("https://cdn.example.com/"),
            &options
        ));
        assert!(is_domain_allowed(&url("https://other.org/"), &options));
    }

    #[test]
    fn whitelist_flips_domains() {
        let options = options(true);

        assert!(is_domain_allowed(
            &url("https://cdn.example.com/"),
            &options
        ));
        assert!(!is_domain_allowed(&url("https://other.org/"), &options));
    }
}
