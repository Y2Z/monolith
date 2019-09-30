extern crate base64;

use self::base64::encode;
use regex::Regex;
use url::{ParseError, Url};

lazy_static! {
    static ref HAS_PROTOCOL: Regex = Regex::new(r"^[a-z0-9]+:").unwrap();
    static ref REGEX_URL: Regex = Regex::new(r"^https?://").unwrap();
}

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

pub fn detect_mimetype(data: &[u8]) -> String {
    let mut re = String::new();

    for item in MAGIC.iter() {
        if data.starts_with(item[0]) {
            re = String::from_utf8(item[1].to_vec()).unwrap();
            break;
        }
    }

    re
}

pub fn url_has_protocol(url: &str) -> bool {
    HAS_PROTOCOL.is_match(&url.to_lowercase())
}

pub fn is_data_url(url: &str) -> Result<bool, ParseError> {
    match Url::parse(url) {
        Ok(parsed_url) => Ok(parsed_url.scheme() == "data"),
        Err(err) => Err(err),
    }
}

pub fn is_valid_url(path: &str) -> bool {
    REGEX_URL.is_match(path)
}

pub fn resolve_url(from: &str, to: &str) -> Result<String, ParseError> {
    let result = if is_valid_url(to) {
        to.to_string()
    } else {
        Url::parse(from)?.join(to)?.to_string()
    };

    Ok(result)
}
