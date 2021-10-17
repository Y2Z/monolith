//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use monolith::url;

    #[test]
    fn basic_httsp_relative() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("https://www.kernel.org").unwrap(),
                "category/signatures.html"
            )
            .as_str(),
            Url::parse("https://www.kernel.org/category/signatures.html")
                .unwrap()
                .as_str()
        );
    }

    #[test]
    fn basic_httsp_absolute() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("https://www.kernel.org").unwrap(),
                "/category/signatures.html"
            )
            .as_str(),
            Url::parse("https://www.kernel.org/category/signatures.html")
                .unwrap()
                .as_str()
        );
    }

    #[test]
    fn from_https_to_level_up_relative() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("https://www.kernel.org").unwrap(),
                "../category/signatures.html"
            )
            .as_str(),
            Url::parse("https://www.kernel.org/category/signatures.html")
                .unwrap()
                .as_str()
        );
    }

    #[test]
    fn from_https_url_to_url_with_no_protocol() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("https://www.kernel.org").unwrap(),
                "//www.kernel.org/theme/images/logos/tux.png",
            )
            .as_str(),
            "https://www.kernel.org/theme/images/logos/tux.png"
        );
    }

    #[test]
    fn from_https_url_to_url_with_no_protocol_and_on_different_hostname() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("https://www.kernel.org").unwrap(),
                "//another-host.org/theme/images/logos/tux.png",
            )
            .as_str(),
            "https://another-host.org/theme/images/logos/tux.png"
        );
    }

    #[test]
    fn from_https_url_to_absolute_path() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("https://www.kernel.org/category/signatures.html").unwrap(),
                "/theme/images/logos/tux.png",
            )
            .as_str(),
            "https://www.kernel.org/theme/images/logos/tux.png"
        );
    }

    #[test]
    fn from_https_to_just_filename() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("https://www.w3schools.com/html/html_iframe.asp").unwrap(),
                "default.asp",
            )
            .as_str(),
            "https://www.w3schools.com/html/default.asp"
        );
    }

    #[test]
    fn from_data_url_to_https() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h")
                    .unwrap(),
                "https://www.kernel.org/category/signatures.html",
            )
            .as_str(),
            "https://www.kernel.org/category/signatures.html"
        );
    }

    #[test]
    fn from_data_url_to_data_url() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h")
                    .unwrap(),
                "data:text/html;base64,PGEgaHJlZj0iaW5kZXguaHRtbCI+SG9tZTwvYT4K",
            )
            .as_str(),
            "data:text/html;base64,PGEgaHJlZj0iaW5kZXguaHRtbCI+SG9tZTwvYT4K"
        );
    }

    #[test]
    fn from_file_url_to_relative_path() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("file:///home/user/Websites/my-website/index.html").unwrap(),
                "assets/images/logo.png",
            )
            .as_str(),
            "file:///home/user/Websites/my-website/assets/images/logo.png"
        );
    }

    #[test]
    fn from_file_url_to_relative_path_with_backslashes() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("file:\\\\\\home\\user\\Websites\\my-website\\index.html").unwrap(),
                "assets\\images\\logo.png",
            )
            .as_str(),
            "file:///home/user/Websites/my-website/assets/images/logo.png"
        );
    }

    #[test]
    fn from_data_url_to_file_url() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h")
                    .unwrap(),
                "file:///etc/passwd",
            )
            .as_str(),
            "file:///etc/passwd"
        );
    }

    #[test]
    fn preserve_fragment() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("http://doesnt-matter.local/").unwrap(),
                "css/fonts/fontmarvelous.svg#fontmarvelous",
            )
            .as_str(),
            "http://doesnt-matter.local/css/fonts/fontmarvelous.svg#fontmarvelous"
        );
    }

    #[test]
    fn resolve_from_file_url_to_file_url() {
        if cfg!(windows) {
            assert_eq!(
                url::resolve_url(
                    &Url::parse("file:///c:/index.html").unwrap(),
                    "file:///c:/image.png"
                )
                .as_str(),
                "file:///c:/image.png"
            );
        } else {
            assert_eq!(
                url::resolve_url(
                    &Url::parse("file:///tmp/index.html").unwrap(),
                    "file:///tmp/image.png"
                )
                .as_str(),
                "file:///tmp/image.png"
            );
        }
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

    use monolith::url;

    #[test]
    fn from_data_url_to_url_with_no_protocol() {
        assert_eq!(
            url::resolve_url(
                &Url::parse("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h")
                    .unwrap(),
                "//www.w3schools.com/html/html_iframe.asp",
            )
            .as_str(),
            "data:,"
        );
    }
}
