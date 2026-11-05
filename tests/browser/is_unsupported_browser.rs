//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::is_unsupported_browser;
    use std::path::Path;

    #[test]
    fn chromium_and_browsers_based_on_it() {
        assert!(is_unsupported_browser(Path::new("/snap/bin/chromium")));
        assert!(is_unsupported_browser(Path::new("/usr/bin/google-chrome")));
        assert!(is_unsupported_browser(Path::new("/usr/bin/brave-browser")));
        assert!(is_unsupported_browser(Path::new("/usr/bin/microsoft-edge")));
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
    use monolith::browser::is_unsupported_browser;
    use std::path::Path;

    #[test]
    fn firefox_and_browsers_based_on_it() {
        assert!(!is_unsupported_browser(Path::new("/usr/bin/firefox")));
        assert!(!is_unsupported_browser(Path::new("/usr/bin/librewolf")));
        assert!(!is_unsupported_browser(Path::new("/usr/bin/icecat")));
    }

    #[test]
    fn directories_do_not_matter() {
        assert!(!is_unsupported_browser(Path::new("/opt/chrome/firefox")));
    }
}
