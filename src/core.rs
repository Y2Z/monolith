use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{SecondsFormat, Utc};
use encoding_rs::Encoding;
use markup5ever_rcdom::RcDom;
use url::Url;

use crate::html::{
    add_favicon, create_metadata_tag, get_base_url, get_charset, get_robots, get_title,
    has_favicon, html_to_dom, serialize_document, set_base_url, set_charset, set_robots, walk,
};
use crate::session::Session;
use crate::url::{create_data_url, is_windows_drive_path, resolve_url};

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
    pub threads: usize,
    pub timeout: u64,
    pub unwrap_noscript: bool,
    pub user_agent: Option<String>,
}

const ANSI_COLOR_RED: &str = "\x1b[31m";
const ANSI_COLOR_RESET: &str = "\x1b[0m";
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
    // A BOM or a charset from the transport layer takes precedence over META
    let authoritative_encoding = Encoding::for_bom(&input_data)
        .map(|(encoding, _)| encoding)
        .or_else(|| input_encoding.as_deref().and_then(declared_encoding));
    let mut document_encoding = authoritative_encoding
        .map(|encoding| encoding.name().to_string())
        .unwrap_or("utf-8".to_string());
    let mut dom: RcDom;

    // Encoding the document declares (if any), used for decoding unlabeled stylesheets
    let mut environment_encoding = authoritative_encoding;

    // Initial parse
    dom = html_to_dom(&input_data, document_encoding.clone());

    // Attempt to determine document's encoding
    if authoritative_encoding.is_none()
        && let Some(html_charset) = get_charset(&dom.document)
    {
        if !html_charset.is_empty() {
            // Check if the charset specified inside HTML is valid
            if let Some(document_charset) =
                Encoding::for_label_no_replacement(html_charset.as_bytes())
            {
                document_encoding = html_charset;
                environment_encoding = Some(document_charset);
                dom = html_to_dom(&input_data, document_charset.name().to_string());
            }
        }
    }

    if authoritative_encoding.is_some() {
        // UTF-16 documents get serialized as UTF-8
        if document_encoding == "UTF-16LE" || document_encoding == "UTF-16BE" {
            document_encoding = "UTF-8".to_string();
        }
        if get_charset(&dom.document).is_some() || !input_data.is_ascii() {
            dom = set_charset(dom, document_encoding.clone());
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
            Ok(parsed_url) if !is_windows_drive_path(&custom_base_url) => {
                if parsed_url.scheme() == "file" {
                    // File base URLs can only work with documents saved from filesystem
                    if base_url.scheme() == "file" {
                        base_url = parsed_url;
                    }
                } else {
                    base_url = parsed_url;
                }
            }
            _ => {
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

    session.prefetch_assets(&base_url, &dom.document);

    // A byte order mark beats any declaration when it comes to decoding the document
    if let Some((bom_encoding, _)) = Encoding::for_bom(&input_data) {
        environment_encoding = Some(bom_encoding);
    }

    // Traverse through the document and embed remote assets
    session.environment_encoding = environment_encoding;
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
                let favicon_data_url: String =
                    create_data_url(&media_type, &charset, data, &final_url, None);
                dom = add_favicon(&dom.document, favicon_data_url);
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
    // Serialize DOM tree
    let mut html: Vec<u8> = serialize_document(dom, document_encoding, &session.options);

    // Prepend metadata comment tag
    if !session.options.no_metadata && !input_target.clone().unwrap_or_default().is_empty() {
        let mut metadata_comment: String =
            create_metadata_tag(&Url::parse(&input_target.unwrap_or_default()).unwrap());
        // let mut metadata_comment: String = create_metadata_tag(target);
        metadata_comment += "\n";
        html.splice(0..0, metadata_comment.as_bytes().to_vec());
    }

    if session.options.output_format == MonolithOutputFormat::MHTML {
        // Outputting in MHTML format

        let mut mhtml: Vec<u8> = vec![];

        let mime = "MIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=\"----=_NextPart_000_0000\"\r\n";
        mhtml.splice(0..0, mime.as_bytes().to_vec());

        // Target document
        let mime = format!(
            "\r\n------=_NextPart_000_0000\r\nContent-Type: {}; charset=\"{}\"\r\nContent-Location: {}\r\n\r\n",
            "text/html",
            "utf-8",
            // input_target.unwrap_or("data:;".to_string())
            "data:;",
        );
        mhtml.extend_from_slice(mime.as_bytes());
        mhtml.append(&mut html);

        // Assets
        for asset_url in session.asset_urls.clone().iter_mut() {
            // TODO: skip if file: (?)

            match session.retrieve_asset(
                &Url::parse(&asset_url).unwrap(),
                &Url::parse(&asset_url).unwrap(),
            ) {
                Ok((retrieved_data, final_url, media_type, _charset)) => {
                    if is_plaintext_media_type(&media_type) {
                        let mime = format!(
                            "\r\n------=_NextPart_000_0000\r\nContent-Type: {}\r\nContent-Transfer-Encoding: 7bit\r\nContent-Location: {}\r\n\r\n",
                            media_type, final_url
                        );
                        mhtml.extend_from_slice(mime.as_bytes());
                        // Append plaintext contents
                        mhtml = vec![mhtml, retrieved_data].concat();
                    } else {
                        let mime = format!(
                            "\r\n------=_NextPart_000_0000\r\nContent-Type: {}\r\nContent-Transfer-Encoding: base64\r\nContent-Location: {}\r\n\r\n",
                            media_type, final_url
                        );
                        mhtml.extend_from_slice(mime.as_bytes());
                        // Append base64-encoded blob
                        mhtml.extend_from_slice(BASE64_STANDARD.encode(retrieved_data).as_bytes());
                    }
                }
                Err(_) => {
                    // TODO: if not silent, print red message
                }
            }
        }

        // End of MIME
        let mime = "\r\n------=_NextPart_000_0000--\r\n";
        mhtml.extend_from_slice(mime.as_bytes());

        // Return
        Ok((mhtml, document_title))
    } else {
        // Outputting in HTML format

        // Ensure newline at end of result
        if html.last() != Some(&b"\n"[0]) {
            html.extend_from_slice(b"\n");
        }

        // Return
        Ok((html, document_title))
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
            Ok(target_url) if !is_windows_drive_path(target_str) => match target_url.scheme() {
                "data" | "file" | "http" | "https" => target_url,
                unsupported_scheme => {
                    return Err(MonolithError::new(&format!(
                        "unsupported target URL scheme \"{}\"",
                        unsupported_scheme
                    )));
                }
            },
            _ => {
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
    // Detect by file signature, falling back to the file name's extension
    let file_name: Option<&str> = url.path().rsplit('/').next();
    dataurl::detect_media_type(data, file_name)
        .unwrap_or_default()
        .to_string()
}

pub fn format_output_path(
    path: &str,
    document_title: &str,
    output_format: MonolithOutputFormat,
) -> String {
    let datetime: &str = &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let title: String = document_title
        .replace(['/', '\\'], "_")
        .replace('<', "[")
        .replace('>', "]")
        .replace(':', " - ")
        .replace('\"', "")
        .replace('|', "-")
        .replace('?', "");
    // Like browsers do when saving a page, turn newlines, tabs and other whitespace
    // or control characters into single spaces, then trim leading/trailing spaces and dots
    let title: String = title
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|word| !word.is_empty())
        .collect::<Vec<&str>>()
        .join(" ");
    let title: &str = title.trim_matches(|c: char| c == '.' || c == ' ');

    path.replace("%timestamp%", &datetime.replace(':', "_"))
        .replace("%title%", title)
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

/// Maps a charset label (Content-Type header, data URL, META element) onto an encoding.
/// Returns None for labels that don't declare anything usable: empty (no charset given) and unknown ones.
/// Like in browsers, "US-ASCII" is a valid label, and means windows-1252.
pub fn declared_encoding(charset: &str) -> Option<&'static Encoding> {
    let charset = charset.trim();

    if charset.is_empty() {
        return None;
    }

    Encoding::for_label_no_replacement(charset.as_bytes())
}

pub fn is_plaintext_media_type(media_type: &str) -> bool {
    media_type.to_lowercase().as_str().starts_with("text/")
        || PLAINTEXT_MEDIA_TYPES.contains(&media_type.to_lowercase().as_str())
}

/// Parses a Content-Type header (or META content), returning its media type, charset (None if not given),
/// and whether it has a base64 marker. Parameters are parsed the way the WHATWG MIME Sniffing Standard
/// does it (https://mimesniff.spec.whatwg.org/#parsing-a-mime-type): quoted values may contain semicolons
/// and escaped characters, whitespace around "=" makes a parameter invalid, and the first valid charset wins.
pub fn parse_media_type(content_type: &str) -> (String, Option<String>, bool) {
    let is_http_whitespace = |c: char| matches!(c, '\t' | '\n' | '\r' | ' ');
    let is_quoted_string_token = |c: char| matches!(c, '\t' | ' '..='~' | '\u{80}'..='\u{ff}');

    let s: &str = content_type.trim_matches(is_http_whitespace);
    let b: &[u8] = s.as_bytes();

    let media_type_end: usize = s.find(';').unwrap_or(s.len());
    let media_type: &str = s[..media_type_end].trim();
    let media_type: String = if media_type.is_empty() {
        "text/plain".to_string()
    } else {
        media_type.to_string()
    };
    let mut charset: Option<String> = None;
    let mut is_base64: bool = false;

    // All delimiters are ASCII, so byte offsets always land on char boundaries
    let mut pos: usize = media_type_end;
    while pos < b.len() {
        // Skip past the semicolon and any whitespace after it
        pos += 1;
        while pos < b.len() && is_http_whitespace(b[pos] as char) {
            pos += 1;
        }

        let name_start: usize = pos;
        while pos < b.len() && b[pos] != b';' && b[pos] != b'=' {
            pos += 1;
        }
        let name: &str = &s[name_start..pos];

        if pos >= b.len() || b[pos] == b';' {
            // A parameter without a value, only meaningful as data URLs' base64 marker
            if name.trim().eq_ignore_ascii_case("base64") {
                is_base64 = true;
            }
            continue;
        }
        pos += 1; // Skip past "="

        let value: String = if pos < b.len() && b[pos] == b'"' {
            // Quoted string: backslash escapes the next character, anything after the closing quote is ignored
            let mut value = String::new();
            pos += 1;
            while let Some(c) = s[pos..].chars().next() {
                pos += c.len_utf8();
                match c {
                    '"' => break,
                    '\\' => match s[pos..].chars().next() {
                        Some(escaped) => {
                            value.push(escaped);
                            pos += escaped.len_utf8();
                        }
                        None => value.push('\\'),
                    },
                    _ => value.push(c),
                }
            }
            while pos < b.len() && b[pos] != b';' {
                pos += 1;
            }
            value
        } else {
            let value_start: usize = pos;
            while pos < b.len() && b[pos] != b';' {
                pos += 1;
            }
            let value: &str = s[value_start..pos].trim_end_matches(is_http_whitespace);
            if value.is_empty() {
                continue;
            }
            value.to_string()
        };

        if charset.is_none()
            && name.eq_ignore_ascii_case("charset")
            && value.chars().all(is_quoted_string_token)
        {
            charset = Some(value);
        }
    }

    (media_type, charset, is_base64)
}

pub fn print_error_message(text: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    const ENV_VAR_NO_COLOR: &str = "NO_COLOR";
    const ENV_VAR_TERM: &str = "TERM";

    let mut no_color = env::var_os(ENV_VAR_NO_COLOR).is_some() || !std::io::stderr().is_terminal();
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
