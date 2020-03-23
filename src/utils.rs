use base64;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use url::{form_urlencoded, ParseError, Url};

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

pub fn data_to_data_url(mime: &str, data: &[u8]) -> String {
    let media_type = if mime.is_empty() {
        detect_media_type(data)
    } else {
        mime.to_string()
    };
    format!("data:{};base64,{}", media_type, base64::encode(data))
}

pub fn detect_media_type(data: &[u8]) -> String {
    for item in MAGIC.iter() {
        if data.starts_with(item[0]) {
            return String::from_utf8(item[1].to_vec()).unwrap();
        }
    }
    str!()
}

pub fn url_has_protocol<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme().len() > 0))
        .unwrap_or(false)
}

pub fn is_data_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "data"))
        .unwrap_or(false)
}

pub fn is_file_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "file"))
        .unwrap_or(false)
}

pub fn is_http_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "http" || u.scheme() == "https"))
        .unwrap_or(false)
}

pub fn resolve_url<T: AsRef<str>, U: AsRef<str>>(from: T, to: U) -> Result<String, ParseError> {
    let result = if is_http_url(to.as_ref()) {
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
    as_data_url: bool,
    parent_url: &str,
    href: &str,
    opt_no_images: bool,
    opt_silent: bool,
) -> String {
    let mut resolved_css = String::from(css_string);

    for link in REGEX_CSS_URL.captures_iter(&css_string) {
        let target_link = link.name("url").unwrap().as_str();

        // Determine linked asset type
        let is_stylesheet = link.name("stylesheet").is_some();
        let is_font = link.name("font").is_some();
        let is_image = !is_stylesheet && !is_font;

        // Generate absolute URL for the content
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
                &parent_url,
                &embedded_url,
                false,      // Formatting as data URL will be done later
                "text/css", // Expect CSS
                opt_silent,
            )
            .map(|(content, _)| {
                resolve_css_imports(
                    cache,
                    client,
                    &content,
                    true, // Finally, convert to a data URL
                    &parent_url,
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
                &parent_url,
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
        if resolved_css.len() > css_string.len() {
            let offset = resolved_css.len() - css_string.len();
            let target_range = (dest.start() + offset)..(dest.end() + offset);
            resolved_css.replace_range(target_range, &replacement);
        }
    }

    if as_data_url {
        data_to_data_url("text/css", resolved_css.as_bytes())
    } else {
        resolved_css
    }
}

pub fn clean_url<T: AsRef<str>>(url: T) -> String {
    let mut result = Url::parse(url.as_ref()).unwrap();
    // Clear fragment
    result.set_fragment(None);
    // Get rid of stray question mark
    if result.query() == Some("") {
        result.set_query(None);
    }
    result.to_string()
}

pub fn data_url_to_text<T: AsRef<str>>(url: T) -> String {
    let parsed_url = Url::parse(url.as_ref()).unwrap_or(Url::parse("http://[::1]").unwrap());
    let path: String = parsed_url.path().to_string();
    let comma_loc: usize = path.find(',').unwrap_or(path.len());

    if comma_loc == path.len() {
        return str!();
    }

    let meta_data: String = path.chars().take(comma_loc).collect();
    let raw_data: String = path.chars().skip(comma_loc + 1).collect();

    let data: String = decode_url(raw_data);

    let meta_data_items: Vec<&str> = meta_data.split(';').collect();
    let mut mime_type: &str = "";
    let mut encoding: &str = "";

    let mut i: i8 = 0;
    for item in &meta_data_items {
        if i == 0 {
            if item.eq_ignore_ascii_case("text/html") {
                mime_type = item;
                continue;
            }
        }

        if item.eq_ignore_ascii_case("base64") || item.eq_ignore_ascii_case("utf8") {
            encoding = item;
        }

        i = i + 1;
    }

    if mime_type.eq_ignore_ascii_case("text/html") {
        if encoding.eq_ignore_ascii_case("base64") {
            String::from_utf8(base64::decode(&data).unwrap_or(vec![])).unwrap_or(str!())
        } else {
            data
        }
    } else {
        str!()
    }
}

pub fn decode_url(input: String) -> String {
    form_urlencoded::parse(input.as_bytes())
        .map(|(key, val)| {
            [
                key.to_string(),
                if val.to_string().len() == 0 {
                    str!()
                } else {
                    str!('=')
                },
                val.to_string(),
            ]
            .concat()
        })
        .collect()
}

pub fn retrieve_asset(
    cache: &mut HashMap<String, String>,
    client: &Client,
    parent_url: &str,
    url: &str,
    as_data_url: bool,
    mime: &str,
    opt_silent: bool,
) -> Result<(String, String), reqwest::Error> {
    if url.len() == 0 {
        return Ok((str!(), str!()));
    }

    let cache_key = clean_url(&url);

    if is_data_url(&url) {
        Ok((url.to_string(), url.to_string()))
    } else if is_file_url(&url) {
        // Check if parent_url is also file:///
        // (if not then we don't download/embed the asset)
        if !is_file_url(&parent_url) {
            return Ok((str!(), str!()));
        }

        let cutoff = if cfg!(windows) { 8 } else { 7 };
        let fs_file_path: String = decode_url(url.to_string()[cutoff..].to_string());
        let path = Path::new(&fs_file_path);
        if path.exists() {
            if !opt_silent {
                eprintln!("{}", &url);
            }

            if as_data_url {
                let data_url: String = data_to_data_url(&mime, &fs::read(&fs_file_path).unwrap());
                Ok((data_url, url.to_string()))
            } else {
                let data: String = fs::read_to_string(&fs_file_path).expect(url);
                Ok((data, url.to_string()))
            }
        } else {
            Ok((str!(), url.to_string()))
        }
    } else {
        if cache.contains_key(&cache_key) {
            // URL is in cache
            if !opt_silent {
                eprintln!("{} (from cache)", &url);
            }
            let data = cache.get(&cache_key).unwrap();
            Ok((data.to_string(), url.to_string()))
        } else {
            // URL not in cache, we request it
            let mut response = client.get(url).send()?;
            let res_url = response.url().to_string();

            if !opt_silent {
                if url == res_url {
                    eprintln!("{}", &url);
                } else {
                    eprintln!("{} -> {}", &url, &res_url);
                }
            }

            let new_cache_key = clean_url(&res_url);

            if as_data_url {
                // Convert response into a byte array
                let mut data: Vec<u8> = vec![];
                response.copy_to(&mut data)?;

                // Attempt to obtain MIME type by reading the Content-Type header
                let media_type = if mime == "" {
                    response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|header| header.to_str().ok())
                        .unwrap_or(&mime)
                } else {
                    mime
                };
                let data_url = data_to_data_url(&media_type, &data);
                // Add to cache
                cache.insert(new_cache_key, data_url.clone());
                Ok((data_url, res_url))
            } else {
                let content = response.text().unwrap();
                // Add to cache
                cache.insert(new_cache_key, content.clone());
                Ok((content, res_url))
            }
        }
    }
}
