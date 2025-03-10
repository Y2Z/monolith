use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, prelude::*};
use std::path::{Path, PathBuf};
use std::time::Duration;

use encoding_rs::Encoding;
use markup5ever_rcdom::RcDom;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE, REFERER, USER_AGENT};
use url::Url;

use crate::cache::Cache;
use crate::cookies::Cookie;
use crate::html::{
    add_favicon, create_metadata_tag, get_base_url, get_charset, has_favicon, html_to_dom,
    serialize_document, set_base_url, set_charset, walk_and_embed_assets,
};
use crate::url::{clean_url, create_data_url, get_referer_url, parse_data_url, resolve_url};

#[derive(Debug)]
pub struct MonolithError {
    details: String,
}

impl MonolithError {
    fn new(msg: &str) -> MonolithError {
        MonolithError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for MonolithError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MonolithError {
    fn description(&self) -> &str {
        &self.details
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
pub enum MonolithOutputFormat {
    #[default]
    HTML,
    // MHT,
    // WARC,
    // ZIM,
    // HAR,
}

#[derive(Default)]
pub struct Options {
    pub base_url: Option<String>,
    pub blacklist_domains: bool,
    pub cookies: Vec<Cookie>, // TODO: move out of this struct
    pub domains: Option<Vec<String>>,
    pub encoding: Option<String>,
    pub ignore_errors: bool,
    pub insecure: bool,
    pub isolate: bool,
    pub no_audio: bool,
    pub no_color: bool,
    pub no_css: bool,
    pub no_fonts: bool,
    pub no_frames: bool,
    pub no_images: bool,
    pub no_js: bool,
    pub no_metadata: bool,
    pub no_video: bool,
    pub output_format: MonolithOutputFormat,
    pub silent: bool,
    pub timeout: u64,
    pub unwrap_noscript: bool,
    pub user_agent: Option<String>,
}

const ANSI_COLOR_RED: &str = "\x1b[31m";
const ANSI_COLOR_RESET: &str = "\x1b[0m";
const FILE_SIGNATURES: [[&[u8]; 2]; 18] = [
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

pub fn create_monolithic_document(
    source: String,
    options: &Options,
    cache: &mut Option<Cache>,
) -> Result<Vec<u8>, MonolithError> {
    // Check if source was provided
    if source.is_empty() {
        return Err(MonolithError::new("no target specified"));
    }

    // Check if custom encoding value is acceptable
    if let Some(custom_encoding) = options.encoding.clone() {
        if Encoding::for_label_no_replacement(custom_encoding.as_bytes()).is_none() {
            return Err(MonolithError::new(&format!(
                "unknown encoding \"{}\"",
                &custom_encoding
            )));
        }
    }

    let mut use_stdin: bool = false;

    let target_url = match source.as_str() {
        "-" => {
            // Read from pipe (stdin)
            use_stdin = true;
            // Set default target URL to an empty data URL; the user can set it via --base-url
            Url::parse("data:text/html,").unwrap()
        }
        target => match Url::parse(target) {
            Ok(url) => match url.scheme() {
                "data" | "file" | "http" | "https" => url,
                unsupported_scheme => {
                    return Err(MonolithError::new(&format!(
                        "unsupported target URL scheme \"{}\"",
                        unsupported_scheme
                    )));
                }
            },
            Err(_) => {
                // Failed to parse given base URL (perhaps it's a filesystem path?)
                let path: &Path = Path::new(&target);
                match path.exists() {
                    true => match path.is_file() {
                        true => {
                            let canonical_path = fs::canonicalize(path).unwrap();
                            match Url::from_file_path(canonical_path) {
                                Ok(url) => url,
                                Err(_) => {
                                    return Err(MonolithError::new(&format!(
                                        "could not generate file URL out of given path \"{}\"",
                                        &target
                                    )));
                                }
                            }
                        }
                        false => {
                            return Err(MonolithError::new(&format!(
                                "local target \"{}\" is not a file",
                                &target
                            )));
                        }
                    },
                    false => {
                        // It is not a FS path, now we do what browsers do:
                        // prepend "http://" and hope it points to a website
                        Url::parse(&format!("http://{}", &target)).unwrap()
                    }
                }
            }
        },
    };

    // Initialize HTTP client
    let mut header_map = HeaderMap::new();
    if let Some(user_agent) = &options.user_agent {
        header_map.insert(
            USER_AGENT,
            HeaderValue::from_str(user_agent).expect("Invalid User-Agent header specified"),
        );
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(if options.timeout > 0 {
            options.timeout
        } else {
            // We have to specify something that eventually makes the program fail
            // (to prevent it from hanging forever)
            600
        }))
        .danger_accept_invalid_certs(options.insecure)
        .default_headers(header_map)
        .build()
        .expect("Failed to initialize HTTP client");

    // At first we assume that base URL is same as target URL
    let mut base_url: Url = target_url.clone();

    let data: Vec<u8>;
    let mut document_encoding: String = "".to_string();
    let mut dom: RcDom;

    // Retrieve target document
    if use_stdin {
        data = read_stdin();
    } else if target_url.scheme() == "file"
        || target_url.scheme() == "http"
        || target_url.scheme() == "https"
        || target_url.scheme() == "data"
    {
        match retrieve_asset(cache, &client, &target_url, &target_url, options) {
            Ok((retrieved_data, final_url, media_type, charset)) => {
                // Provide output as text (without processing it, the way browsers do)
                if !media_type.eq_ignore_ascii_case("text/html")
                    && !media_type.eq_ignore_ascii_case("application/xhtml+xml")
                {
                    return Ok(retrieved_data);
                }

                if options
                    .base_url
                    .clone()
                    .unwrap_or("".to_string())
                    .is_empty()
                {
                    base_url = final_url;
                }

                data = retrieved_data;
                document_encoding = charset;
            }
            Err(_) => {
                return Err(MonolithError::new("could not retrieve target document"));
            }
        }
    } else {
        return Err(MonolithError::new("unsupported target"));
    }

    // Initial parse
    dom = html_to_dom(&data, document_encoding.clone());

    // TODO: investigate if charset from filesystem/data URL/HTTP headers
    //       has say over what's specified in HTML

    // Attempt to determine document's charset
    if let Some(html_charset) = get_charset(&dom.document) {
        if !html_charset.is_empty() {
            // Check if the charset specified inside HTML is valid
            if let Some(encoding) = Encoding::for_label_no_replacement(html_charset.as_bytes()) {
                document_encoding = html_charset;
                dom = html_to_dom(&data, encoding.name().to_string());
            }
        }
    }

    // Use custom base URL if specified, read and use what's in the DOM otherwise
    let custom_base_url: String = options.base_url.clone().unwrap_or("".to_string());
    if custom_base_url.is_empty() {
        // No custom base URL is specified
        // Try to see if document has BASE element
        if let Some(existing_base_url) = get_base_url(&dom.document) {
            base_url = resolve_url(&target_url, &existing_base_url);
        }
    } else {
        // Custom base URL provided
        match Url::parse(&custom_base_url) {
            Ok(parsed_url) => {
                if parsed_url.scheme() == "file" {
                    // File base URLs can only work with
                    // documents saved from filesystem
                    if target_url.scheme() == "file" {
                        base_url = parsed_url;
                    }
                } else {
                    base_url = parsed_url;
                }
            }
            Err(_) => {
                // Failed to parse given base URL, perhaps it's a filesystem path?
                if target_url.scheme() == "file" {
                    // Relative paths could work for documents saved from filesystem
                    let path: &Path = Path::new(&custom_base_url);
                    if path.exists() {
                        match Url::from_file_path(fs::canonicalize(path).unwrap()) {
                            Ok(file_url) => {
                                base_url = file_url;
                            }
                            Err(_) => {
                                return Err(MonolithError::new(&format!(
                                    "could not map given path to base URL \"{}\"",
                                    custom_base_url
                                )));
                            }
                        }
                    }
                }
            }
        }
    }

    // Traverse through the document and embed remote assets
    walk_and_embed_assets(cache, &client, &base_url, &dom.document, options);

    // Update or add new BASE element to reroute network requests and hash-links
    if let Some(new_base_url) = options.base_url.clone() {
        dom = set_base_url(&dom.document, new_base_url);
    }

    // Request and embed /favicon.ico (unless it's already linked in the document)
    if !options.no_images
        && (target_url.scheme() == "http" || target_url.scheme() == "https")
        && !has_favicon(&dom.document)
    {
        let favicon_ico_url: Url = resolve_url(&base_url, "/favicon.ico");

        match retrieve_asset(cache, &client, &target_url, &favicon_ico_url, options) {
            Ok((data, final_url, media_type, charset)) => {
                let favicon_data_url: Url =
                    create_data_url(&media_type, &charset, &data, &final_url);
                dom = add_favicon(&dom.document, favicon_data_url.to_string());
            }
            Err(_) => {
                // Failed to retrieve /favicon.ico
            }
        }
    }

    // Save using specified charset, if given
    if let Some(custom_encoding) = options.encoding.clone() {
        document_encoding = custom_encoding;
        dom = set_charset(dom, document_encoding.clone());
    }

    if options.output_format == MonolithOutputFormat::HTML {
        // Serialize DOM tree
        let mut result: Vec<u8> = serialize_document(dom, document_encoding, options);

        // Prepend metadata comment tag
        if !options.no_metadata {
            let mut metadata_comment: String = create_metadata_tag(&target_url);
            metadata_comment += "\n";
            result.splice(0..0, metadata_comment.as_bytes().to_vec());
        }

        Ok(result)
    } else {
        Ok(vec![])
    }
}

pub fn detect_media_type(data: &[u8], url: &Url) -> String {
    // At first attempt to read file's header
    for file_signature in FILE_SIGNATURES.iter() {
        if data.starts_with(file_signature[0]) {
            return String::from_utf8(file_signature[1].to_vec()).unwrap();
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

    let domain_partials: Vec<&str> = domain.trim_end_matches(".").rsplit(".").collect();
    let domain_to_match_against_partials: Vec<&str> = domain_to_match_against
        .trim_end_matches(".")
        .rsplit(".")
        .collect();
    let domain_to_match_against_starts_with_a_dot = domain_to_match_against.starts_with(".");

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
            if !item.trim().is_empty() {
                media_type = item.trim().to_string();
            }
        } else if item.trim().eq_ignore_ascii_case("base64") {
            is_base64 = true;
        } else if item.trim().starts_with("charset=") {
            charset = item.trim().chars().skip(8).collect();
        }

        i += 1;
    }

    (media_type, charset, is_base64)
}

pub fn retrieve_asset(
    cache: &mut Option<Cache>,
    client: &Client,
    parent_url: &Url,
    url: &Url,
    options: &Options,
) -> Result<(Vec<u8>, Url, String, String), reqwest::Error> {
    if url.scheme() == "data" {
        let (media_type, charset, data) = parse_data_url(url);
        Ok((data, url.clone(), media_type, charset))
    } else if url.scheme() == "file" {
        // Check if parent_url is also a file: URL (if not, then we don't embed the asset)
        if parent_url.scheme() != "file" {
            if !options.silent {
                eprintln!(
                    "{}{} (Security Error){}",
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
            client.get("").send()?;
        }

        let path_buf: PathBuf = url.to_file_path().unwrap().clone();
        let path: &Path = path_buf.as_path();
        if path.exists() {
            if path.is_dir() {
                if !options.silent {
                    eprintln!(
                        "{}{} (is a directory){}",
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
                    eprintln!("{}", &url);
                }

                let file_blob: Vec<u8> = fs::read(path).expect("Unable to read file");

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
                    "{}{} (not found){}",
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

        if cache.is_some() && cache.as_ref().unwrap().contains_key(&cache_key) {
            // URL is in cache, we get and return it
            if !options.silent {
                eprintln!("{} (from cache)", &url);
            }

            Ok((
                cache.as_ref().unwrap().get(&cache_key).unwrap().0.to_vec(),
                url.clone(),
                cache.as_ref().unwrap().get(&cache_key).unwrap().1,
                cache.as_ref().unwrap().get(&cache_key).unwrap().2,
            ))
        } else {
            if let Some(domains) = &options.domains {
                let domain_matches = domains
                    .iter()
                    .any(|d| domain_is_within_domain(url.host_str().unwrap(), d.trim()));
                if (options.blacklist_domains && domain_matches)
                    || (!options.blacklist_domains && !domain_matches)
                {
                    return Err(client.get("").send().unwrap_err());
                }
            }

            // URL not in cache, we retrieve the file
            let mut headers = HeaderMap::new();
            if !options.cookies.is_empty() {
                for cookie in &options.cookies {
                    if !cookie.is_expired() && cookie.matches_url(url.as_str()) {
                        let cookie_header_value: String = cookie.name.clone() + "=" + &cookie.value;
                        headers
                            .insert(COOKIE, HeaderValue::from_str(&cookie_header_value).unwrap());
                    }
                }
            }
            // Add referer header for page resource requests
            if ["https", "http"].contains(&parent_url.scheme()) && parent_url != url {
                headers.insert(
                    REFERER,
                    HeaderValue::from_str(get_referer_url(parent_url.clone()).as_str()).unwrap(),
                );
            }
            match client.get(url.as_str()).headers(headers).send() {
                Ok(response) => {
                    if !options.ignore_errors && response.status() != reqwest::StatusCode::OK {
                        if !options.silent {
                            eprintln!(
                                "{}{} ({}){}",
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
                            eprintln!("{}", &url);
                        } else {
                            eprintln!("{} -> {}", &url, &response_url);
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
                                    "{}{}{}",
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
                    if cache.is_some() {
                        cache.as_mut().unwrap().set(
                            &new_cache_key,
                            &data,
                            media_type.clone(),
                            charset.clone(),
                        );
                    }

                    // Return
                    Ok((data, response_url, media_type, charset))
                }
                Err(error) => {
                    if !options.silent {
                        eprintln!(
                            "{}{} ({}){}",
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

pub fn read_stdin() -> Vec<u8> {
    let mut buffer: Vec<u8> = vec![];

    match io::stdin().lock().read_to_end(&mut buffer) {
        Ok(_) => buffer,
        Err(_) => buffer,
    }
}
