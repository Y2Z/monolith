use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

use crate::opts::Options;
use crate::url::{clean_url, parse_data_url};

const ANSI_COLOR_RED: &'static str = "\x1b[31m";
const ANSI_COLOR_RESET: &'static str = "\x1b[0m";
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
    let mut i: i8 = 0;
    for item in &content_type_items {
        if i == 0 {
            if item.trim().len() > 0 {
                media_type = item.trim().to_string();
            }
        } else {
            if item.trim().eq_ignore_ascii_case("base64") {
                is_base64 = true;
            } else if item.trim().starts_with("charset=") {
                charset = item.trim().chars().skip(8).collect();
            }
        }

        i += 1;
    }

    (media_type, charset, is_base64)
}

pub fn retrieve_asset(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    parent_url: &Url,
    url: &Url,
    options: &Options,
    depth: u32,
) -> Result<(Vec<u8>, Url, String, String), reqwest::Error> {
    if url.scheme() == "data" {
        let (media_type, charset, data) = parse_data_url(url);
        Ok((data, url.clone(), media_type, charset))
    } else if url.scheme() == "file" {
        // Check if parent_url is also file:/// (if not, then we don't embed the asset)
        if parent_url.scheme() != "file" {
            if !options.silent {
                eprintln!(
                    "{}{}{} ({}){}",
                    indent(depth).as_str(),
                    if options.no_color { "" } else { ANSI_COLOR_RED },
                    &url,
                    "Security Error",
                    if options.no_color {
                        ""
                    } else {
                        ANSI_COLOR_RESET
                    },
                );
            }
            // Provoke error
            client.get("").send()?;
        }

        let path_buf: PathBuf = url.to_file_path().unwrap().clone();
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
                Err(client.get("").send().unwrap_err())
            } else {
                if !options.silent {
                    eprintln!("{}{}", indent(depth).as_str(), &url);
                }

                let file_blob: Vec<u8> = fs::read(&path).expect("Unable to read file");

                Ok((
                    file_blob.clone(),
                    url.clone(),
                    detect_media_type(&file_blob, url),
                    "".to_string(),
                ))
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
            Err(client.get("").send().unwrap_err())
        }
    } else {
        let cache_key: String = clean_url(url.clone()).as_str().to_string();

        if cache.contains_key(&cache_key) {
            // URL is in cache, we get and return it
            if !options.silent {
                eprintln!("{}{} (from cache)", indent(depth).as_str(), &url);
            }

            Ok((
                cache.get(&cache_key).unwrap().to_vec(),
                url.clone(),
                "".to_string(),
                "".to_string(),
            ))
        } else {
            // URL not in cache, we retrieve the file
            match client.get(url.as_str()).send() {
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
                        return Err(client.get("").send().unwrap_err());
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

                    let (media_type, charset, _is_base64) = parse_content_type(&content_type);

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
                    cache.insert(new_cache_key, data.clone());

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

                    Err(client.get("").send().unwrap_err())
                }
            }
        }
    }
}
