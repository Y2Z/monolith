extern crate base64;

use self::base64::encode;

static MAGIC: [[&[u8]; 2]; 19] = [
    // Image
    [b"GIF87a", b"image/gif"],
    [b"GIF89a", b"image/gif"],
    [b"\xFF\xD8\xFF", b"image/jpeg"],
    [b"\x89PNG\x0D\x0A\x1A\x0A", b"image/png"],
    [b"<?xml ", b"image/svg+xml"],
    [b"<svg ", b"image/svg+xml"],
    [b"RIFF....WEBPVP8 ", b"image/webp"],
    [b"\x00\x00\x01\x00", b"image/x-icon"],
    // Audio
    [b"ID3", b"audio/mpeg"],
    [b"\xFF\x0E", b"audio/mpeg"],
    [b"\xFF\x0F", b"audio/mpeg"],
    [b"OggS", b"audio/ogg"],
    [b"RIFF....WAVEfmt ", b"audio/wav"],
    [b"fLaC", b"audio/x-flac"],
    // Video
    [b"RIFF....AVI LIST", b"video/avi"],
    [b"....ftyp", b"video/mp4"],
    [b"\x00\x00\x01\x0B", b"video/mpeg"],
    [b"....moov", b"video/quicktime"],
    [b"\x1A\x45\xDF\xA3", b"video/webm"],
];

pub fn data_to_dataurl(mime: &str, data: &[u8]) -> String {
    let mimetype = if mime == "" {
        detect_mimetype(data)
    } else {
        mime.to_string()
    };
    format!("data:{};base64,{}", mimetype, encode(data))
}

fn detect_mimetype(data: &[u8]) -> String {
    let mut re = String::new();

    for item in MAGIC.iter() {
        if data.starts_with(item[0]) {
            re = String::from_utf8(item[1].to_vec()).unwrap();
            break;
        }
    }

    re
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_to_dataurl() {
        let mime = "application/javascript";
        let data = "var word = 'hello';\nalert(word);\n";
        let datauri = data_to_dataurl(mime, data.as_bytes());
        assert_eq!(
            &datauri,
            "data:application/javascript;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
        );
    }

    #[test]
    fn test_detect_mimetype() {
        // Image
        assert_eq!(detect_mimetype(b"GIF87a"), "image/gif");
        assert_eq!(detect_mimetype(b"GIF89a"), "image/gif");
        assert_eq!(detect_mimetype(b"\xFF\xD8\xFF"), "image/jpeg");
        assert_eq!(detect_mimetype(b"\x89PNG\x0D\x0A\x1A\x0A"), "image/png");
        assert_eq!(detect_mimetype(b"<?xml "), "image/svg+xml");
        assert_eq!(detect_mimetype(b"<svg "), "image/svg+xml");
        assert_eq!(detect_mimetype(b"RIFF....WEBPVP8 "), "image/webp");
        assert_eq!(detect_mimetype(b"\x00\x00\x01\x00"), "image/x-icon");
        // Audio
        assert_eq!(detect_mimetype(b"ID3"), "audio/mpeg");
        assert_eq!(detect_mimetype(b"\xFF\x0E"), "audio/mpeg");
        assert_eq!(detect_mimetype(b"\xFF\x0F"), "audio/mpeg");
        assert_eq!(detect_mimetype(b"OggS"), "audio/ogg");
        assert_eq!(detect_mimetype(b"RIFF....WAVEfmt "), "audio/wav");
        assert_eq!(detect_mimetype(b"fLaC"), "audio/x-flac");
        // Video
        assert_eq!(detect_mimetype(b"RIFF....AVI LIST"), "video/avi");
        assert_eq!(detect_mimetype(b"....ftyp"), "video/mp4");
        assert_eq!(detect_mimetype(b"\x00\x00\x01\x0B"), "video/mpeg");
        assert_eq!(detect_mimetype(b"....moov"), "video/quicktime");
        assert_eq!(detect_mimetype(b"\x1A\x45\xDF\xA3"), "video/webm");
    }
}
