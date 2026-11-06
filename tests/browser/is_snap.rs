//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::is_snap;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn launcher_script() {
        let directory = TempDir::new().unwrap();
        let launcher = directory.path().join("firefox");
        fs::write(&launcher, "#!/bin/sh\nexec /snap/bin/firefox \"$@\"\n").unwrap();

        assert!(is_snap(&launcher));
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
    use monolith::browser::is_snap;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn regular_launcher_script() {
        let directory = TempDir::new().unwrap();
        let launcher = directory.path().join("firefox");
        fs::write(
            &launcher,
            "#!/bin/sh\nexec /usr/lib/firefox/firefox \"$@\"\n",
        )
        .unwrap();

        assert!(!is_snap(&launcher));
    }
}
