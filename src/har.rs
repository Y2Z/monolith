//! HAR archives: what the network panel of a web browser has seen, exported as JSON.

use base64::prelude::*;
use serde_json::Value;

use crate::replay::{
    ArchivedResponse, CapturedPage, HttpDetails, create_page_from_responses, set_content_type,
    with_utf8_charset,
};

fn parse(data: &[u8]) -> Option<Value> {
    let data = data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data);

    serde_json::from_slice(data).ok()
}

fn find_header<'a>(message: &'a Value, name: &str) -> Option<&'a str> {
    message["headers"].as_array()?.iter().find(|header| {
        header["name"]
            .as_str()
            .is_some_and(|key| key.eq_ignore_ascii_case(name))
    })?["value"]
        .as_str()
}

pub fn is_har(data: &[u8]) -> bool {
    let start = data.iter().position(|byte| !byte.is_ascii_whitespace());

    (start.is_some_and(|start| data[start] == b'{') || data.starts_with(b"\xef\xbb\xbf"))
        && parse(data).is_some_and(|har| har["log"]["entries"].is_array())
}

/// Nothing but what the response was, plain text in here has already been decoded.
fn parse_response(entry: &Value) -> Option<ArchivedResponse> {
    let response = &entry["response"];
    let content = &response["content"];
    let content_type = find_header(response, "Content-Type")
        .or(content["mimeType"].as_str())
        .unwrap_or_default();
    let status = response["status"].as_u64()?;
    let data = match (content["text"].as_str(), content["encoding"].as_str()) {
        (Some(text), Some("base64")) => Some(BASE64_STANDARD.decode(text).ok()?),
        (Some(text), _) => Some(text.as_bytes().to_vec()),
        (None, _) => None,
    };
    // Some responses have nothing in them by definition
    let is_head = entry["request"]["method"].as_str() == Some("HEAD");
    if data.is_none()
        && !is_head
        && !(300..400).contains(&status)
        && !matches!(status, 100..200 | 204 | 205)
    {
        return None;
    }
    let is_text = content["text"].is_string() && content["encoding"] != "base64";
    let headers_of = |message: &Value| -> Vec<(String, String)> {
        message["headers"]
            .as_array()
            .map(|headers| {
                headers
                    .iter()
                    .filter_map(|header| {
                        Some((
                            header["name"].as_str()?.to_string(),
                            header["value"].as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let location = find_header(response, "Location")
        .or(response["redirectURL"].as_str())
        .filter(|location| !location.is_empty())
        .map(str::to_string);
    let content_type = match is_text {
        true => with_utf8_charset(content_type),
        false => content_type.to_string(),
    };
    let mut response_headers = headers_of(response);
    if is_text && !content_type.is_empty() {
        set_content_type(&mut response_headers, &content_type);
    }
    if let Some(location) = &location
        && find_header(response, "Location").is_none()
    {
        response_headers.push(("Location".to_string(), location.clone()));
    }
    let http = HttpDetails {
        http_version: response["httpVersion"].as_str().unwrap_or("").to_string(),
        request_headers: headers_of(&entry["request"]),
        response_headers,
        status_text: response["statusText"].as_str().unwrap_or("").to_string(),
        time: entry["startedDateTime"]
            .as_str()
            .and_then(|date| chrono::DateTime::parse_from_rfc3339(date).ok())
            .map(|date| date.timestamp_millis())
            .unwrap_or(0),
        redirects: vec![],
        is_resource: false,
    };

    Some(ArchivedResponse {
        content_type,
        data: data.unwrap_or_default(),
        http,
        location,
        method: entry["request"]["method"].as_str()?.to_string(),
        status,
        url: entry["request"]["url"].as_str()?.to_string(),
    })
}

/// Finds a page in the archive, along with whether there's more than that one page in there.
pub fn parse_har(data: &[u8], wanted_url: Option<&str>) -> Result<(CapturedPage, bool), String> {
    let har = parse(data).ok_or("could not parse HAR archive")?;
    let entries = har["log"]["entries"]
        .as_array()
        .ok_or("HAR has no entries")?;
    let pages = har["log"]["pages"].as_array();
    let wanted_entry = wanted_url.and_then(|url| {
        entries
            .iter()
            .find(|entry| entry["request"]["url"].as_str() == Some(url))
    });
    let page_id = wanted_entry
        .map(|entry| &entry["pageref"])
        .or(pages
            .and_then(|pages| pages.first())
            .map(|page| &page["id"]))
        .and_then(Value::as_str);

    let is_whole_log = page_id.is_none() || (wanted_url.is_some() && wanted_entry.is_none());

    let responses: Vec<ArchivedResponse> = entries
        .iter()
        .filter(|entry| {
            is_whole_log
                || entry["pageref"]
                    .as_str()
                    .is_none_or(|id| Some(id) == page_id)
        })
        .filter_map(parse_response)
        .collect();

    if responses.is_empty() && !entries.is_empty() {
        return Err("HAR archive has no contents, it has to be exported with them".to_string());
    }

    Ok((
        create_page_from_responses(responses, wanted_url)?,
        pages.is_some_and(|pages| pages.len() > 1),
    ))
}
