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
    fn backslashes() {
        assert!(url::is_windows_drive_path(
            r"C:\Users\Me\Downloads\page.html"
        ));
    }

    #[test]
    fn forward_slashes() {
        assert!(url::is_windows_drive_path(
            "C:/Users/Me/Downloads/page.html"
        ));
    }

    #[test]
    fn lowercase_drive_letter() {
        assert!(url::is_windows_drive_path(r"d:\page.html"));
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
    fn file_url() {
        assert!(!url::is_windows_drive_path(
            "file:///C:/Users/Me/Downloads/page.html"
        ));
    }

    #[test]
    fn http_url() {
        assert!(!url::is_windows_drive_path("https://example.com/"));
    }

    #[test]
    fn unix_path() {
        assert!(!url::is_windows_drive_path("/home/me/page.html"));
    }

    #[test]
    fn relative_path() {
        assert!(!url::is_windows_drive_path("page.html"));
    }

    #[test]
    fn one_letter_scheme() {
        assert!(!url::is_windows_drive_path("c:page.html"));
    }
}
