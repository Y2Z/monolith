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
    fn parse_file() {
        let file_contents =
            "# Netscape HTTP Cookie File\n127.0.0.1\tFALSE\t/\tFALSE\t0\tUSER_TOKEN\tin";
        let result = cookies::parse_cookie_file_contents(&file_contents).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].domain, "127.0.0.1");
        assert_eq!(result[0].include_subdomains, false);
        assert_eq!(result[0].path, "/");
        assert_eq!(result[0].https_only, false);
        assert_eq!(result[0].expires, 0);
        assert_eq!(result[0].name, "USER_TOKEN");
        assert_eq!(result[0].value, "in");
    }

    #[test]
    fn parse_multiline_file() {
        let file_contents = "# HTTP Cookie File\n127.0.0.1\tFALSE\t/\tFALSE\t0\tUSER_TOKEN\tin\n127.0.0.1\tTRUE\t/\tTRUE\t9\tUSER_TOKEN\tout\n\n";
        let result = cookies::parse_cookie_file_contents(&file_contents).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].domain, "127.0.0.1");
        assert_eq!(result[0].include_subdomains, false);
        assert_eq!(result[0].path, "/");
        assert_eq!(result[0].https_only, false);
        assert_eq!(result[0].expires, 0);
        assert_eq!(result[0].name, "USER_TOKEN");
        assert_eq!(result[0].value, "in");
        assert_eq!(result[1].domain, "127.0.0.1");
        assert_eq!(result[1].include_subdomains, true);
        assert_eq!(result[1].path, "/");
        assert_eq!(result[1].https_only, true);
        assert_eq!(result[1].expires, 9);
        assert_eq!(result[1].name, "USER_TOKEN");
        assert_eq!(result[1].value, "out");
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
    fn empty() {
        let file_contents = "";
        let result = cookies::parse_cookie_file_contents(&file_contents).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn no_header() {
        let file_contents = "127.0.0.1   FALSE   /   FALSE   0   USER_TOKEN  in";
        match cookies::parse_cookie_file_contents(&file_contents) {
            Ok(_result) => {
                assert!(false);
            }
            Err(_e) => {
                assert!(true);
            }
        }
    }

    #[test]
    fn spaces_instead_of_tabs() {
        let file_contents =
            "# HTTP Cookie File\n127.0.0.1   FALSE   /   FALSE   0   USER_TOKEN  in";
        let result = cookies::parse_cookie_file_contents(&file_contents).unwrap();
        assert_eq!(result.len(), 0);
    }
}
