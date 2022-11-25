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
    fn never_expires() {
        let cookie = cookies::Cookie {
            domain: String::from("127.0.0.1"),
            include_subdomains: true,
            path: String::from("/"),
            https_only: false,
            expires: 0,
            name: String::from(""),
            value: String::from(""),
        };

        assert!(!cookie.is_expired());
    }

    #[test]
    fn expires_long_from_now() {
        let cookie = cookies::Cookie {
            domain: String::from("127.0.0.1"),
            include_subdomains: true,
            path: String::from("/"),
            https_only: false,
            expires: 9999999999,
            name: String::from(""),
            value: String::from(""),
        };

        assert!(!cookie.is_expired());
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
    fn expired() {
        let cookie = cookies::Cookie {
            domain: String::from("127.0.0.1"),
            include_subdomains: true,
            path: String::from("/"),
            https_only: false,
            expires: 1,
            name: String::from(""),
            value: String::from(""),
        };

        assert!(cookie.is_expired());
    }
}
