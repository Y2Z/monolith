//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::snap_name;
    use std::path::Path;

    #[test]
    fn names_of_snaps() {
        assert_eq!(snap_name(Path::new("/snap/bin/chromium")), "chromium");
        assert_eq!(snap_name(Path::new("/usr/bin/firefox")), "firefox");
        assert_eq!(snap_name(Path::new("/snap/bin/brave")), "brave");
    }

    #[test]
    fn ubuntu_launcher() {
        assert_eq!(
            snap_name(Path::new("/usr/bin/chromium-browser")),
            "chromium"
        );
    }
}
