extern crate base64;

use self::base64::encode;
use http::retrieve_asset;
use regex::Regex;
use url::{ParseError, Url};

lazy_static! {
    static ref HAS_PROTOCOL: Regex = Regex::new(r"^[a-z0-9]+:").unwrap();
    static ref REGEX_URL: Regex = Regex::new(r"^https?://").unwrap();
    static ref EMPTY_STRING: String = String::new();
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

pub fn resolve_css_imports(
    css_string: &str,
    href: &str,
    opt_user_agent: &str,
    opt_silent: bool,
    opt_insecure: bool,
) -> Result<String, String> {
    let mut resolved_css = String::from(css_string);
    let re = Regex::new(r###"url\("?([^"]+)"?\)"###).unwrap();

    for link in re.captures_iter(&css_string) {
        let target_link = dbg!(link.get(1).unwrap().as_str());

        // Generate absolute URL for content
        let embedded_url = match resolve_url(href, target_link) {
            Ok(url) => url,
            Err(_) => continue, // Malformed URL
        };

        let (css_dataurl, _) = retrieve_asset(
            &embedded_url,
            true, // true
            "",
            opt_user_agent,
            opt_silent,
            opt_insecure,
        )
        .unwrap_or((EMPTY_STRING.clone(), EMPTY_STRING.clone()));

        let replacement = &[
            "\"",
            &css_dataurl
                .replace("\"", &["\\", "\""].concat())
                .to_string(),
            "\"",
        ]
        .concat();
        let t = resolved_css
            .replace(&link[0][4..link[0].len() - 1], &replacement)
            .to_string();
        resolved_css = t.clone();
    }

    let encoded_css = data_to_dataurl("text/css", resolved_css.as_bytes());

    Ok(encoded_css.to_string())
}
