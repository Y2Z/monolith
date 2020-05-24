//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::utils;

    #[test]
    fn http_url() {
        assert!(utils::is_http_url("http://kernel.org"));
    }

    #[test]
    fn https_url() {
        assert!(utils::is_http_url("https://www.rust-lang.org/"));
    }

    #[test]
    fn http_url_with_backslashes() {
        assert!(utils::is_http_url("http:\\\\freebsd.org\\"));
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
    use crate::utils;

    #[test]
    fn url_with_no_protocol() {
        assert!(!utils::is_http_url("//kernel.org"));
    }

    #[test]
    fn dot_slash_filename() {
        assert!(!utils::is_http_url("./index.html"));
    }

    #[test]
    fn just_filename() {
        assert!(!utils::is_http_url("some-local-page.htm"));
    }

    #[test]
    fn https_ip_port_url() {
        assert!(!utils::is_http_url("ftp://1.2.3.4/www/index.html"));
    }

    #[test]
    fn data_url() {
        assert!(!utils::is_http_url(
            "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
        ));
    }
}
