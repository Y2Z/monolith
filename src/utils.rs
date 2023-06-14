use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use url::Url;

use crate::opts::{Options, OPTIONS};
use crate::url::{clean_url, parse_data_url};

const ANSI_COLOR_RED: &str = "\x1b[31m";
const ANSI_COLOR_RESET: &str = "\x1b[0m";
const MAGIC: [[&[u8]; 2]; 18] = [
    // Image
    [b"GIF87a", b"image/gif"],
    [b"GIF89a", b"image/gif"],
    [b"\xFF\xD8\xFF", b"image/jpeg"],
    [b"\x89PNG\x0D\x0A\x1A\x0A", b"image/png"],
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
const PLAINTEXT_MEDIA_TYPES: &[&str] = &[
    "application/javascript",
    "application/json",
    "image/svg+xml",
];

static CLIENT: Lazy<Client> = Lazy::new(|| {
    let mut header_map = HeaderMap::new();
    if let Some(user_agent) = &OPTIONS.user_agent {
        header_map.insert(
            USER_AGENT,
            HeaderValue::from_str(user_agent).expect("Invalid User-Agent header specified"),
        );
    }

    if OPTIONS.timeout > 0 {
        Client::builder().timeout(Duration::from_secs(OPTIONS.timeout))
    } else {
        Client::builder()
    }
    .danger_accept_invalid_certs(OPTIONS.insecure)
    .default_headers(header_map)
    .build()
    .expect("Failed to initialize HTTP client")
});

static CACHE: Lazy<Mutex<HashMap<String, Vec<u8>>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    Mutex::new(m)
});

pub fn detect_media_type(data: &[u8], url: &Url) -> String {
    // At first attempt to read file's header
    for magic_item in MAGIC.iter() {
        if data.starts_with(magic_item[0]) {
            return String::from_utf8(magic_item[1].to_vec()).unwrap();
        }
    }

    // If header didn't match any known magic signatures,
    // try to guess media type from file name
    let parts: Vec<&str> = url.path().split('/').collect();
    detect_media_type_by_file_name(parts.last().unwrap())
}

pub fn detect_media_type_by_file_name(filename: &str) -> String {
    let filename_lowercased: &str = &filename.to_lowercase();
    let parts: Vec<&str> = filename_lowercased.split('.').collect();

    let mime: &str = match parts.last() {
        Some(v) => match *v {
            "avi" => "video/avi",
            "bmp" => "image/bmp",
            "css" => "text/css",
            "flac" => "audio/flac",
            "gif" => "image/gif",
            "htm" | "html" => "text/html",
            "ico" => "image/x-icon",
            "jpeg" | "jpg" => "image/jpeg",
            "js" => "application/javascript",
            "json" => "application/json",
            "mp3" => "audio/mpeg",
            "mp4" | "m4v" => "video/mp4",
            "ogg" => "audio/ogg",
            "ogv" => "video/ogg",
            "pdf" => "application/pdf",
            "png" => "image/png",
            "svg" => "image/svg+xml",
            "swf" => "application/x-shockwave-flash",
            "tif" | "tiff" => "image/tiff",
            "txt" => "text/plain",
            "wav" => "audio/wav",
            "webp" => "image/webp",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "xml" => "text/xml",
            &_ => "",
        },
        None => "",
    };

    mime.to_string()
}

pub fn domain_is_within_domain(domain: &str, domain_to_match_against: &str) -> bool {
    if domain_to_match_against.is_empty() {
        return false;
    }

    if domain_to_match_against == "." {
        return true;
    }

    let domain_partials: Vec<&str> = domain.trim_end_matches('.').rsplit('.').collect();
    let domain_to_match_against_partials: Vec<&str> = domain_to_match_against
        .trim_end_matches('.')
        .rsplit('.')
        .collect();
    let domain_to_match_against_starts_with_a_dot = domain_to_match_against.starts_with('.');

    let mut i: usize = 0;
    let l: usize = std::cmp::max(
        domain_partials.len(),
        domain_to_match_against_partials.len(),
    );
    let mut ok: bool = true;

    while i < l {
        // Exit and return false if went out of bounds of domain to match against, and it didn't start with a dot
        if !domain_to_match_against_starts_with_a_dot
            && domain_to_match_against_partials.len() < i + 1
        {
            ok = false;
            break;
        }

        let domain_partial = if domain_partials.len() < i + 1 {
            ""
        } else {
            domain_partials.get(i).unwrap()
        };
        let domain_to_match_against_partial = if domain_to_match_against_partials.len() < i + 1 {
            ""
        } else {
            domain_to_match_against_partials.get(i).unwrap()
        };

        let parts_match = domain_to_match_against_partial.eq_ignore_ascii_case(domain_partial);

        if !parts_match && !domain_to_match_against_partial.is_empty() {
            ok = false;
            break;
        }

        i += 1;
    }

    ok
}

pub fn indent(level: u32) -> String {
    let mut result: String = String::new();
    let mut l: u32 = level;

    while l > 0 {
        result += " ";
        l -= 1;
    }

    result
}

pub fn is_plaintext_media_type(media_type: &str) -> bool {
    media_type.to_lowercase().as_str().starts_with("text/")
        || PLAINTEXT_MEDIA_TYPES.contains(&media_type.to_lowercase().as_str())
}

pub fn parse_content_type(content_type: &str) -> (String, String, bool) {
    let mut media_type: String = "text/plain".to_string();
    let mut charset: String = "US-ASCII".to_string();
    let mut is_base64: bool = false;

    // Parse meta data
    let content_type_items: Vec<&str> = content_type.split(';').collect();
    for (i, item) in (0_i8..).zip(content_type_items.iter()) {
        if i == 0 {
            if !item.trim().is_empty() {
                media_type = item.trim().to_string();
            }
        } else if item.trim().eq_ignore_ascii_case("base64") {
            is_base64 = true;
        } else if item.trim().starts_with("charset=") {
            charset = item.trim().chars().skip(8).collect();
        }
    }

    (media_type, charset, is_base64)
}

pub fn retrieve_asset(
    parent_url: &Url,
    url: &Url,
    options: &Options,
    depth: u32,
) -> Result<(Vec<u8>, Url, String, String), reqwest::Error> {
    if url.scheme() == "data" {
        let (media_type, charset, data) = parse_data_url(url);
        Ok((data, url.clone(), media_type, charset))
    } else if url.scheme() == "file" {
        // Check if parent_url is also a file: URL (if not, then we don't embed the asset)
        if parent_url.scheme() != "file" {
            if !options.silent {
                eprintln!(
                    "{}{}{} (Security Error){}",
                    indent(depth).as_str(),
                    if options.no_color { "" } else { ANSI_COLOR_RED },
                    &url,
                    if options.no_color {
                        ""
                    } else {
                        ANSI_COLOR_RESET
                    },
                );
            }
            // Provoke error
            CLIENT.get("").send()?;
        }

        let path_buf: PathBuf = url.to_file_path().unwrap();
        let path: &Path = path_buf.as_path();
        if path.exists() {
            if path.is_dir() {
                if !options.silent {
                    eprintln!(
                        "{}{}{} (is a directory){}",
                        indent(depth).as_str(),
                        if options.no_color { "" } else { ANSI_COLOR_RED },
                        &url,
                        if options.no_color {
                            ""
                        } else {
                            ANSI_COLOR_RESET
                        },
                    );
                }

                // Provoke error
                Err(CLIENT.get("").send().unwrap_err())
            } else {
                if !options.silent {
                    eprintln!("{}{}", indent(depth).as_str(), &url);
                }

                let file_blob: Vec<u8> = fs::read(path).expect("Unable to read file");
                let file_type = detect_media_type(&file_blob, url);

                Ok((file_blob, url.clone(), file_type, "".to_string()))
            }
        } else {
            if !options.silent {
                eprintln!(
                    "{}{}{} (not found){}",
                    indent(depth).as_str(),
                    if options.no_color { "" } else { ANSI_COLOR_RED },
                    &url,
                    if options.no_color {
                        ""
                    } else {
                        ANSI_COLOR_RESET
                    },
                );
            }

            // Provoke error
            Err(CLIENT.get("").send().unwrap_err())
        }
    } else {
        let cache_key: String = clean_url(url.clone()).as_str().to_string();

        if CACHE.lock().unwrap().contains_key(&cache_key) {
            // URL is in cache, we get and return it
            if !options.silent {
                eprintln!("{}{} (from cache)", indent(depth).as_str(), &url);
            }

            Ok((
                CACHE.lock().unwrap().get(&cache_key).unwrap().to_vec(),
                url.clone(),
                "".to_string(),
                "".to_string(),
            ))
        } else {
            if let Some(domains) = &options.domains {
                let domain_matches = domains
                    .iter()
                    .any(|d| domain_is_within_domain(url.host_str().unwrap(), d.trim()));
                if (options.blacklist_domains && domain_matches)
                    || (!options.blacklist_domains && !domain_matches)
                {
                    return Err(CLIENT.get("").send().unwrap_err());
                }
            }

            // URL not in cache, we retrieve the file
            match CLIENT.get(url.as_str()).send() {
                Ok(response) => {
                    if !options.ignore_errors && response.status() != reqwest::StatusCode::OK {
                        if !options.silent {
                            eprintln!(
                                "{}{}{} ({}){}",
                                indent(depth).as_str(),
                                if options.no_color { "" } else { ANSI_COLOR_RED },
                                &url,
                                response.status(),
                                if options.no_color {
                                    ""
                                } else {
                                    ANSI_COLOR_RESET
                                },
                            );
                        }
                        // Provoke error
                        return Err(CLIENT.get("").send().unwrap_err());
                    }

                    let response_url: Url = response.url().clone();

                    if !options.silent {
                        if url.as_str() == response_url.as_str() {
                            eprintln!("{}{}", indent(depth).as_str(), &url);
                        } else {
                            eprintln!("{}{} -> {}", indent(depth).as_str(), &url, &response_url);
                        }
                    }

                    let new_cache_key: String = clean_url(response_url.clone()).to_string();

                    // Attempt to obtain media type and charset by reading Content-Type header
                    let content_type: &str = response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|header| header.to_str().ok())
                        .unwrap_or("");

                    let (media_type, charset, _is_base64) = parse_content_type(content_type);

                    // Convert response into a byte array
                    let mut data: Vec<u8> = vec![];
                    match response.bytes() {
                        Ok(b) => {
                            data = b.to_vec();
                        }
                        Err(error) => {
                            if !options.silent {
                                eprintln!(
                                    "{}{}{}{}",
                                    indent(depth).as_str(),
                                    if options.no_color { "" } else { ANSI_COLOR_RED },
                                    error,
                                    if options.no_color {
                                        ""
                                    } else {
                                        ANSI_COLOR_RESET
                                    },
                                );
                            }
                        }
                    }

                    // Add retrieved resource to cache
                    CACHE.lock().unwrap().insert(new_cache_key, data.clone());

                    // Return
                    Ok((data, response_url, media_type, charset))
                }
                Err(error) => {
                    if !options.silent {
                        eprintln!(
                            "{}{}{} ({}){}",
                            indent(depth).as_str(),
                            if options.no_color { "" } else { ANSI_COLOR_RED },
                            &url,
                            error,
                            if options.no_color {
                                ""
                            } else {
                                ANSI_COLOR_RESET
                            },
                        );
                    }

                    Err(CLIENT.get("").send().unwrap_err())
                }
            }
        }
    }
}
