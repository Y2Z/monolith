//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::{executable_name, find_browser_in};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn prefers_firefox_over_its_forks() {
        let directory = TempDir::new().unwrap();
        let firefox = directory.path().join(executable_name("firefox"));
        let icecat = directory.path().join(executable_name("icecat"));

        fs::write(&icecat, "").unwrap();
        assert_eq!(
            find_browser_in(&[directory.path().to_path_buf()]),
            Some(icecat)
        );

        fs::write(&firefox, "").unwrap();
        assert_eq!(
            find_browser_in(&[directory.path().to_path_buf()]),
            Some(firefox)
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
    use monolith::browser::find_browser_in;
    use tempfile::TempDir;

    #[test]
    fn empty_directory() {
        let directory = TempDir::new().unwrap();

        assert_eq!(find_browser_in(&[directory.path().to_path_buf()]), None);
    }
}
