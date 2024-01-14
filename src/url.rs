use base64::prelude::*;
use percent_encoding::percent_decode_str;
use url::Url;

use crate::utils::{detect_media_type, parse_content_type};

pub const EMPTY_IMAGE_DATA_URL: &'static str = "data:image/png;base64,\
iVBORw0KGgoAAAANSUhEUgAAAA0AAAANCAQAAADY4iz3AAAAEUlEQVR42mNkwAkYR6UolgIACvgADsuK6xYAAAAASUVORK5CYII=";

pub fn clean_url(url: Url) -> Url {
    let mut url = url.clone();

    // Clear fragment (if any)
    url.set_fragment(None);

    url
}

pub fn create_data_url(media_type: &str, charset: &str, data: &[u8], final_asset_url: &Url) -> Url {
    // TODO: move this block out of this function
    let media_type: String = if media_type.is_empty() {
        detect_media_type(data, &final_asset_url)
    } else {
        media_type.to_string()
    };

    let mut data_url: Url = Url::parse("data:,").unwrap();

    let c: String =
        if !charset.trim().is_empty() && !charset.trim().eq_ignore_ascii_case("US-ASCII") {
            format!(";charset={}", charset.trim())
        } else {
            "".to_string()
        };

    data_url.set_path(
        format!(
            "{}{};base64,{}",
            media_type,
            c,
            BASE64_STANDARD.encode(data)
        )
        .as_str(),
    );

    data_url
}

pub fn is_url_and_has_protocol(input: &str) -> bool {
    match Url::parse(&input) {
        Ok(parsed_url) => {
            return parsed_url.scheme().len() > 0;
        }
        Err(_) => {
            return false;
        }
    }
}

pub fn parse_data_url(url: &Url) -> (String, String, Vec<u8>) {
    let path: String = url.path().to_string();
    let comma_loc: usize = path.find(',').unwrap_or(path.len());

    // Split data URL into meta data and raw data
    let content_type: String = path.chars().take(comma_loc).collect();
    let data: String = path.chars().skip(comma_loc + 1).collect();

    // Parse meta data
    let (media_type, charset, is_base64) = parse_content_type(&content_type);

    // Parse raw data into vector of bytes
    let text: String = percent_decode_str(&data).decode_utf8_lossy().to_string();
    let blob: Vec<u8> = if is_base64 {
        BASE64_STANDARD.decode(&text).unwrap_or(vec![])
    } else {
        text.as_bytes().to_vec()
    };

    (media_type, charset, blob)
}

pub fn resolve_url(from: &Url, to: &str) -> Url {
    match Url::parse(&to) {
        Ok(parsed_url) => parsed_url,
        Err(_) => match from.join(to) {
            Ok(joined) => joined,
            Err(_) => Url::parse("data:,").unwrap(),
        },
    }
}
