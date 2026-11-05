//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::{ColorScheme, PROFILE_PREFERENCES, profile_preferences};
    use monolith::core::MonolithOptions;

    #[test]
    fn nothing_to_add() {
        let options = MonolithOptions::default();

        assert_eq!(profile_preferences(&options, None), PROFILE_PREFERENCES);
    }

    #[test]
    fn escapes_user_agent() {
        let mut options = MonolithOptions::default();
        options.browser_user_agent = Some("a\"b".to_string());

        assert!(
            profile_preferences(&options, None)
                .contains(r#"user_pref("general.useragent.override", "a\"b");"#)
        );
    }

    #[test]
    fn dark_color_scheme() {
        let preferences = profile_preferences(&MonolithOptions::default(), Some(ColorScheme::Dark));

        assert!(preferences.contains(r#"prefers-color-scheme.content-override", 0);"#));
        assert!(preferences.contains(r#"user_pref("ui.systemUsesDarkTheme", 1);"#));
    }

    #[test]
    fn light_color_scheme() {
        let preferences =
            profile_preferences(&MonolithOptions::default(), Some(ColorScheme::Light));

        assert!(preferences.contains(r#"prefers-color-scheme.content-override", 1);"#));
        assert!(preferences.contains(r#"user_pref("ui.systemUsesDarkTheme", 0);"#));
    }
}
