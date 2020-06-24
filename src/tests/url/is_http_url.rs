//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::url;

    #[test]
    fn http_url() {
        assert!(url::is_http_url("http://kernel.org"));
    }

    #[test]
    fn https_url() {
        assert!(url::is_http_url("https://www.rust-lang.org/"));
    }

    #[test]
    fn http_url_with_backslashes() {
        assert!(url::is_http_url("http:\\\\freebsd.org\\"));
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
    use crate::url;

    #[test]
    fn url_with_no_protocol() {
        assert!(!url::is_http_url("//kernel.org"));
    }

    #[test]
    fn dot_slash_filename() {
        assert!(!url::is_http_url("./index.html"));
    }

    #[test]
    fn just_filename() {
        assert!(!url::is_http_url("some-local-page.htm"));
    }

    #[test]
    fn https_ip_port_url() {
        assert!(!url::is_http_url("ftp://1.2.3.4/www/index.html"));
    }

    #[test]
    fn data_url() {
        assert!(!url::is_http_url(
            "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
        ));
    }
}
