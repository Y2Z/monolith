//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::devtools_arguments;
    use monolith::core::MonolithOptions;
    use std::ffi::OsString;
    use std::path::Path;

    fn arguments(options: &MonolithOptions, is_visible: bool) -> Vec<OsString> {
        devtools_arguments(Path::new("/tmp/profile"), options, is_visible)
    }

    #[test]
    fn stays_away_from_the_default_profile() {
        let arguments = arguments(&MonolithOptions::default(), false);

        assert!(arguments.contains(&OsString::from("--user-data-dir=/tmp/profile")));
        assert!(arguments.contains(&OsString::from("--headless=new")));
    }

    #[test]
    fn window_is_a_matter_of_not_being_headless() {
        let arguments = arguments(&MonolithOptions::default(), true);

        assert!(!arguments.contains(&OsString::from("--headless=new")));
    }

    #[test]
    fn follows_what_was_specified() {
        let options = MonolithOptions {
            insecure: true,
            browser_user_agent: Some("Agent".to_string()),
            ..Default::default()
        };
        let arguments = arguments(&options, false);

        assert!(arguments.contains(&OsString::from("--ignore-certificate-errors")));
        assert!(arguments.contains(&OsString::from("--user-agent=Agent")));
    }
}
