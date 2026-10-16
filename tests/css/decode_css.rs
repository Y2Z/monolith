//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use encoding_rs::{EUC_JP, SHIFT_JIS, UTF_8, WINDOWS_1252};

    use monolith::css::{decode_css, decode_stylesheet};

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
    fn missing_charset_and_us_ascii_label() {
        // UTF-8 stylesheets served without a charset
        assert_eq!(
            decode_css("a{content:'café'}".as_bytes(), ""),
            "a{content:'café'}"
        );
        // Legacy stylesheets labeled US-ASCII (WHATWG maps that label onto Windows-1252)
        assert_eq!(
            decode_css(b"a{content:'caf\xe9'}", "us-ascii"),
            "a{content:'café'}"
        );
        // Like in browsers, the label is taken at its word, even if the data happens to be UTF-8
        assert_eq!(
            decode_css("a{content:'café'}".as_bytes(), "US-ASCII"),
            "a{content:'cafÃ©'}"
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

    #[test]
    fn environment_encoding_is_used_for_unlabeled_stylesheets() {
        let css = b"p{font-family:'\x83\x81\x83\x43\x83\x8a\x83\x49'}";

        assert_eq!(
            decode_stylesheet(css, "", Some(SHIFT_JIS)),
            ("p{font-family:'メイリオ'}".to_string(), SHIFT_JIS)
        );
        // A declared charset (US-ASCII included) beats the environment encoding
        assert_eq!(
            decode_stylesheet(css, "US-ASCII", Some(SHIFT_JIS)).1,
            WINDOWS_1252
        );
    }

    #[test]
    fn environment_encoding_beats_accidentally_valid_utf8() {
        // EUC-JP "両" is the same two bytes as UTF-8 "ξ"
        let css = b"p::before{content:'\xce\xbe'}";

        assert_eq!(
            decode_stylesheet(css, "", Some(EUC_JP)),
            ("p::before{content:'両'}".to_string(), EUC_JP)
        );
        assert_eq!(
            decode_stylesheet(css, "", None),
            ("p::before{content:'ξ'}".to_string(), UTF_8)
        );
    }

    #[test]
    fn declarations_beat_environment_encoding() {
        // Protocol charset
        assert_eq!(
            decode_stylesheet("a{content:'café'}".as_bytes(), "utf-8", Some(SHIFT_JIS)),
            ("a{content:'café'}".to_string(), UTF_8)
        );
        // @charset rule
        assert_eq!(
            decode_stylesheet(
                b"@charset \"windows-1252\";a{content:'caf\xe9'}",
                "",
                Some(SHIFT_JIS)
            ),
            (
                "@charset \"UTF-8\";a{content:'café'}".to_string(),
                WINDOWS_1252
            )
        );
        // Byte order mark
        assert_eq!(
            decode_stylesheet(b"\xef\xbb\xbfa{content:'caf\xc3\xa9'}", "", Some(SHIFT_JIS)),
            ("a{content:'café'}".to_string(), UTF_8)
        );
    }

    #[test]
    fn utf8_environment_encoding_changes_nothing() {
        assert_eq!(
            decode_stylesheet(b"a{content:'caf\xe9'}", "", Some(UTF_8)),
            ("a{content:'café'}".to_string(), WINDOWS_1252)
        );
        assert_eq!(
            decode_stylesheet("a{content:'café'}".as_bytes(), "", Some(UTF_8)),
            ("a{content:'café'}".to_string(), UTF_8)
        );
    }
}
