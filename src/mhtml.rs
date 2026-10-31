//! MHTML (MIME HTML, RFC 2557): writing documents as MHTML archives and reading them back.
//!
//! An MHTML archive is a multipart/related MIME message: the first (or "start") part is the document,
//! every other part is one of its assets, named by Content-Location (its URL) and/or Content-ID (cid: URL).
//! Archives saved by Chromium-based browsers, Internet Explorer, Word, Firefox add-ons and monolith itself
//! all look like that, they just differ in which headers and transfer encodings they use.

use std::collections::HashMap;

use base64::alphabet;
use base64::engine::general_purpose::{GeneralPurpose, PAD_INDIFFERENT};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::Utc;
use url::Url;

use crate::core::{detect_media_type, is_plaintext_media_type};
use crate::session::Session;
use crate::url::clean_url;

/// Boundary between parts of MHTML documents monolith writes.
/// Bodies get encoded in a way that never has a line start with "-", so it can't collide with anything.
pub const MHTML_BOUNDARY: &str = "----MultipartBoundary--monolith----";

// Max length of encoded lines, as per RFC 2045 (not counting CRLF)
const MAX_LINE_LENGTH: usize = 76;

// Base64 encoders out there aren't always strict about padding and trailing bits, readers shouldn't be either
const BASE64_LENIENT: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    PAD_INDIFFERENT.with_decode_allow_trailing_bits(true),
);

/// MIME headers (names and values), in order of appearance
pub type Headers = Vec<(String, String)>;

/// A single part of an MHTML archive, already decoded.
pub struct ArchivedAsset {
    pub data: Vec<u8>,
    /// Content-Location of the part, resolved into a full URL (None if the part doesn't have one)
    pub url: Option<Url>,
    pub media_type: String,
    /// Empty if not specified
    pub charset: String,
}

pub struct MhtmlArchive {
    assets: Vec<ArchivedAsset>,
    /// Lookup table: URL (without fragment) or "cid:<Content-ID>" -> index in assets
    index: HashMap<String, usize>,
    root: usize,
}

impl MhtmlArchive {
    /// The document itself.
    pub fn root(&self) -> &ArchivedAsset {
        &self.assets[self.root]
    }

    pub fn get(&self, url: &Url) -> Option<&ArchivedAsset> {
        self.index.get(&archive_key(url)).map(|i| &self.assets[*i])
    }

    pub fn contains(&self, url: &Url) -> bool {
        self.index.contains_key(&archive_key(url))
    }

    /// Number of parts in the archive (the document included).
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

fn archive_key(url: &Url) -> String {
    if url.scheme().eq_ignore_ascii_case("cid") {
        // RFC 2392: cid: URLs are percent-encoded Content-IDs
        format!("cid:{}", percent_decode(url.path()))
    } else {
        clean_url(url.clone()).as_str().to_string()
    }
}

/// Checks if the data looks like an MHTML archive (a MIME message with multipart/related or multipart/mixed content).
pub fn is_mhtml(data: &[u8]) -> bool {
    match parse_headers(data, true) {
        Some((headers, _)) => multipart_boundary(&headers).is_some(),
        None => false,
    }
}

/// Unpacks an MHTML archive.
/// Relative Content-Location values get resolved against the archive's own location (if it has one), or fallback_url.
/// Returns None if the data isn't an MHTML archive, or if there's nothing in it.
pub fn parse_mhtml(data: &[u8], fallback_url: Option<&Url>) -> Option<MhtmlArchive> {
    let (headers, body) = parse_headers(data, true)?;
    let (boundary, start) = multipart_boundary(&headers)?;

    // Base URL for relative Content-Location values
    let base_url: Option<Url> = [
        "content-location",
        "content-base",
        "snapshot-content-location",
    ]
    .iter()
    .filter_map(|name| get_header(&headers, name))
    .find_map(|value| Url::parse(value.trim()).ok())
    .or_else(|| fallback_url.cloned());

    let mut archive = MhtmlArchive {
        assets: Vec::new(),
        index: HashMap::new(),
        root: 0,
    };
    let mut root: Option<usize> = None;

    for part in split_multipart(body, &boundary) {
        let (part_headers, part_body) = parse_headers(part, false).unwrap_or_default();

        let (mut media_type, params) =
            parse_content_type(get_header(&part_headers, "content-type").unwrap_or_default());
        let charset: String = params.get("charset").cloned().unwrap_or_default();

        let data: Vec<u8> = match get_header(&part_headers, "content-transfer-encoding")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "base64" => decode_base64(part_body),
            "quoted-printable" => decode_quoted_printable(part_body),
            _ => part_body.to_vec(), // 7bit, 8bit, binary
        };

        let url: Option<Url> = get_header(&part_headers, "content-location").and_then(|location| {
            let location = location.trim();
            Url::parse(location).ok().or_else(|| {
                base_url
                    .as_ref()
                    .and_then(|base_url| base_url.join(location).ok())
            })
        });

        let content_id: Option<String> = get_header(&part_headers, "content-id")
            .map(|id| {
                id.trim()
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string()
            })
            .filter(|id| !id.is_empty());

        if media_type.is_empty() {
            media_type = match &url {
                Some(url) => detect_media_type(&data, url),
                None => detect_media_type(&data, &Url::parse("data:,").unwrap()),
            };
        }

        let i: usize = archive.assets.len();

        // Parts that come first win, in case there are duplicates
        if let Some(url) = &url
            && url.scheme() != "data"
        {
            archive.index.entry(archive_key(url)).or_insert(i);
        }
        if let Some(content_id) = &content_id {
            archive
                .index
                .entry(format!("cid:{}", content_id))
                .or_insert(i);

            if root.is_none() && start.as_deref() == Some(content_id.as_str()) {
                root = Some(i);
            }
        }

        archive.assets.push(ArchivedAsset {
            data,
            url,
            media_type,
            charset,
        });
    }

    if archive.assets.is_empty() {
        return None;
    }

    // Without the "start" parameter it's the first part, unless that one isn't a document (and some other part is)
    archive.root = root.unwrap_or_else(|| {
        archive
            .assets
            .iter()
            .position(|asset| is_html_media_type(&asset.media_type))
            .unwrap_or(0)
    });

    Some(archive)
}

/// Packs the document and all assets it references into an MHTML archive.
/// The document's assets must already have been logged by the session (see Session::log_asset_url).
pub fn create_mhtml_document(
    session: &mut Session,
    html: Vec<u8>,
    document_url: &Url,
    document_location: Option<&Url>,
    document_charset: &str,
    document_title: Option<&str>,
) -> Vec<u8> {
    let mut mhtml: Vec<u8> = Vec::with_capacity(html.len() * 2);

    // Message headers
    if !session.options.no_metadata {
        mhtml.extend_from_slice(b"From: <Saved by Monolith>\r\n");
    }
    if let Some(location) = document_location {
        push_header(&mut mhtml, "Snapshot-Content-Location", location.as_str());
    }
    if let Some(title) = document_title.filter(|title| !title.trim().is_empty()) {
        push_header(&mut mhtml, "Subject", &encode_header_value(title));
    }
    if !session.options.no_metadata {
        push_header(&mut mhtml, "Date", &Utc::now().to_rfc2822());
    }
    mhtml.extend_from_slice(b"MIME-Version: 1.0\r\n");
    mhtml.extend_from_slice(
        format!(
            "Content-Type: multipart/related;\r\n\ttype=\"text/html\";\r\n\tboundary=\"{}\"\r\n\r\n",
            MHTML_BOUNDARY
        )
        .as_bytes(),
    );

    // Document
    push_part(
        &mut mhtml,
        "text/html",
        document_charset,
        document_location,
        &html,
    );

    // Assets
    let document_key: String = clean_url(document_url.clone()).to_string();
    let location_key: Option<String> =
        document_location.map(|url| clean_url(url.clone()).to_string());
    let mut packed: Vec<String> = Vec::new();

    for asset_url in session.asset_urls.clone().iter() {
        let asset_url: Url = match Url::parse(asset_url) {
            Ok(asset_url) => clean_url(asset_url),
            Err(_) => continue,
        };

        // Data URLs carry their own contents; the document isn't an asset of itself
        if asset_url.scheme() == "data"
            || asset_url.as_str() == document_key
            || Some(asset_url.as_str()) == location_key.as_deref()
            || packed.iter().any(|url| url == asset_url.as_str())
        {
            continue;
        }

        // Errors are already reported by retrieve_asset(); assets that couldn't be retrieved
        // remain referenced by their full URLs in the document, the way they do in HTML output
        if let Ok((data, _final_url, media_type, charset)) =
            session.retrieve_asset(document_url, &asset_url)
        {
            push_part(&mut mhtml, &media_type, &charset, Some(&asset_url), &data);
            packed.push(asset_url.to_string());
        }
    }

    // End of MIME message
    mhtml.extend_from_slice(format!("--{}--\r\n", MHTML_BOUNDARY).as_bytes());

    mhtml
}

fn push_header(mhtml: &mut Vec<u8>, name: &str, value: &str) {
    mhtml.extend_from_slice(format!("{}: {}\r\n", name, value).as_bytes());
}

fn push_part(mhtml: &mut Vec<u8>, media_type: &str, charset: &str, url: Option<&Url>, data: &[u8]) {
    let is_text: bool = is_plaintext_media_type(media_type);

    mhtml.extend_from_slice(format!("--{}\r\n", MHTML_BOUNDARY).as_bytes());

    // Headers can't have line breaks in them
    let media_type: String = media_type.replace(['\r', '\n'], "");
    let charset: String = charset.replace(['\r', '\n', '"', '\\'], "");
    if is_text && !charset.is_empty() {
        push_header(
            mhtml,
            "Content-Type",
            &format!("{}; charset=\"{}\"", media_type, charset),
        );
    } else if !media_type.is_empty() {
        push_header(mhtml, "Content-Type", &media_type);
    }

    push_header(
        mhtml,
        "Content-Transfer-Encoding",
        if is_text {
            "quoted-printable"
        } else {
            "base64"
        },
    );

    if let Some(url) = url {
        if url.scheme().eq_ignore_ascii_case("cid") {
            push_header(
                mhtml,
                "Content-ID",
                &format!("<{}>", percent_decode(url.path())),
            );
        } else {
            push_header(mhtml, "Content-Location", url.as_str());
        }
    }

    mhtml.extend_from_slice(b"\r\n");
    if is_text {
        mhtml.extend_from_slice(encode_quoted_printable(data).as_bytes());
    } else {
        mhtml.extend_from_slice(encode_base64(data).as_bytes());
    }
    // The line break before the next boundary belongs to the boundary, not the part's contents
    mhtml.extend_from_slice(b"\r\n");
}

fn is_html_media_type(media_type: &str) -> bool {
    media_type.eq_ignore_ascii_case("text/html")
        || media_type.eq_ignore_ascii_case("application/xhtml+xml")
}

/// Returns the multipart boundary and the Content-ID of the root part ("start" parameter, if given).
fn multipart_boundary(headers: &Headers) -> Option<(String, Option<String>)> {
    let (media_type, params) = parse_content_type(get_header(headers, "content-type")?);

    if media_type != "multipart/related" && media_type != "multipart/mixed" {
        return None;
    }

    let boundary: String = params.get("boundary").filter(|b| !b.is_empty())?.clone();
    let start: Option<String> = params.get("start").map(|start| {
        start
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_string()
    });

    Some((boundary, start))
}

fn get_header<'a>(headers: &'a Headers, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// Splits a MIME entity into its headers and body.
/// When strict, returns None unless the data begins with a valid header (that's what tells MIME messages from HTML).
pub fn parse_headers(data: &[u8], strict: bool) -> Option<(Headers, &[u8])> {
    let mut headers: Headers = Vec::new();
    let mut pos: usize = 0;

    // Skip UTF-8 BOM
    if data.starts_with(b"\xEF\xBB\xBF") {
        pos = 3;
    }

    while pos < data.len() {
        let line_end: usize = data[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|i| pos + i)
            .unwrap_or(data.len());
        let next: usize = (line_end + 1).min(data.len());
        let line: &[u8] = data[pos..line_end]
            .strip_suffix(b"\r")
            .unwrap_or(&data[pos..line_end]);

        if line.is_empty() {
            // End of headers
            return Some((headers, &data[next..]));
        }

        if line[0] == b' ' || line[0] == b'\t' {
            // Folded header: continuation of the previous one
            match headers.last_mut() {
                Some((_, value)) => value.push_str(&String::from_utf8_lossy(line)),
                None if strict => return None,
                None => {}
            }
        } else {
            let colon: Option<usize> = line.iter().position(|&b| b == b':');
            let is_valid_name = |name: &[u8]| {
                !name.is_empty() && name.iter().all(|b| (33..=126).contains(b) && *b != b':')
            };

            match colon {
                Some(colon) if is_valid_name(&line[..colon]) => {
                    headers.push((
                        String::from_utf8_lossy(&line[..colon]).to_string(),
                        String::from_utf8_lossy(&line[colon + 1..])
                            .trim()
                            .to_string(),
                    ));
                }
                _ if strict && headers.is_empty() => return None,
                _ => {} // Skip malformed lines
            }
        }

        pos = next;
    }

    // Headers only, no body
    if strict && headers.is_empty() {
        None
    } else {
        Some((headers, &data[data.len()..]))
    }
}

/// Parses Content-Type header values: returns the media type (lowercase), and parameters (with lowercase names).
pub fn parse_content_type(value: &str) -> (String, HashMap<String, String>) {
    let mut params: HashMap<String, String> = HashMap::new();
    let chars: Vec<char> = value.chars().collect();
    let mut i: usize = 0;

    let mut media_type = String::new();
    while i < chars.len() && chars[i] != ';' {
        media_type.push(chars[i]);
        i += 1;
    }

    while i < chars.len() {
        i += 1; // Skip ";"

        let mut name = String::new();
        while i < chars.len() && chars[i] != '=' && chars[i] != ';' {
            name.push(chars[i]);
            i += 1;
        }
        if i >= chars.len() || chars[i] == ';' {
            continue; // Parameter without value
        }
        i += 1; // Skip "="
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        let mut value = String::new();
        if i < chars.len() && chars[i] == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                value.push(chars[i]);
                i += 1;
            }
            // Skip whatever is between the closing quote and the next parameter
            while i < chars.len() && chars[i] != ';' {
                i += 1;
            }
        } else {
            while i < chars.len() && chars[i] != ';' {
                value.push(chars[i]);
                i += 1;
            }
            value = value.trim().to_string();
        }

        let name: String = name.trim().to_ascii_lowercase();
        if !name.is_empty() {
            params.entry(name).or_insert(value);
        }
    }

    (media_type.trim().to_ascii_lowercase(), params)
}

/// Splits the body of a multipart entity into parts (each one still has its headers).
pub fn split_multipart<'a>(body: &'a [u8], boundary: &str) -> Vec<&'a [u8]> {
    let delimiter: Vec<u8> = format!("--{}", boundary).into_bytes();
    let mut parts: Vec<&[u8]> = Vec::new();
    let mut part_start: Option<usize> = None;
    let mut pos: usize = 0;

    while pos < body.len() {
        let line_end: usize = body[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|i| pos + i)
            .unwrap_or(body.len());
        let line: &[u8] = &body[pos..line_end];

        if let Some(rest) = line.strip_prefix(delimiter.as_slice()) {
            // Delimiter lines may have whitespace ("transport padding") after them
            let rest: &[u8] = rest.trim_ascii();
            let is_last: bool = rest.starts_with(b"--");

            if rest.is_empty() || is_last {
                if let Some(start) = part_start {
                    // The line break before the delimiter is part of the delimiter
                    let mut end: usize = pos;
                    if end > start && body[end - 1] == b'\n' {
                        end -= 1;
                        if end > start && body[end - 1] == b'\r' {
                            end -= 1;
                        }
                    }
                    parts.push(&body[start..end]);
                }

                if is_last {
                    return parts;
                }

                part_start = Some((line_end + 1).min(body.len()));
            }
        }

        pos = line_end + 1;
    }

    // Truncated archive (no closing delimiter): keep what's there
    if let Some(start) = part_start {
        parts.push(&body[start..]);
    }

    parts
}

pub fn decode_base64(data: &[u8]) -> Vec<u8> {
    let data: Vec<u8> = data
        .iter()
        .filter(|b| !b.is_ascii_whitespace())
        .copied()
        .collect();

    BASE64_LENIENT.decode(&data).unwrap_or_default()
}

pub fn encode_base64(data: &[u8]) -> String {
    let encoded: String = BASE64_STANDARD.encode(data);
    let mut result: String =
        String::with_capacity(encoded.len() + encoded.len() / MAX_LINE_LENGTH * 2);

    // Base64 is pure ASCII, so it's safe to split it at any byte
    for (i, line) in encoded.as_bytes().chunks(MAX_LINE_LENGTH).enumerate() {
        if i > 0 {
            result.push_str("\r\n");
        }
        result.push_str(std::str::from_utf8(line).unwrap());
    }

    result
}

pub fn decode_quoted_printable(data: &[u8]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(data.len());
    let mut i: usize = 0;

    while i < data.len() {
        if data[i] != b'=' {
            result.push(data[i]);
            i += 1;
            continue;
        }

        // Soft line break: "=", optionally followed by whitespace, then end of line
        let mut j: usize = i + 1;
        while j < data.len() && (data[j] == b' ' || data[j] == b'\t') {
            j += 1;
        }
        if j >= data.len() {
            i = j;
            continue;
        }
        if data[j] == b'\n' {
            i = j + 1;
            continue;
        }
        if data[j] == b'\r' && data.get(j + 1) == Some(&b'\n') {
            i = j + 2;
            continue;
        }

        // Encoded byte
        if let Some(byte) = decode_hex_escape(data, i) {
            result.push(byte);
            i += 3;
            continue;
        }

        // Not valid quoted-printable, keep as is
        result.push(b'=');
        i += 1;
    }

    result
}

/// Encodes data as quoted-printable, preserving it byte for byte: CRLFs become hard line breaks,
/// lone LFs get encoded (followed by a soft line break, to keep the output readable).
/// Lines never begin with "-" or ".", so the output can't be mistaken for a MIME boundary (or SMTP terminator).
pub fn encode_quoted_printable(data: &[u8]) -> String {
    let mut result: String = String::with_capacity(data.len() + data.len() / 8);
    let mut line_length: usize = 0;
    let mut i: usize = 0;

    let is_literal = |b: u8, line_length: usize, is_before_line_end: bool| match b {
        b' ' | b'\t' => !is_before_line_end, // Trailing whitespace may get stripped in transit
        b'-' | b'.' => line_length > 0,
        b'=' => false,
        33..=126 => true,
        _ => false,
    };

    while i < data.len() {
        let b: u8 = data[i];

        if b == b'\r' && data.get(i + 1) == Some(&b'\n') {
            result.push_str("\r\n");
            line_length = 0;
            i += 2;
            continue;
        }

        let is_before_line_end: bool =
            i + 1 == data.len() || (data[i + 1] == b'\r' && data.get(i + 2) == Some(&b'\n'));
        let mut literal: bool = is_literal(b, line_length, is_before_line_end);

        // One spot is reserved for the soft line break's "="
        if line_length + if literal { 1 } else { 3 } > MAX_LINE_LENGTH - 1 {
            result.push_str("=\r\n");
            line_length = 0;
            literal = is_literal(b, line_length, is_before_line_end);
        }

        if literal {
            result.push(b as char);
            line_length += 1;
        } else {
            result.push_str(&format!("={:02X}", b));
            line_length += 3;
        }

        if b == b'\n' && i + 1 < data.len() {
            result.push_str("=\r\n");
            line_length = 0;
        }

        i += 1;
    }

    result
}

/// Makes text usable as a header value (RFC 2047 encoded-words if it's not plain ASCII).
fn encode_header_value(text: &str) -> String {
    // No line breaks or other control characters allowed
    let text: String = text
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|word| !word.is_empty())
        .collect::<Vec<&str>>()
        .join(" ");

    if text.is_ascii() && !text.contains("=?") {
        return text;
    }

    // Encoded-words can't be longer than 75 characters: 45 bytes of text per word fit
    let mut words: Vec<String> = Vec::new();
    let mut chunk = String::new();
    for c in text.chars() {
        if chunk.len() + c.len_utf8() > 45 {
            words.push(format!("=?utf-8?B?{}?=", BASE64_STANDARD.encode(&chunk)));
            chunk.clear();
        }
        chunk.push(c);
    }
    if !chunk.is_empty() {
        words.push(format!("=?utf-8?B?{}?=", BASE64_STANDARD.encode(&chunk)));
    }

    words.join("\r\n ")
}

fn percent_decode(text: &str) -> String {
    let bytes: &[u8] = text.as_bytes();
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i: usize = 0;

    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = decode_hex_escape(bytes, i)
        {
            result.push(byte);
            i += 3;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }

    String::from_utf8_lossy(&result).to_string()
}

/// Decodes the two hex digits that follow position i ("=XX" in quoted-printable, "%XX" in URLs).
fn decode_hex_escape(data: &[u8], i: usize) -> Option<u8> {
    let hex_value = |b: &u8| (*b as char).to_digit(16).map(|d| d as u8);
    let high: u8 = hex_value(data.get(i + 1)?)?;
    let low: u8 = hex_value(data.get(i + 2)?)?;

    Some(high << 4 | low)
}
