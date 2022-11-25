//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::cookies;

    #[test]
    fn secure_url() {
        let cookie = cookies::Cookie {
            domain: String::from("127.0.0.1"),
            include_subdomains: true,
            path: String::from("/"),
            https_only: true,
            expires: 0,
            name: String::from(""),
            value: String::from(""),
        };
        assert!(cookie.matches_url("https://127.0.0.1/something"));
    }

    #[test]
    fn non_secure_url() {
        let cookie = cookies::Cookie {
            domain: String::from("127.0.0.1"),
            include_subdomains: true,
            path: String::from("/"),
            https_only: false,
            expires: 0,
            name: String::from(""),
            value: String::from(""),
        };
        assert!(cookie.matches_url("http://127.0.0.1/something"));
    }

    #[test]
    fn subdomain() {
        let cookie = cookies::Cookie {
            domain: String::from(".somethingsomething.com"),
            include_subdomains: true,
            path: String::from("/"),
            https_only: true,
            expires: 0,
            name: String::from(""),
            value: String::from(""),
        };
        assert!(cookie.matches_url("https://cdn.somethingsomething.com/something"));
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
    use monolith::cookies;

    #[test]
    fn empty_url() {
        let cookie = cookies::Cookie {
            domain: String::from("127.0.0.1"),
            include_subdomains: true,
            path: String::from("/"),
            https_only: false,
            expires: 0,
            name: String::from(""),
            value: String::from(""),
        };
        assert!(!cookie.matches_url(""));
    }

    #[test]
    fn wrong_hostname() {
        let cookie = cookies::Cookie {
            domain: String::from("127.0.0.1"),
            include_subdomains: true,
            path: String::from("/"),
            https_only: false,
            expires: 0,
            name: String::from(""),
            value: String::from(""),
        };
        assert!(!cookie.matches_url("http://0.0.0.0/"));
    }

    #[test]
    fn wrong_path() {
        let cookie = cookies::Cookie {
            domain: String::from("127.0.0.1"),
            include_subdomains: false,
            path: String::from("/"),
            https_only: false,
            expires: 0,
            name: String::from(""),
            value: String::from(""),
        };
        assert!(!cookie.matches_url("http://0.0.0.0/path"));
    }
}
