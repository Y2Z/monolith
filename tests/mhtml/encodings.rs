//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::mhtml::{
        decode_base64, decode_quoted_printable, encode_base64, encode_quoted_printable,
    };

    #[test]
    fn quoted_printable_decoding() {
        assert_eq!(
            decode_quoted_printable(b"a=3Db=3dc=\r\nd=\ne= \r\nf"),
            b"a=b=cdef"
        );
    }

    #[test]
    fn quoted_printable_decoding_invalid_escapes() {
        assert_eq!(decode_quoted_printable(b"=ZZ x=4"), b"=ZZ x=4");
    }

    #[test]
    fn quoted_printable_encoding_line_breaks() {
        assert_eq!(encode_quoted_printable(b"a\r\nb"), "a\r\nb");
        assert_eq!(encode_quoted_printable(b"a\nb"), "a=0A=\r\nb");
        assert_eq!(encode_quoted_printable(b"a\rb"), "a=0Db");
    }

    #[test]
    fn quoted_printable_encoding_line_starts() {
        // Lines can't start with "-" (boundaries) or "." (SMTP)
        assert_eq!(encode_quoted_printable(b"--x\r\n.y-z."), "=2D-x\r\n=2Ey-z.");
    }

    #[test]
    fn quoted_printable_encoding_trailing_whitespace() {
        assert_eq!(encode_quoted_printable(b"a \r\nb\t"), "a=20\r\nb=09");
    }

    #[test]
    fn quoted_printable_round_trip() {
        let mut data: Vec<u8> = Vec::new();
        for i in 0..5000u32 {
            data.push(((i * 7919) % 256) as u8);
            if i % 97 == 0 {
                data.extend_from_slice(b"\r\n-- x.  \n-\t\r");
            }
        }

        let encoded: String = encode_quoted_printable(&data);

        assert!(encoded.is_ascii());
        for line in encoded.split("\r\n") {
            assert!(line.len() <= 76);
            assert!(!line.starts_with('-'));
            assert!(!line.ends_with(' ') && !line.ends_with('\t'));
        }
        assert_eq!(decode_quoted_printable(encoded.as_bytes()), data);
    }

    #[test]
    fn base64_round_trip() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        let encoded: String = encode_base64(&data);

        assert!(encoded.split("\r\n").all(|line| line.len() <= 76));
        assert_eq!(decode_base64(encoded.as_bytes()), data);
    }

    #[test]
    fn base64_decoding_without_padding() {
        assert_eq!(decode_base64(b"YQ"), b"a");
        assert_eq!(decode_base64(b"R0lG\r\nODlh"), b"GIF89a");
    }
}
