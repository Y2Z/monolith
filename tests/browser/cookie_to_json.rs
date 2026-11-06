//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::cookie_to_json;
    use monolith::cookies::Cookie;

    fn cookie() -> Cookie {
        Cookie {
            domain: ".example.com".to_string(),
            include_subdomains: true,
            path: "/".to_string(),
            https_only: true,
            expires: 0,
            name: "a".to_string(),
            value: "b".to_string(),
        }
    }

    #[test]
    fn session_cookie_for_subdomains() {
        let description = cookie_to_json(&cookie());

        assert_eq!(description["domain"], ".example.com");
        assert_eq!(description["value"]["value"], "b");
        assert!(description.get("expiry").is_none());
    }

    #[test]
    fn expiring_cookie_for_single_host() {
        let mut cookie = cookie();
        cookie.include_subdomains = false;
        cookie.expires = 7;
        let description = cookie_to_json(&cookie);

        assert_eq!(description["domain"], "example.com");
        assert_eq!(description["expiry"], 7);
    }
}
