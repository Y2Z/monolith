//! WARC (ISO 28500) archives: everything a crawler has exchanged with servers, record by record.

use std::collections::HashMap;
use std::io::Read;

use brotli_decompressor::Decompressor;
use flate2::read::{MultiGzDecoder, ZlibDecoder};

use crate::replay::{ArchivedResponse, CapturedPage, create_page_from_responses, is_html};

const GZIP_SIGNATURE: [u8; 2] = [0x1f, 0x8b];
const WARC_SIGNATURE: &[u8] = b"WARC/";

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

/// Parts of an HTTP response, with its body back in the state it was before getting transferred.
fn parse_http_response(block: &[u8]) -> Option<(u64, Headers, Vec<u8>)> {
    let end_of_headers = find_bytes(block, b"\r\n\r\n")?;
    let head = &block[..end_of_headers];
    let end_of_status = find_bytes(head, b"\r\n").unwrap_or(head.len());
    let status_line = String::from_utf8_lossy(&head[..end_of_status]);
    let status = status_line.split_whitespace().nth(1)?.parse().ok()?;
    let headers = parse_headers(&head[end_of_status..]);
    let mut body = block[end_of_headers + 4..].to_vec();

    if find_header(&headers, "Transfer-Encoding")
        .is_some_and(|encoding| encoding.eq_ignore_ascii_case("chunked"))
        && let Some(decoded) = decode_chunks(&body)
    {
        body = decoded;
    }
    if let Some(decoded) = find_header(&headers, "Content-Encoding")
        .and_then(|encoding| decompress(&body, encoding.trim()))
    {
        body = decoded;
    }

    Some((status, headers, body))
}

fn request_method(block: &[u8]) -> Option<String> {
    let end_of_line = find_bytes(block, b"\r\n").unwrap_or(block.len());

    String::from_utf8_lossy(&block[..end_of_line])
        .split_whitespace()
        .next()
        .map(str::to_string)
}

fn target_url(record: &Record) -> Option<String> {
    find_header(&record.headers, "WARC-Target-URI")
        .map(|url| url.trim_matches(['<', '>']).to_string())
}

fn is_gzip(data: &[u8]) -> bool {
    data.starts_with(&GZIP_SIGNATURE)
}

/// Compressed archives are compressed record by record, and may end up being cut short.
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
    let mut methods: HashMap<&str, String> = HashMap::new();
    let mut payloads: HashMap<&str, Vec<u8>> = HashMap::new();
    let mut responses = vec![];

    // Requests tend to come after their responses
    for record in &records {
        if find_header(&record.headers, "WARC-Type") == Some("request")
            && let Some(id) = find_header(&record.headers, "WARC-Concurrent-To")
            && let Some(method) = request_method(record.block)
        {
            methods.insert(id, method);
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

        match find_header(&record.headers, "WARC-Type") {
            Some("response") => {
                let Some((status, headers, body)) = parse_http_response(record.block) else {
                    continue;
                };
                if let Some(digest) = digest {
                    payloads.entry(digest).or_insert(body.clone());
                }
                responses.push(ArchivedResponse {
                    content_type: find_header(&headers, "Content-Type")
                        .unwrap_or_default()
                        .to_string(),
                    data: body,
                    location: find_header(&headers, "Location").map(str::to_string),
                    method: methods.get(id).cloned().unwrap_or("GET".to_string()),
                    status,
                    url,
                });
            }
            Some("resource") => responses.push(ArchivedResponse {
                content_type: find_header(&record.headers, "Content-Type")
                    .unwrap_or_default()
                    .to_string(),
                data: record.block.to_vec(),
                location: None,
                method: "GET".to_string(),
                status: 200,
                url,
            }),
            // What has been seen before gets stored once
            Some("revisit") => {
                let (Some((status, headers, _)), Some(payload)) = (
                    parse_http_response(record.block),
                    digest.and_then(|digest| payloads.get(digest)),
                ) else {
                    continue;
                };
                responses.push(ArchivedResponse {
                    content_type: find_header(&headers, "Content-Type")
                        .unwrap_or_default()
                        .to_string(),
                    data: payload.clone(),
                    location: None,
                    method: "GET".to_string(),
                    status,
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
