//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::is_request_allowed;
    use monolith::core::MonolithOptions;

    #[test]
    fn what_was_not_excluded() {
        let mut options = MonolithOptions::default();
        options.domains = Some(vec!["ads.example.com".to_string()]);
        options.no_images = true;

        assert!(is_request_allowed("https://example.com/", "", &options));
        assert!(is_request_allowed(
            "https://example.com/a.woff",
            "font",
            &options
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
    use monolith::browser::is_request_allowed;
    use monolith::core::MonolithOptions;

    #[test]
    fn excluded_kind_of_asset() {
        let mut options = MonolithOptions::default();
        options.no_images = true;

        assert!(!is_request_allowed(
            "https://example.com/a.png",
            "image",
            &options
        ));
    }

    #[test]
    fn domain_not_allowed() {
        let mut options = MonolithOptions::default();
        options.domains = Some(vec!["ads.example.com".to_string()]);

        assert!(!is_request_allowed(
            "https://ads.example.com/a.js",
            "script",
            &options
        ));
    }
}
