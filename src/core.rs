use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use chrono::{SecondsFormat, Utc};
use encoding_rs::Encoding;
use markup5ever_rcdom::RcDom;
use url::Url;

use crate::html::{
    add_favicon, create_metadata_tag, get_base_url, get_charset, get_robots, get_title,
    has_favicon, html_to_dom, serialize_document, set_base_url, set_charset, set_robots, walk,
};
use crate::session::Session;
use crate::url::{create_data_url, resolve_url};

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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum MonolithOutputFormat {
    #[default]
    HTML,
    MHTML,
    // WARC,
    // ZIM,
    // HAR,
}

#[derive(Default)]
pub struct MonolithOptions {
    pub base_url: Option<String>,
    pub blacklist_domains: bool,
    pub domains: Option<Vec<String>>,
    pub encoding: Option<String>,
    pub ignore_errors: bool,
    pub insecure: bool,
    pub isolate: bool,
    pub no_audio: bool,
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
// All known non-"text/..." plaintext media types
const PLAINTEXT_MEDIA_TYPES: &[&str] = &[
    "application/javascript",          // .js
    "application/json",                // .json
    "application/ld+json",             // .jsonld
    "application/x-sh",                // .sh
    "application/xhtml+xml",           // .xhtml
    "application/xml",                 // .xml
    "application/vnd.mozilla.xul+xml", // .xul
    "image/svg+xml",                   // .svg
];

pub fn create_monolithic_document_from_data(
    mut session: Session,
    input_data: Vec<u8>,
    input_encoding: Option<String>,
    input_target: Option<String>,
) -> Result<(Vec<u8>, Option<String>), MonolithError> {
    // Validate options
    {
        // Check if custom encoding value is acceptable
        if let Some(custom_output_encoding) = session.options.encoding.clone() {
            if Encoding::for_label_no_replacement(custom_output_encoding.as_bytes()).is_none() {
                return Err(MonolithError::new(&format!(
                    "unknown encoding \"{}\"",
                    &custom_output_encoding
                )));
            }
        }
    }

    let mut base_url: Url = if input_target.is_some() {
        Url::parse(&input_target.clone().unwrap()).unwrap()
    } else {
        Url::parse("data:text/html,").unwrap()
    };
    let mut document_encoding: String = input_encoding.clone().unwrap_or("utf-8".to_string());
    let mut dom: RcDom;

    // Initial parse
    dom = html_to_dom(&input_data, document_encoding.clone());

    // Attempt to determine document's encoding
    if let Some(html_charset) = get_charset(&dom.document) {
        if !html_charset.is_empty() {
            // Check if the charset specified inside HTML is valid
            if let Some(document_charset) =
                Encoding::for_label_no_replacement(html_charset.as_bytes())
            {
                document_encoding = html_charset;
                dom = html_to_dom(&input_data, document_charset.name().to_string());
            }
        }
    }

    // Use custom base URL if specified; read and use what's in the DOM otherwise
    let custom_base_url: String = session.options.base_url.clone().unwrap_or_default();
    if custom_base_url.is_empty() {
        // No custom base URL is specified; try to see if document has BASE element
        if let Some(existing_base_url) = get_base_url(&dom.document) {
            base_url = resolve_url(&base_url, &existing_base_url);
        }
    } else {
        // Custom base URL provided
        match Url::parse(&custom_base_url) {
            Ok(parsed_url) => {
                if parsed_url.scheme() == "file" {
                    // File base URLs can only work with documents saved from filesystem
                    if base_url.scheme() == "file" {
                        base_url = parsed_url;
                    }
                } else {
                    base_url = parsed_url;
                }
            }
            Err(_) => {
                // Failed to parse given base URL, perhaps it's a filesystem path?
                if base_url.scheme() == "file" {
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
    walk(&mut session, &base_url, &dom.document);

    // Update or add new BASE element to reroute network requests and hash-links
    if let Some(new_base_url) = session.options.base_url.clone() {
        dom = set_base_url(&dom.document, new_base_url);
    }

    // Request and embed /favicon.ico (unless it's already linked in the document)
    if !session.options.no_images
        && (base_url.scheme() == "http" || base_url.scheme() == "https")
        && (input_target.is_some()
            && (input_target.as_ref().unwrap().starts_with("http:")
                || input_target.as_ref().unwrap().starts_with("https:")))
        && !has_favicon(&dom.document)
    {
        let favicon_ico_url: Url = resolve_url(&base_url, "/favicon.ico");

        match session.retrieve_asset(/*&target_url, */ &base_url, &favicon_ico_url) {
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

    // Append noindex META-tag
    let meta_robots_content_value = get_robots(&dom.document).unwrap_or_default();
    if meta_robots_content_value.trim().is_empty() || meta_robots_content_value != "none" {
        dom = set_robots(dom, "none");
    }

    // Save using specified charset, if given
    if let Some(custom_encoding) = session.options.encoding.clone() {
        document_encoding = custom_encoding;
        dom = set_charset(dom, document_encoding.clone());
    }

    let document_title: Option<String> = get_title(&dom.document);

    if session.options.output_format == MonolithOutputFormat::HTML {
        // Serialize DOM tree
        let mut result: Vec<u8> = serialize_document(dom, document_encoding, &session.options);

        // Prepend metadata comment tag
        if !session.options.no_metadata && !input_target.clone().unwrap_or_default().is_empty() {
            let mut metadata_comment: String =
                create_metadata_tag(&Url::parse(&input_target.unwrap_or_default()).unwrap());
            // let mut metadata_comment: String = create_metadata_tag(target);
            metadata_comment += "\n";
            result.splice(0..0, metadata_comment.as_bytes().to_vec());
        }

        // Ensure newline at end of result
        if result.last() != Some(&b"\n"[0]) {
            result.extend_from_slice(b"\n");
        }

        Ok((result, document_title))
    } else if session.options.output_format == MonolithOutputFormat::MHTML {
        // Serialize DOM tree
        let mut result: Vec<u8> = serialize_document(dom, document_encoding, &session.options);

        // Prepend metadata comment tag
        if !session.options.no_metadata && !input_target.clone().unwrap_or_default().is_empty() {
            let mut metadata_comment: String =
                create_metadata_tag(&Url::parse(&input_target.unwrap_or_default()).unwrap());
            // let mut metadata_comment: String = create_metadata_tag(target);
            metadata_comment += "\n";
            result.splice(0..0, metadata_comment.as_bytes().to_vec());
        }

        // Extremely hacky way to convert output to MIME
        let mime = "MIME-Version: 1.0\r\n\
Content-Type: multipart/related; boundary=\"----=_NextPart_000_0000\"\r\n\
\r\n\
------=_NextPart_000_0000\r\n\
Content-Type: text/html; charset=\"utf-8\"\r\n\
Content-Location: http://example.com/\r\n\
\r\n";

        result.splice(0..0, mime.as_bytes().to_vec());

        let mime = "\r\n------=_NextPart_000_0000--\r\n";

        result.extend_from_slice(mime.as_bytes());

        Ok((result, document_title))
    } else {
        Ok((vec![], document_title))
    }
}

pub fn create_monolithic_document(
    mut session: Session,
    target: String,
) -> Result<(Vec<u8>, Option<String>), MonolithError> {
    // Check if target was provided
    if target.is_empty() {
        return Err(MonolithError::new("no target specified"));
    }

    // Validate options
    {
        // Check if custom encoding value is acceptable
        if let Some(custom_encoding) = session.options.encoding.clone() {
            if Encoding::for_label_no_replacement(custom_encoding.as_bytes()).is_none() {
                return Err(MonolithError::new(&format!(
                    "unknown encoding \"{}\"",
                    &custom_encoding
                )));
            }
        }
    }

    let mut target_url = match target.as_str() {
        target_str => match Url::parse(target_str) {
            Ok(target_url) => match target_url.scheme() {
                "data" | "file" | "http" | "https" => target_url,
                unsupported_scheme => {
                    return Err(MonolithError::new(&format!(
                        "unsupported target URL scheme \"{}\"",
                        unsupported_scheme
                    )));
                }
            },
            Err(_) => {
                // Failed to parse given base URL (perhaps it's a filesystem path?)
                let path: &Path = Path::new(&target_str);

                match path.exists() {
                    true => match path.is_file() {
                        true => {
                            let canonical_path = fs::canonicalize(path).unwrap();

                            match Url::from_file_path(canonical_path) {
                                Ok(url) => url,
                                Err(_) => {
                                    return Err(MonolithError::new(&format!(
                                        "could not generate file URL out of given path \"{}\"",
                                        &target_str
                                    )));
                                }
                            }
                        }
                        false => {
                            return Err(MonolithError::new(&format!(
                                "local target \"{}\" is not a file",
                                &target_str
                            )));
                        }
                    },
                    false => {
                        // It is not a FS path, now we do what browsers do:
                        // prepend "http://" and hope it points to a website
                        Url::parse(&format!("http://{}", &target_str)).unwrap()
                    }
                }
            }
        },
    };

    let data: Vec<u8>;
    let document_encoding: Option<String>;

    // Retrieve target document
    if target_url.scheme() == "file"
        || target_url.scheme() == "http"
        || target_url.scheme() == "https"
        || target_url.scheme() == "data"
    {
        match session.retrieve_asset(&target_url, &target_url) {
            Ok((retrieved_data, final_url, media_type, charset)) => {
                if !media_type.eq_ignore_ascii_case("text/html")
                    && !media_type.eq_ignore_ascii_case("application/xhtml+xml")
                {
                    // Provide output as text (without processing it, the way browsers do)
                    return Ok((retrieved_data, None));
                }

                // If got redirected, set target_url to that
                if final_url != target_url {
                    target_url = final_url.clone();
                }

                data = retrieved_data;
                document_encoding = Some(charset);
            }
            Err(_) => {
                return Err(MonolithError::new("could not retrieve target document"));
            }
        }
    } else {
        return Err(MonolithError::new("unsupported target"));
    }

    create_monolithic_document_from_data(
        session,
        data,
        document_encoding,
        Some(target_url.to_string()),
    )
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
            "js" => "text/javascript",
            "json" => "application/json",
            "jsonld" => "application/ld+json",
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
            "xhtml" => "application/xhtml+xml",
            "xml" => "text/xml",
            &_ => "",
        },
        None => "",
    };
    mime.to_string()
}

pub fn format_output_path(
    path: &str,
    document_title: &str,
    output_format: MonolithOutputFormat,
) -> String {
    let datetime: &str = &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    path.replace("%timestamp%", &datetime.replace(':', "_"))
        .replace(
            "%title%",
            document_title
                .to_string()
                .replace(['/', '\\'], "_")
                .replace('<', "[")
                .replace('>', "]")
                .replace(':', " - ")
                .replace('\"', "")
                .replace('|', "-")
                .replace('?', "")
                .trim_start_matches('.'),
        )
        .replace(
            "%ext%",
            if output_format == MonolithOutputFormat::HTML {
                "htm"
            } else if output_format == MonolithOutputFormat::MHTML {
                "mht"
            } else {
                ""
            },
        )
        .replace(
            "%extension%",
            if output_format == MonolithOutputFormat::HTML {
                "html"
            } else if output_format == MonolithOutputFormat::MHTML {
                "mhtml"
            } else {
                ""
            },
        )
        .to_string()
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

pub fn print_error_message(text: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    const ENV_VAR_NO_COLOR: &str = "NO_COLOR";
    const ENV_VAR_TERM: &str = "TERM";

    let mut no_color = env::var_os(ENV_VAR_NO_COLOR).is_some() || atty::isnt(atty::Stream::Stderr);
    if let Some(term) = env::var_os(ENV_VAR_TERM) {
        if term == "dumb" {
            no_color = true;
        }
    }

    if handle
        .write_all(
            format!(
                "{}{}{}\n",
                if no_color { "" } else { ANSI_COLOR_RED },
                &text,
                if no_color { "" } else { ANSI_COLOR_RESET },
            )
            .as_bytes(),
        )
        .is_ok()
    {}
}

pub fn print_info_message(text: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    if handle.write_all(format!("{}\n", &text).as_bytes()).is_ok() {}
}
