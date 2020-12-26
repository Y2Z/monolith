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
    fn from_https_to_level_up_relative() {
        assert_eq!(
            url::resolve_url("https://www.kernel.org", "../category/signatures.html")
                .unwrap_or_default(),
            "https://www.kernel.org/category/signatures.html"
        );
    }

    #[test]
    fn from_just_filename_to_full_https_url() {
        assert_eq!(
            url::resolve_url(
                "saved_page.htm",
                "https://www.kernel.org/category/signatures.html",
            )
            .unwrap_or_default(),
            "https://www.kernel.org/category/signatures.html"
        );
    }

    #[test]
    fn from_https_url_to_url_with_no_protocol() {
        assert_eq!(
            url::resolve_url(
                "https://www.kernel.org",
                "//www.kernel.org/theme/images/logos/tux.png",
            )
            .unwrap_or_default(),
            "https://www.kernel.org/theme/images/logos/tux.png"
        );
    }

    #[test]
    fn from_https_url_to_url_with_no_protocol_and_on_different_hostname() {
        assert_eq!(
            url::resolve_url(
                "https://www.kernel.org",
                "//another-host.org/theme/images/logos/tux.png",
            )
            .unwrap_or_default(),
            "https://another-host.org/theme/images/logos/tux.png"
        );
    }

    #[test]
    fn from_https_url_to_relative_root_path() {
        assert_eq!(
            url::resolve_url(
                "https://www.kernel.org/category/signatures.html",
                "/theme/images/logos/tux.png",
            )
            .unwrap_or_default(),
            "https://www.kernel.org/theme/images/logos/tux.png"
        );
    }

    #[test]
    fn from_https_to_just_filename() {
        assert_eq!(
            url::resolve_url(
                "https://www.w3schools.com/html/html_iframe.asp",
                "default.asp",
            )
            .unwrap_or_default(),
            "https://www.w3schools.com/html/default.asp"
        );
    }

    #[test]
    fn from_data_url_to_https() {
        assert_eq!(
            url::resolve_url(
                "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
                "https://www.kernel.org/category/signatures.html",
            )
            .unwrap_or_default(),
            "https://www.kernel.org/category/signatures.html"
        );
    }

    #[test]
    fn from_data_url_to_data_url() {
        assert_eq!(
            url::resolve_url(
                "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
                "data:text/html;base64,PGEgaHJlZj0iaW5kZXguaHRtbCI+SG9tZTwvYT4K",
            )
            .unwrap_or_default(),
            "data:text/html;base64,PGEgaHJlZj0iaW5kZXguaHRtbCI+SG9tZTwvYT4K"
        );
    }

    #[test]
    fn from_file_url_to_relative_path() {
        assert_eq!(
            url::resolve_url(
                "file:///home/user/Websites/my-website/index.html",
                "assets/images/logo.png",
            )
            .unwrap_or_default(),
            "file:///home/user/Websites/my-website/assets/images/logo.png"
        );
    }

    #[test]
    fn from_file_url_to_relative_path_with_backslashes() {
        assert_eq!(
            url::resolve_url(
                "file:\\\\\\home\\user\\Websites\\my-website\\index.html",
                "assets\\images\\logo.png",
            )
            .unwrap_or_default(),
            "file:///home/user/Websites/my-website/assets/images/logo.png"
        );
    }

    #[test]
    fn from_data_url_to_file_url() {
        assert_eq!(
            url::resolve_url(
                "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
                "file:///etc/passwd",
            )
            .unwrap_or_default(),
            "file:///etc/passwd"
        );
    }

    #[test]
    fn preserve_fragment() {
        assert_eq!(
            url::resolve_url(
                "http://doesnt-matter.local/",
                "css/fonts/fontmarvelous.svg#fontmarvelous",
            )
            .unwrap_or_default(),
            "http://doesnt-matter.local/css/fonts/fontmarvelous.svg#fontmarvelous"
        );
    }

    #[test]
    fn resolve_from_file_url_to_file_url() {
        assert_eq!(
            if cfg!(windows) {
                url::resolve_url("file:///c:/index.html", "file:///c:/image.png")
                    .unwrap_or_default()
            } else {
                url::resolve_url("file:///tmp/index.html", "file:///tmp/image.png")
                    .unwrap_or_default()
            },
            if cfg!(windows) {
                "file:///c:/image.png"
            } else {
                "file:///tmp/image.png"
            }
        );
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
    fn from_data_url_to_url_with_no_protocol() {
        assert_eq!(
            url::resolve_url(
                "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
                "//www.w3schools.com/html/html_iframe.asp",
            )
            .unwrap_or_default(),
            ""
        );
    }
}
