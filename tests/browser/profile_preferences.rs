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

        assert_eq!(
            profile_preferences(&options, None, false),
            PROFILE_PREFERENCES
        );
    }

    #[test]
    fn escapes_user_agent() {
        let options = MonolithOptions {
            browser_user_agent: Some("a\"b".to_string()),
            ..Default::default()
        };

        assert!(
            profile_preferences(&options, None, false)
                .contains(r#"user_pref("general.useragent.override", "a\"b");"#)
        );
    }

    #[test]
    fn dark_color_scheme() {
        let preferences =
            profile_preferences(&MonolithOptions::default(), Some(ColorScheme::Dark), false);

        assert!(preferences.contains(r#"prefers-color-scheme.content-override", 0);"#));
        assert!(preferences.contains(r#"user_pref("ui.systemUsesDarkTheme", 1);"#));
    }

    #[test]
    fn light_color_scheme() {
        let preferences =
            profile_preferences(&MonolithOptions::default(), Some(ColorScheme::Light), false);

        assert!(preferences.contains(r#"prefers-color-scheme.content-override", 1);"#));
        assert!(preferences.contains(r#"user_pref("ui.systemUsesDarkTheme", 0);"#));
    }

    #[test]
    fn javascript_disabled() {
        let preferences = profile_preferences(&MonolithOptions::default(), None, true);

        assert!(preferences.contains(r#"user_pref("javascript.enabled", false);"#));
    }
}
