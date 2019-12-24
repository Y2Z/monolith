extern crate base64;

use self::base64::encode;
use http::retrieve_asset;
use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use url::{ParseError, Url};

/// This monster of a regex is used to match any kind of URL found in CSS.
///
/// There  are roughly three different categories that a found URL could fit
/// into:
///    - Font       [found after a src: property in an @font-family rule]
///    - Stylesheet [denoted by an @import before the url
///    - Image      [covers all other uses of the url() function]
///
/// This regex aims to extract the following information:
///    - What type of URL is it (font/image/css)
///    - Where is the part that needs to be replaced (incl any wrapping quotes)
///    - What is the URL (excl any wrapping quotes)
///
/// Essentially, the regex can be broken down into two parts:
///
/// `(?:(?P<import>@import)|(?P<font>src\s*:)\s+)?`
/// This matches the precursor to a font or CSS URL, and fills in a match under
/// either `<import>` (if it's a CSS URL) or `<font>` (if it's a font).
/// Determining whether or not it's an image can be done by the negation of both
/// of these. Either zero or one of these can match.
///
/// `url\((?P<to_repl>['"]?(?P<url>[^"'\)]+)['"]?)\)`
/// This matches the actual URL part of the url(), and must always match. It also
/// sets `<to_repl>` and `<url>` which correspond to everything within
/// `url(...)` and a usable URL, respectively.
///
/// Note, however, that this does not perform any validation of the found URL.
/// Malformed CSS could lead to an invalid URL being present. It is therefore
/// recomended that the URL gets manually validated.
const CSS_URL_REGEX_STR: &str = r###"(?:(?:(?P<stylesheet>@import)|(?P<font>src\s*:))\s+)?url\((?P<to_repl>['"]?(?P<url>[^"'\)]+)['"]?)\)"###;

lazy_static! {
    static ref HAS_PROTOCOL: Regex = Regex::new(r"^[a-z0-9]+:").unwrap();
    static ref REGEX_URL: Regex = Regex::new(r"^https?://").unwrap();
    static ref REGEX_CSS_URL: Regex = Regex::new(CSS_URL_REGEX_STR).unwrap();
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

pub fn resolve_css_imports(
    cache: &mut HashMap<String, String>,
    client: &Client,
    css_string: &str,
    as_dataurl: bool,
    href: &str,
    opt_no_images: bool,
    opt_silent: bool,
) -> String {
    let mut resolved_css = String::from(css_string);

    for link in REGEX_CSS_URL.captures_iter(&css_string) {
        let target_link = link.name("url").unwrap().as_str();

        // Determine the type of link
        let is_stylesheet = link.name("stylesheet").is_some();
        let is_font = link.name("font").is_some();
        let is_image = !is_stylesheet && !is_font;

        // Generate absolute URL for content
        let embedded_url = match resolve_url(href, target_link) {
            Ok(url) => url,
            Err(_) => continue, // Malformed URL
        };

        // Download the asset. If it's more CSS, resolve that too
        let content = if is_stylesheet {
            // The link is an @import link
            retrieve_asset(
                cache,
                client,
                &embedded_url,
                false,      // Formating as data URL will be done later
                "text/css", // Expect CSS
                opt_silent,
            )
            .map(|(content, _)| {
                resolve_css_imports(
                    cache,
                    client,
                    &content,
                    true, // Finally, convert to a dataurl
                    &embedded_url,
                    opt_no_images,
                    opt_silent,
                )
            })
        } else if (is_image && !opt_no_images) || is_font {
            // The link is some other, non-@import link
            retrieve_asset(
                cache,
                client,
                &embedded_url,
                true, // Format as data URL
                "",   // Unknown MIME type
                opt_silent,
            )
            .map(|(a, _)| a)
        } else {
            // If it's a datatype that has been opt_no'd out of, replace with
            // absolute URL

            Ok(embedded_url.clone())
        }
        .unwrap_or_else(|e| {
            eprintln!("Warning: {}", e);

            // If failed to resolve, replace with absolute URL
            embedded_url
        });

        let replacement = format!("\"{}\"", &content);
        let dest = link.name("to_repl").unwrap();
        let offset = resolved_css.len() - css_string.len();
        let target_range = (dest.start() + offset)..(dest.end() + offset);

        resolved_css.replace_range(target_range, &replacement);
    }

    if as_dataurl {
        data_to_dataurl("text/css", resolved_css.as_bytes())
    } else {
        resolved_css
    }
}
