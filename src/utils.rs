extern crate base64;

use self::base64::encode;
use regex::Regex;
use url::{ParseError, Url};

lazy_static! {
    static ref HAS_PROTOCOL: Regex = Regex::new(r"^[a-z0-9]+:").unwrap();
    static ref REGEX_URL: Regex = Regex::new(r"^https?://").unwrap();
}

const MAGIC: [[&[u8]; 2]; 19] = [
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
    let mimetype = if mime.is_empty() {
        detect_mimetype(data)
    } else {
        mime.to_string()
    };
    format!("data:{};base64,{}", mimetype, encode(data))
}

pub fn detect_mimetype(data: &[u8]) -> String {
    for item in MAGIC.iter() {
        if data.starts_with(item[0]) {
            return String::from_utf8(item[1].to_vec()).unwrap();
        }
    }
    "".to_owned()
}

pub fn url_has_protocol<T: AsRef<str>>(url: T) -> bool {
    HAS_PROTOCOL.is_match(url.as_ref().to_lowercase().as_str())
}

pub fn is_data_url<T: AsRef<str>>(url: T) -> Result<bool, ParseError> {
    Url::parse(url.as_ref()).and_then(|u| Ok(u.scheme() == "data"))
}

pub fn is_valid_url<T: AsRef<str>>(path: T) -> bool {
    REGEX_URL.is_match(path.as_ref())
}

pub fn resolve_url<T: AsRef<str>, U: AsRef<str>>(from: T, to: U) -> Result<String, ParseError> {
    let result = if is_valid_url(to.as_ref()) {
        to.as_ref().to_string()
    } else {
        Url::parse(from.as_ref())?
            .join(to.as_ref())?
            .as_ref()
            .to_string()
    };
    Ok(result)
}
