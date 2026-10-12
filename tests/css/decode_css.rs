//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::css::decode_css;

    #[test]
    fn utf8_without_charset() {
        assert_eq!(
            decode_css("a{content:'café'}".as_bytes(), ""),
            "a{content:'café'}"
        );
    }

    #[test]
    fn protocol_charset() {
        assert_eq!(
            decode_css(b"a{content:'caf\xe9'}", "windows-1252"),
            "a{content:'café'}"
        );
        assert_eq!(
            decode_css(b"a{content:'caf\xe9'}", "ISO-8859-1"),
            "a{content:'café'}"
        );
    }

    #[test]
    fn us_ascii_is_treated_as_missing_charset() {
        // UTF-8 stylesheets served without a charset
        assert_eq!(
            decode_css("a{content:'café'}".as_bytes(), "US-ASCII"),
            "a{content:'café'}"
        );
        // Legacy stylesheets labeled US-ASCII (WHATWG maps that label onto Windows-1252)
        assert_eq!(
            decode_css(b"a{content:'caf\xe9'}", "us-ascii"),
            "a{content:'café'}"
        );
    }

    #[test]
    fn invalid_utf8_without_charset_falls_back_to_windows_1252() {
        assert_eq!(decode_css(b"a{content:'caf\xe9'}", ""), "a{content:'café'}");
    }

    #[test]
    fn unknown_protocol_charset_is_ignored() {
        assert_eq!(
            decode_css("a{content:'café'}".as_bytes(), "made-up-charset"),
            "a{content:'café'}"
        );
    }

    #[test]
    fn bom_beats_protocol_charset_and_is_stripped() {
        assert_eq!(
            decode_css(b"\xef\xbb\xbfa{content:'caf\xc3\xa9'}", "windows-1252"),
            "a{content:'café'}"
        );
        assert_eq!(decode_css(b"\xff\xfea\x00{\x00}\x00", ""), "a{}");
    }

    #[test]
    fn charset_rule_is_used_and_rewritten() {
        assert_eq!(
            decode_css(b"@charset \"windows-1252\";a{content:'caf\xe9'}", ""),
            "@charset \"UTF-8\";a{content:'café'}"
        );
    }

    #[test]
    fn protocol_charset_beats_charset_rule() {
        assert_eq!(
            decode_css(
                "@charset \"windows-1252\";a{content:'café'}".as_bytes(),
                "utf-8"
            ),
            "@charset \"UTF-8\";a{content:'café'}"
        );
    }

    #[test]
    fn utf8_charset_rule_is_left_alone() {
        assert_eq!(
            decode_css("@charset \"utf-8\";a{content:'café'}".as_bytes(), ""),
            "@charset \"utf-8\";a{content:'café'}"
        );
    }

    #[test]
    fn utf16_charset_rule_without_bom_means_utf8() {
        assert_eq!(
            decode_css("@charset \"UTF-16\";a{content:'café'}".as_bytes(), ""),
            "@charset \"UTF-8\";a{content:'café'}"
        );
    }

    #[test]
    fn malformed_charset_rule_is_ignored() {
        // Per spec, only the exact `@charset "...";` form counts
        assert_eq!(
            decode_css(b"@charset 'koi8-r';a{content:'caf\xe9'}", ""),
            "@charset 'koi8-r';a{content:'café'}"
        );
        assert_eq!(
            decode_css(b"@charset  \"koi8-r\";a{content:'caf\xe9'}", ""),
            "@charset  \"koi8-r\";a{content:'café'}"
        );
    }
}
