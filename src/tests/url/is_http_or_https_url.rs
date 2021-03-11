//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use crate::url;

    #[test]
    fn http_url() {
        assert!(url::is_http_or_https_url(&Url::parse("http://kernel.org").unwrap()));
    }

    #[test]
    fn https_url() {
        assert!(url::is_http_or_https_url(&Url::parse("https://www.rust-lang.org/").unwrap()));
    }

    #[test]
    fn http_url_with_backslashes() {
        assert!(url::is_http_or_https_url(&Url::parse("http:\\\\freebsd.org\\").unwrap()));
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
    use reqwest::Url;

    use crate::url;

    #[test]
    fn url_with_no_protocol() {
        assert!(!url::is_http_or_https_url(&Url::parse("//kernel.org").unwrap()));
    }

    #[test]
    fn dot_slash_filename() {
        assert!(!url::is_http_or_https_url(&Url::parse("./index.html").unwrap()));
    }

    #[test]
    fn just_filename() {
        assert!(!url::is_http_or_https_url(&Url::parse("some-local-page.htm").unwrap()));
    }

    #[test]
    fn https_ip_port_url() {
        assert!(!url::is_http_or_https_url(&Url::parse("ftp://1.2.3.4/www/index.html").unwrap()));
    }

    #[test]
    fn data_url() {
        assert!(!url::is_http_or_https_url(
            &Url::parse("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h").unwrap()
        ));
    }
}
