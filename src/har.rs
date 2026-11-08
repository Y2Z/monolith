//! HAR archives: what the network panel of a web browser has seen, exported as JSON.

use base64::prelude::*;
use serde_json::Value;

use crate::replay::{ArchivedResponse, CapturedPage, create_page_from_responses};

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
    if data.is_none() && !(300..400).contains(&status) {
        return None;
    }
    let is_text = content["text"].is_string() && content["encoding"] != "base64";

    Some(ArchivedResponse {
        content_type: match is_text {
            true => format!(
                "{}; charset=utf-8",
                content_type.split(';').next().unwrap_or("").trim()
            ),
            false => content_type.to_string(),
        },
        data: data.unwrap_or_default(),
        location: find_header(response, "Location")
            .or(response["redirectURL"].as_str())
            .filter(|location| !location.is_empty())
            .map(str::to_string),
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
