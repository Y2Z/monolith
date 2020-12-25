use base64;
use url::{form_urlencoded, ParseError, Url};

use crate::utils::detect_media_type;

pub fn clean_url<T: AsRef<str>>(input: T) -> String {
    let mut url = Url::parse(input.as_ref()).unwrap();

    // Clear fragment
    url.set_fragment(None);

    // Get rid of stray question mark
    if url.query() == Some("") {
        url.set_query(None);
    }

    // Remove empty trailing ampersand(s)
    let mut result: String = url.to_string();
    while result.ends_with("&") {
        result.pop();
    }

    result
}

pub fn data_to_data_url(media_type: &str, data: &[u8], url: &str) -> String {
    let media_type: String = if media_type.is_empty() {
        detect_media_type(data, &url)
    } else {
        media_type.to_string()
    };

    format!("data:{};base64,{}", media_type, base64::encode(data))
}

pub fn decode_url(input: String) -> String {
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

pub fn file_url_to_fs_path(url: &str) -> String {
    if !is_file_url(url) {
        return str!();
    }

    let cutoff_l = if cfg!(windows) { 8 } else { 7 };
    let mut fs_file_path: String = decode_url(url.to_string()[cutoff_l..].to_string());
    let url_fragment = get_url_fragment(url);
    if url_fragment != "" {
        let max_len = fs_file_path.len() - 1 - url_fragment.len();
        fs_file_path = fs_file_path[0..max_len].to_string();
    }

    if cfg!(windows) {
        fs_file_path = fs_file_path.replace("/", "\\");
    }

    // File paths should not be %-encoded
    decode_url(fs_file_path)
}

pub fn get_url_fragment<T: AsRef<str>>(url: T) -> String {
    if Url::parse(url.as_ref()).unwrap().fragment() == None {
        str!()
    } else {
        str!(Url::parse(url.as_ref()).unwrap().fragment().unwrap())
    }
}

pub fn is_data_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "data"))
        .unwrap_or(false)
}

pub fn is_file_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "file"))
        .unwrap_or(false)
}

pub fn is_http_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "http" || u.scheme() == "https"))
        .unwrap_or(false)
}

pub fn parse_data_url<T: AsRef<str>>(url: T) -> (String, Vec<u8>) {
    let parsed_url: Url = Url::parse(url.as_ref()).unwrap_or(Url::parse("data:,").unwrap());
    let path: String = parsed_url.path().to_string();
    let comma_loc: usize = path.find(',').unwrap_or(path.len());

    let meta_data: String = path.chars().take(comma_loc).collect();
    let raw_data: String = path.chars().skip(comma_loc + 1).collect();

    let text: String = decode_url(raw_data);

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

pub fn resolve_url<T: AsRef<str>, U: AsRef<str>>(from: T, to: U) -> Result<String, ParseError> {
    let result = if is_http_url(to.as_ref()) {
        to.as_ref().to_string()
    } else {
        Url::parse(from.as_ref())?
            .join(to.as_ref())?
            .as_ref()
            .to_string()
    };
    Ok(result)
}

pub fn url_has_protocol<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme().len() > 0))
        .unwrap_or(false)
}

pub fn url_with_fragment(url: &str, fragment: &str) -> String {
    let mut result = str!(&url);

    if !fragment.is_empty() {
        result += "#";
        result += fragment;
    }

    result
}
