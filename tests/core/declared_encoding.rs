//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use encoding_rs::{SHIFT_JIS, UTF_8, WINDOWS_1252};

    use monolith::core::declared_encoding;

    #[test]
    fn known_labels() {
        assert_eq!(declared_encoding("utf-8"), Some(UTF_8));
        assert_eq!(declared_encoding(" Shift_JIS "), Some(SHIFT_JIS));
    }

    #[test]
    fn us_ascii_means_windows_1252() {
        // Like in browsers, an explicit US-ASCII label is a declaration, not a missing charset
        assert_eq!(declared_encoding("US-ASCII"), Some(WINDOWS_1252));
        assert_eq!(declared_encoding("us-ascii"), Some(WINDOWS_1252));
    }
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use monolith::core::declared_encoding;

    #[test]
    fn missing_charset() {
        assert_eq!(declared_encoding(""), None);
        assert_eq!(declared_encoding("  "), None);
    }

    #[test]
    fn unknown_label() {
        assert_eq!(declared_encoding("utf0"), None);
    }

    #[test]
    fn replacement_label() {
        // Labels of the "replacement" encoding are unusable for decoding documents
        assert_eq!(declared_encoding("iso-2022-kr"), None);
    }
}
