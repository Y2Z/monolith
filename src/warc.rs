//! WARC (ISO 28500) archives: everything a crawler has exchanged with servers, record by record.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};

use brotli_decompressor::Decompressor;
use chrono::{DateTime, SecondsFormat, Utc};
use flate2::Compression;
use flate2::read::{MultiGzDecoder, ZlibDecoder};
use flate2::write::GzEncoder;
use sha2::{Digest, Sha256};
use url::Url;

use crate::cache::Exchange;
use crate::core::MonolithOutputFormat;
use crate::replay::{
    ArchivedResponse, CapturedPage, HttpDetails, create_page_from_responses, is_html,
};
use crate::session::Session;

const GZIP_SIGNATURE: [u8; 2] = [0x1f, 0x8b];
const WARC_SIGNATURE: &[u8] = b"WARC/";
const WARC_VERSION: &str = "WARC/1.1";
const BASE32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
// What an archive must not give away: who it has been saved as
const PRIVATE_REQUEST_HEADERS: [&str; 3] = ["authorization", "cookie", "proxy-authorization"];
const PRIVATE_RESPONSE_HEADERS: [&str; 2] = ["set-cookie", "set-cookie2"];
// Bodies are kept the way they were before getting encoded for transfer
const TRANSFER_HEADERS: [&str; 3] = ["content-encoding", "content-length", "transfer-encoding"];

type Headers = Vec<(String, String)>;

struct Record<'a> {
    block: &'a [u8],
    headers: Headers,
}

fn find_header<'a>(headers: &'a Headers, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn find_bytes(data: &[u8], needle: &[u8]) -> Option<usize> {
    data.windows(needle.len()).position(|part| part == needle)
}

fn parse_headers(data: &[u8]) -> Headers {
    String::from_utf8_lossy(data)
        .split("\r\n")
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

/// Goes through records one by one, a broken one puts an end to it: nothing after it can be found.
fn parse_records(data: &[u8]) -> Vec<Record<'_>> {
    let mut records = vec![];
    let mut rest = data;

    while let Some(start) = find_bytes(rest, WARC_SIGNATURE) {
        rest = &rest[start..];
        let Some(end_of_headers) = find_bytes(rest, b"\r\n\r\n") else {
            break;
        };
        let headers = parse_headers(&rest[..end_of_headers]);
        let body = &rest[end_of_headers + 4..];
        let Some(length) = find_header(&headers, "Content-Length")
            .and_then(|length| length.parse::<usize>().ok())
            .filter(|length| *length <= body.len())
        else {
            break;
        };

        records.push(Record {
            block: &body[..length],
            headers,
        });
        rest = &body[length..];
    }

    records
}

fn decode_chunks(data: &[u8]) -> Option<Vec<u8>> {
    let mut decoded = vec![];
    let mut rest = data;

    loop {
        let end_of_size = find_bytes(rest, b"\r\n")?;
        let size = std::str::from_utf8(&rest[..end_of_size]).ok()?;
        let size = usize::from_str_radix(size.split(';').next()?.trim(), 16).ok()?;
        if size == 0 {
            return Some(decoded);
        }
        rest = &rest[end_of_size + 2..];
        decoded.extend_from_slice(rest.get(..size)?);
        rest = rest.get(size + 2..)?;
    }
}

fn decompress(data: &[u8], encoding: &str) -> Option<Vec<u8>> {
    let mut decoded = vec![];
    let result = match encoding.to_lowercase().as_str() {
        "gzip" | "x-gzip" => MultiGzDecoder::new(data).read_to_end(&mut decoded),
        "deflate" => ZlibDecoder::new(data).read_to_end(&mut decoded),
        "br" => Decompressor::new(data, 4096).read_to_end(&mut decoded),
        _ => return None,
    };

    result.ok().map(|_| decoded)
}

struct HttpResponse {
    body: Vec<u8>,
    headers: Headers,
    status: u64,
    status_text: String,
    version: String,
}

/// Parts of an HTTP response, with its body back in the state it was before getting transferred.
fn parse_http_response(block: &[u8]) -> Option<HttpResponse> {
    let end_of_headers = find_bytes(block, b"\r\n\r\n")?;
    let head = &block[..end_of_headers];
    let end_of_status = find_bytes(head, b"\r\n").unwrap_or(head.len());
    let status_line = String::from_utf8_lossy(&head[..end_of_status]);
    let mut status_parts = status_line.splitn(3, ' ');
    let version = status_parts.next()?.to_string();
    let status = status_parts.next()?.trim().parse().ok()?;
    let status_text = status_parts.next().unwrap_or("").trim().to_string();
    let headers = parse_headers(&head[end_of_status..]);
    let mut body = block[end_of_headers + 4..].to_vec();

    // Transfer codings come first, last one applied gets undone first
    if let Some(codings) = find_header(&headers, "Transfer-Encoding") {
        let decoded = codings
            .split(',')
            .map(str::trim)
            .filter(|coding| !coding.is_empty() && !coding.eq_ignore_ascii_case("identity"))
            .rev()
            .try_fold(body.clone(), |body, coding| {
                match coding.eq_ignore_ascii_case("chunked") {
                    true => decode_chunks(&body),
                    false => decompress(&body, coding),
                }
            });
        if let Some(decoded) = decoded {
            body = decoded;
        }
    }
    // Encodings get undone in the reverse order of how they were applied
    if let Some(encodings) = find_header(&headers, "Content-Encoding") {
        let decoded = encodings
            .split(',')
            .map(str::trim)
            .filter(|encoding| !encoding.is_empty() && !encoding.eq_ignore_ascii_case("identity"))
            .rev()
            .try_fold(body.clone(), |body, encoding| decompress(&body, encoding));
        if let Some(decoded) = decoded {
            body = decoded;
        }
    }

    Some(HttpResponse {
        body,
        headers,
        status,
        status_text,
        version,
    })
}

/// Method and headers of a request.
fn parse_http_request(block: &[u8]) -> Option<(String, Headers)> {
    let end_of_headers = find_bytes(block, b"\r\n\r\n").unwrap_or(block.len());
    let head = &block[..end_of_headers];
    let end_of_line = find_bytes(head, b"\r\n").unwrap_or(head.len());
    let method = String::from_utf8_lossy(&head[..end_of_line])
        .split_whitespace()
        .next()?
        .to_string();

    Some((method, parse_headers(&head[end_of_line..])))
}

fn record_time(record: &Record) -> i64 {
    find_header(&record.headers, "WARC-Date")
        .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
        .map(|date| date.timestamp_millis())
        .unwrap_or(0)
}

fn details_of(record: &Record, response: &HttpResponse, request: Option<&Headers>) -> HttpDetails {
    HttpDetails {
        http_version: response.version.clone(),
        request_headers: request.cloned().unwrap_or_default(),
        response_headers: response.headers.clone(),
        status_text: response.status_text.clone(),
        time: record_time(record),
        redirects: vec![],
        is_resource: false,
    }
}

fn target_url(record: &Record) -> Option<String> {
    find_header(&record.headers, "WARC-Target-URI")
        .map(|url| url.trim_matches(['<', '>']).to_string())
}

fn is_gzip(data: &[u8]) -> bool {
    data.starts_with(&GZIP_SIGNATURE)
}

/// Compressed archives are compressed record by record, and may end up being cut short.
/// WARC archive as it is, uncompressed.
pub(crate) fn decompress_warc(data: Vec<u8>) -> Vec<u8> {
    match is_gzip(&data) {
        true => decompress_archive(&data),
        false => data,
    }
}

fn decompress_archive(data: &[u8]) -> Vec<u8> {
    let mut decompressed = vec![];
    let _ = MultiGzDecoder::new(data).read_to_end(&mut decompressed);
    decompressed
}

pub fn is_warc(data: &[u8]) -> bool {
    if is_gzip(data) {
        let mut start = [0; WARC_SIGNATURE.len()];
        MultiGzDecoder::new(data).read_exact(&mut start).is_ok() && start == WARC_SIGNATURE
    } else {
        data.starts_with(WARC_SIGNATURE)
    }
}

fn parse_responses(data: &[u8]) -> Vec<ArchivedResponse> {
    let records = parse_records(data);
    let mut requests: HashMap<&str, (String, Headers)> = HashMap::new();
    let mut payloads: HashMap<&str, Vec<u8>> = HashMap::new();
    let mut responses = vec![];

    // Requests tend to come after their responses
    for record in &records {
        if find_header(&record.headers, "WARC-Type") == Some("request")
            && let Some(id) = find_header(&record.headers, "WARC-Concurrent-To")
            && let Some(request) = parse_http_request(record.block)
        {
            requests.insert(id, request);
        }
    }

    for record in &records {
        if find_header(&record.headers, "WARC-Truncated").is_some() {
            continue;
        }
        let Some(url) = target_url(record) else {
            continue;
        };
        let id = find_header(&record.headers, "WARC-Record-ID").unwrap_or_default();
        let digest = find_header(&record.headers, "WARC-Payload-Digest");
        let request = requests.get(id);
        let method = request
            .map(|(method, _)| method.clone())
            .unwrap_or("GET".to_string());

        match find_header(&record.headers, "WARC-Type") {
            Some("response") => {
                let Some(response) = parse_http_response(record.block) else {
                    continue;
                };
                if let Some(digest) = digest {
                    payloads.entry(digest).or_insert(response.body.clone());
                }
                responses.push(ArchivedResponse {
                    content_type: find_header(&response.headers, "Content-Type")
                        .unwrap_or_default()
                        .to_string(),
                    http: details_of(record, &response, request.map(|(_, headers)| headers)),
                    location: find_header(&response.headers, "Location").map(str::to_string),
                    method,
                    status: response.status,
                    url,
                    data: response.body,
                });
            }
            Some("resource") => responses.push(ArchivedResponse {
                content_type: find_header(&record.headers, "Content-Type")
                    .unwrap_or_default()
                    .to_string(),
                data: record.block.to_vec(),
                http: HttpDetails {
                    time: record_time(record),
                    is_resource: true,
                    ..Default::default()
                },
                location: None,
                method: "GET".to_string(),
                status: 200,
                url,
            }),
            // What has been seen before gets stored once
            Some("revisit") => {
                let (Some(response), Some(payload)) = (
                    parse_http_response(record.block),
                    digest.and_then(|digest| payloads.get(digest)),
                ) else {
                    continue;
                };
                responses.push(ArchivedResponse {
                    content_type: find_header(&response.headers, "Content-Type")
                        .unwrap_or_default()
                        .to_string(),
                    data: payload.clone(),
                    http: details_of(record, &response, request.map(|(_, headers)| headers)),
                    location: None,
                    method,
                    status: response.status,
                    url,
                });
            }
            _ => {}
        }
    }

    responses
}

/// Finds a page in the archive, along with whether there's more than that one page in there.
pub fn parse_warc(data: &[u8], wanted_url: Option<&str>) -> Result<(CapturedPage, bool), String> {
    let decompressed;
    let data = match is_gzip(data) {
        true => {
            decompressed = decompress_archive(data);
            &decompressed[..]
        }
        false => data,
    };
    let responses = parse_responses(data);
    let documents = responses
        .iter()
        .filter(|response| response.status == 200 && is_html(&response.content_type))
        .count();

    Ok((
        create_page_from_responses(responses, wanted_url)?,
        documents > 1,
    ))
}

fn base32(bytes: &[u8]) -> String {
    let mut encoded = String::new();

    for chunk in bytes.chunks(5) {
        let mut buffer = [0_u8; 5];
        buffer[..chunk.len()].copy_from_slice(chunk);
        let bits = buffer
            .iter()
            .fold(0_u64, |bits, byte| (bits << 8) | *byte as u64);
        let characters = (chunk.len() * 8).div_ceil(5);
        for i in 0..8 {
            encoded.push(match i < characters {
                true => BASE32_ALPHABET[((bits >> (35 - i * 5)) & 31) as usize] as char,
                false => '=',
            });
        }
    }

    encoded
}

fn digest(data: &[u8]) -> String {
    format!("sha256:{}", base32(&Sha256::digest(data)))
}

/// Identifiers have to be unique, nothing but that matters about them.
fn record_id(url: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hash = Sha256::new();
    hash.update(COUNTER.fetch_add(1, Ordering::Relaxed).to_be_bytes());
    hash.update(
        Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_default()
            .to_be_bytes(),
    );
    hash.update(std::process::id().to_be_bytes());
    hash.update(url.as_bytes());
    let mut bytes: [u8; 16] = hash.finalize()[..16].try_into().unwrap();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{:02x}", byte)).collect();

    format!(
        "<urn:uuid:{}-{}-{}-{}-{}>",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// Header lines, without anything that could break them, or that's not to be shared.
fn header_lines(headers: &[(String, String)], left_out: &[&str]) -> String {
    // Names are tokens, what isn't one can't be a header
    let is_token = |name: &str| {
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
    };

    headers
        .iter()
        .filter(|(name, _)| is_token(name) && !left_out.contains(&name.to_lowercase().as_str()))
        .map(|(name, value)| format!("{}: {}\r\n", name, single_line(value)))
        .collect()
}

/// Text that goes on a line of its own: archives being read may have anything in them, nothing
/// they have can be allowed to start another line, or a record of its own.
fn single_line(text: &str) -> String {
    text.chars()
        .map(|character| match character.is_control() {
            true => ' ',
            false => character,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Methods are tokens, anything else in there isn't a method.
fn method_of(exchange: &Exchange) -> &str {
    let is_token = !exchange.method.is_empty()
        && exchange
            .method
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte));

    match is_token {
        true => &exchange.method,
        false => "GET",
    }
}

/// URLs with whoever has used them left out.
fn public_url(url: &str) -> Option<Url> {
    let mut url = Url::parse(url).ok()?;
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_fragment(None);

    Some(url)
}

struct Writer {
    compress: bool,
    output: Vec<u8>,
    warcinfo_id: String,
}

impl Writer {
    fn write_record(
        &mut self,
        kind: &str,
        id: &str,
        date: &DateTime<Utc>,
        fields: &[(&str, String)],
        content_type: &str,
        block: &[u8],
    ) {
        let mut record = format!(
            "{}\r\nWARC-Type: {}\r\nWARC-Record-ID: {}\r\nWARC-Date: {}\r\n",
            WARC_VERSION,
            kind,
            id,
            date.to_rfc3339_opts(SecondsFormat::Secs, true)
        );
        for (name, value) in fields {
            record.push_str(&format!("{}: {}\r\n", name, value));
        }
        if kind != "warcinfo" {
            record.push_str(&format!("WARC-Warcinfo-ID: {}\r\n", self.warcinfo_id));
        }
        record.push_str(&format!(
            "WARC-Block-Digest: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
            digest(block),
            content_type,
            block.len()
        ));
        let mut record = record.into_bytes();
        record.extend_from_slice(block);
        record.extend_from_slice(b"\r\n\r\n");

        // Compressed archives are made of records compressed one by one, that's how they can be
        // read starting from any of them
        if self.compress {
            let mut encoder = GzEncoder::new(vec![], Compression::default());
            let _ = encoder.write_all(&record);
            record = encoder.finish().unwrap_or_default();
        }
        self.output.extend(record);
    }

    fn write_exchange(&mut self, exchange: &Exchange) {
        let Some(url) = public_url(&exchange.url) else {
            return;
        };
        let target = ("WARC-Target-URI", url.to_string());
        let id = record_id(url.as_str());

        let Some(status) = exchange.status else {
            let content_type = match exchange.content_type.is_empty() {
                true => "application/octet-stream",
                false => exchange.content_type.as_str(),
            };
            let content_type = single_line(content_type);
            self.write_record(
                "resource",
                &id,
                &exchange.date,
                &[target, ("WARC-Payload-Digest", digest(&exchange.body))],
                &content_type,
                &exchange.body,
            );
            return;
        };

        let mut left_out = PRIVATE_RESPONSE_HEADERS.to_vec();
        left_out.extend(TRANSFER_HEADERS);
        let version = match exchange.http_version.eq_ignore_ascii_case("HTTP/1.0") {
            true => "HTTP/1.0",
            false => "HTTP/1.1",
        };
        // The space after the status has to be there, even when there's no reason phrase
        let mut response = format!(
            "{} {} {}\r\n",
            version,
            status,
            single_line(&exchange.status_text)
        );
        response.push_str(&header_lines(&exchange.response_headers, &left_out));
        response.push_str(&format!("Content-Length: {}\r\n\r\n", exchange.body.len()));
        let mut response = response.into_bytes();
        response.extend_from_slice(&exchange.body);
        self.write_record(
            "response",
            &id,
            &exchange.date,
            &[
                target.clone(),
                ("WARC-Payload-Digest", digest(&exchange.body)),
            ],
            "application/http; msgtype=response",
            &response,
        );

        let path = &url[::url::Position::BeforePath..::url::Position::AfterQuery];
        // Requests get sent the same way, no matter what servers answer with
        let mut request = format!("{} {} HTTP/1.1\r\n", method_of(exchange), path);
        let has_host = exchange
            .request_headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("host"));
        if !has_host && url.has_host() {
            request.push_str(&format!(
                "Host: {}\r\n",
                &url[::url::Position::BeforeHost..::url::Position::AfterPort]
            ));
        }
        // What was sent along with requests isn't known, saying there was something would be a lie
        let mut left_out = PRIVATE_REQUEST_HEADERS.to_vec();
        left_out.extend(TRANSFER_HEADERS);
        request.push_str(&header_lines(&exchange.request_headers, &left_out));
        request.push_str("\r\n");
        self.write_record(
            "request",
            &record_id(url.as_str()),
            &exchange.date,
            &[target, ("WARC-Concurrent-To", id)],
            "application/http; msgtype=request",
            request.as_bytes(),
        );
    }
}

/// Archive that gets written record by record: what's been put together goes out with take(), so
/// nothing but the record being written has to stay around.
pub(crate) struct WarcStream {
    writer: Writer,
}

impl WarcStream {
    /// Starts an archive, with a record that says what it is.
    pub(crate) fn new(compress: bool) -> WarcStream {
        let mut writer = Writer {
            compress,
            output: vec![],
            warcinfo_id: record_id("warcinfo"),
        };
        let info = format!(
            "software: {}/{}\r\nformat: WARC File Format 1.1\r\n\
             conformsTo: https://iipc.github.io/warc-specifications/specifications/warc-format/warc-1.1/\r\n",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );
        let warcinfo_id = writer.warcinfo_id.clone();
        writer.write_record(
            "warcinfo",
            &warcinfo_id,
            &Utc::now(),
            &[],
            "application/warc-fields",
            info.as_bytes(),
        );

        WarcStream { writer }
    }

    pub(crate) fn add(&mut self, exchange: &Exchange) {
        self.writer.write_exchange(exchange);
    }

    /// What's been put together since the last time.
    pub(crate) fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.writer.output)
    }
}

/// Puts everything the session has recorded into an archive.
pub fn create_warc_document(session: &Session) -> Vec<u8> {
    let mut stream = WarcStream::new(session.options.output_format == MonolithOutputFormat::WARCGZ);

    if let Some(cache) = session.cache() {
        for index in 0..cache.exchange_count() {
            if let Some(exchange) = cache.exchange(index) {
                stream.add(&exchange);
            }
        }
    }

    stream.take()
}
