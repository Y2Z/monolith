#[cfg(test)]
mod passing {
    use monolith::core::MonolithOutputFormat;

    #[test]
    fn html_in_any_case() {
        for format in ["HTML", "HTM", "html", "htm", "Html"] {
            assert_eq!(
                format.parse::<MonolithOutputFormat>(),
                Ok(MonolithOutputFormat::HTML)
            );
        }
    }

    #[test]
    fn mhtml_in_any_case() {
        for format in ["MHTML", "MHT", "mhtml", "mht", "Mhtml"] {
            assert_eq!(
                format.parse::<MonolithOutputFormat>(),
                Ok(MonolithOutputFormat::MHTML)
            );
        }
    }
}

#[cfg(test)]
mod passing_warc {
    use monolith::core::MonolithOutputFormat;

    #[test]
    fn warc_in_any_case() {
        for format in ["WARC", "warc", "Warc"] {
            assert_eq!(
                format.parse::<MonolithOutputFormat>(),
                Ok(MonolithOutputFormat::WARC)
            );
        }
    }

    #[test]
    fn compressed_warc_in_any_case() {
        for format in ["WARCGZ", "warcgz", "WarcGz"] {
            assert_eq!(
                format.parse::<MonolithOutputFormat>(),
                Ok(MonolithOutputFormat::WARCGZ)
            );
        }
    }
}

#[cfg(test)]
mod failing {
    use monolith::core::MonolithOutputFormat;

    #[test]
    fn unknown_format() {
        assert!("zim".parse::<MonolithOutputFormat>().is_err());
    }

    #[test]
    fn nothing_specified() {
        assert!("".parse::<MonolithOutputFormat>().is_err());
    }
}
