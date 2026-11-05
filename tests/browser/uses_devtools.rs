//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::uses_devtools;
    use std::path::Path;

    #[test]
    fn chromium_and_browsers_based_on_it() {
        assert!(uses_devtools(Path::new("/snap/bin/chromium")));
        assert!(uses_devtools(Path::new("/usr/bin/google-chrome")));
        assert!(uses_devtools(Path::new("/usr/bin/brave-browser")));
        assert!(uses_devtools(Path::new("/usr/bin/microsoft-edge")));
    }

    #[test]
    fn executable_names_with_uppercase_letters() {
        assert!(uses_devtools(Path::new(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
        )));
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
    use monolith::browser::uses_devtools;
    use std::path::Path;

    #[test]
    fn firefox_and_browsers_based_on_it() {
        assert!(!uses_devtools(Path::new("/usr/bin/firefox")));
        assert!(!uses_devtools(Path::new("/usr/bin/librewolf")));
        assert!(!uses_devtools(Path::new("/usr/bin/icecat")));
    }

    #[test]
    fn directories_do_not_matter() {
        assert!(!uses_devtools(Path::new("/opt/chrome/firefox")));
    }
}
