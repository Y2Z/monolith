use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::opts::Options;
use crate::url::{clean_url, file_url_to_fs_path, is_data_url, is_file_url, parse_data_url};

const ANSI_COLOR_RED: &str = "\x1b[31m";
const ANSI_COLOR_RESET: &str = "\x1b[0m";
const INDENT: &str = " ";
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
    "image/svg+xml",
    "text/css",
    "text/html",
    "text/javascript",
    "text/plain",
];

pub fn detect_media_type(data: &[u8], url: &str) -> String {
    for item in MAGIC.iter() {
        if data.starts_with(item[0]) {
            return String::from_utf8(item[1].to_vec()).unwrap();
        }
    }

    if url.to_lowercase().ends_with(".svg") {
        return str!("image/svg+xml");
    }

    str!()
}

pub fn is_plaintext_media_type(media_type: &str) -> bool {
    PLAINTEXT_MEDIA_TYPES.contains(&media_type.to_lowercase().as_str())
}

pub fn indent(level: u32) -> String {
    let mut result = str!();
    let mut l: u32 = level;
    while l > 0 {
        result += INDENT;
        l -= 1;
    }
    result
}

pub fn retrieve_asset(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    parent_url: &str,
    url: &str,
    options: &Options,
    depth: u32,
) -> Result<(Vec<u8>, String, String), reqwest::Error> {
    if url.len() == 0 {
        // Provoke error
        client.get("").send()?;
    }

    if is_data_url(&url) {
        let (media_type, data) = parse_data_url(url);
        Ok((data, url.to_string(), media_type))
    } else if is_file_url(&url) {
        // Check if parent_url is also file:///
        // (if not, then we don't embed the asset)
        if !is_file_url(&parent_url) {
            // Provoke error
            client.get("").send()?;
        }

        let fs_file_path: String = file_url_to_fs_path(url);
        let path = Path::new(&fs_file_path);
        if path.exists() {
            if !options.silent {
                eprintln!("{}{}", indent(depth).as_str(), &url);
            }

            Ok((fs::read(&fs_file_path).expect(""), url.to_string(), str!()))
        } else {
            // Provoke error
            Err(client.get("").send().unwrap_err())
        }
    } else {
        let cache_key: String = clean_url(&url);

        if cache.contains_key(&cache_key) {
            // URL is in cache, we get and return it
            if !options.silent {
                eprintln!("{}{} (from cache)", indent(depth).as_str(), &url);
            }

            Ok((
                cache.get(&cache_key).unwrap().to_vec(),
                url.to_string(),
                str!(),
            ))
        } else {
            // URL not in cache, we retrieve the file
            match client.get(url).send() {
                Ok(mut response) => {
                    if !options.ignore_errors && response.status() != 200 {
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

                    let res_url = response.url().to_string();

                    if !options.silent {
                        if url == res_url {
                            eprintln!("{}{}", indent(depth).as_str(), &url);
                        } else {
                            eprintln!("{}{} -> {}", indent(depth).as_str(), &url, &res_url);
                        }
                    }

                    let new_cache_key: String = clean_url(&res_url);

                    // Convert response into a byte array
                    let mut data: Vec<u8> = vec![];
                    response.copy_to(&mut data)?;

                    // Attempt to obtain media type by reading the Content-Type header
                    let media_type = response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|header| header.to_str().ok())
                        .unwrap_or("");

                    // Add retrieved resource to cache
                    cache.insert(new_cache_key, data.clone());

                    Ok((data, res_url, media_type.to_string()))
                }
                Err(error) => Err(error),
            }
        }
    }
}
