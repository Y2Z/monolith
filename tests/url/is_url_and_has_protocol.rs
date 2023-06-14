//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::url;

    #[test]
    fn mailto() {
        assert!(url::is_url_and_has_protocol(
            "mailto:somebody@somewhere.com?subject=hello"
        ));
    }

    #[test]
    fn tel() {
        assert!(url::is_url_and_has_protocol("tel:5551234567"));
    }

    #[test]
    fn ftp_no_slashes() {
        assert!(url::is_url_and_has_protocol("ftp:some-ftp-server.com"));
    }

    #[test]
    fn ftp_with_credentials() {
        assert!(url::is_url_and_has_protocol(
            "ftp://user:password@some-ftp-server.com"
        ));
    }

    #[test]
    fn javascript() {
        assert!(url::is_url_and_has_protocol("javascript:void(0)"));
    }

    #[test]
    fn http() {
        assert!(url::is_url_and_has_protocol("http://news.ycombinator.com"));
    }

    #[test]
    fn https() {
        assert!(url::is_url_and_has_protocol("https://github.com"));
    }

    #[test]
    fn file() {
        assert!(url::is_url_and_has_protocol("file:///tmp/image.png"));
    }

    #[test]
    fn mailto_uppercase() {
        assert!(url::is_url_and_has_protocol(
            "MAILTO:somebody@somewhere.com?subject=hello"
        ));
    }

    #[test]
    fn empty_data_url() {
        assert!(url::is_url_and_has_protocol("data:text/html,"));
    }

    #[test]
    fn empty_data_url_surrounded_by_spaces() {
        assert!(url::is_url_and_has_protocol(" data:text/html, "));
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
    use monolith::url;

    #[test]
    fn url_with_no_protocol() {
        assert!(
            !url::is_url_and_has_protocol("//some-hostname.com/some-file.html")
        );
    }

    #[test]
    fn relative_path() {
        assert!(
            !url::is_url_and_has_protocol("some-hostname.com/some-file.html")
        );
    }

    #[test]
    fn relative_to_root_path() {
        assert!(!url::is_url_and_has_protocol("/some-file.html"));
    }

    #[test]
    fn empty_string() {
        assert!(!url::is_url_and_has_protocol(""));
    }
}
