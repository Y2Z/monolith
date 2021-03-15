use base64;
use url::{form_urlencoded, Url};

use crate::utils::detect_media_type;

pub fn clean_url(url: Url) -> Url {
    let mut url = url.clone();

    // Clear fragment (if any)
    url.set_fragment(None);

    url
}

pub fn create_data_url(media_type: &str, data: &[u8], final_asset_url: &Url) -> Url {
    let media_type: String = if media_type.is_empty() {
        detect_media_type(data, &final_asset_url)
    } else {
        media_type.to_string()
    };

    let mut data_url: Url = Url::parse("data:,").unwrap();

    data_url.set_path(format!("{};base64,{}", media_type, base64::encode(data)).as_str());

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

pub fn parse_data_url(url: &Url) -> (String, Vec<u8>) {
    let path: String = url.path().to_string();
    let comma_loc: usize = path.find(',').unwrap_or(path.len());

    let meta_data: String = path.chars().take(comma_loc).collect();
    let raw_data: String = path.chars().skip(comma_loc + 1).collect();

    let text: String = percent_decode(raw_data);

    let meta_data_items: Vec<&str> = meta_data.split(';').collect();
    let mut media_type: String = str!();
    let mut encoding: &str = "";

    let mut i: i8 = 0;
    for item in &meta_data_items {
        if i == 0 {
            media_type = str!(item);
        } else {
            if item.eq_ignore_ascii_case("base64")
                || item.eq_ignore_ascii_case("utf8")
                || item.eq_ignore_ascii_case("charset=UTF-8")
            {
                encoding = item;
            }
        }

        i = i + 1;
    }

    let data: Vec<u8> = if encoding.eq_ignore_ascii_case("base64") {
        base64::decode(&text).unwrap_or(vec![])
    } else {
        text.as_bytes().to_vec()
    };

    (media_type, data)
}

pub fn percent_decode(input: String) -> String {
    let input: String = input.replace("+", "%2B");

    form_urlencoded::parse(input.as_bytes())
        .map(|(key, val)| {
            [
                key.to_string(),
                if val.to_string().len() == 0 {
                    str!()
                } else {
                    str!('=')
                },
                val.to_string(),
            ]
            .concat()
        })
        .collect()
}

pub fn percent_encode(input: String) -> String {
    form_urlencoded::byte_serialize(input.as_bytes()).collect()
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
