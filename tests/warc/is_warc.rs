#[cfg(test)]
mod passing {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use monolith::warc::is_warc;

    #[test]
    fn plain() {
        assert!(is_warc(b"WARC/1.1\r\nWARC-Type: warcinfo\r\n\r\n"));
    }

    #[test]
    fn compressed() {
        let mut encoder = GzEncoder::new(vec![], Compression::default());
        encoder.write_all(b"WARC/1.0\r\n").unwrap();

        assert!(is_warc(&encoder.finish().unwrap()));
    }
}

#[cfg(test)]
mod failing {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use monolith::warc::is_warc;

    #[test]
    fn html() {
        assert!(!is_warc(b"<html></html>"));
    }

    #[test]
    fn other_compressed_files() {
        let mut encoder = GzEncoder::new(vec![], Compression::default());
        encoder.write_all(b"<html></html>").unwrap();

        assert!(!is_warc(&encoder.finish().unwrap()));
    }

    #[test]
    fn broken_compressed_file() {
        assert!(!is_warc(&[0x1f, 0x8b, 0x08]));
    }

    #[test]
    fn nothing() {
        assert!(!is_warc(b""));
    }
}
