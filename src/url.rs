use std::fmt::Write;

use dataurl::{DataEncoding, DataUrl, Escaping};
pub use url::Url;

use crate::core::detect_media_type;

pub const EMPTY_IMAGE_DATA_URL: &str = "data:image/png,\
%89PNG%0D%0A%1A%0A%00%00%00%0DIHDR%00%00%00%0D%00%00%00%0D%08%04%00%00%00%D8%E2%2C%F7%00%00%00%11IDATx%DAcd%C0%09%18G%A5%28%96%02%00%0A%F8%00%0E%CB%8A%EB%16%00%00%00%00IEND%AEB%60%82";

pub fn clean_url(url: Url) -> Url {
    let mut url = url.clone();

    // Remove fragment (if any)
    url.set_fragment(None);

    url
}

/// Creates a data URL for an asset, with an optional fragment (already percent-encoded, as Url::fragment() returns it).
/// Passing data as a Vec<u8> moves it in without copying.
///
/// The data gets percent-encoded or base64-encoded, whichever is shorter, and percent-encoding only escapes
/// what has to be, which keeps text assets like SVG and CSS small and readable. That's only safe inside
/// double-quoted HTML attributes and CSS strings, which is where monolith puts data URLs; srcset is the exception,
/// see create_srcset_data_url().
pub fn create_data_url(
    media_type: &str,
    charset: &str,
    data: impl Into<Vec<u8>>,
    final_asset_url: &Url,
    fragment: Option<&str>,
) -> String {
    let mut data_url = new_data_url(media_type, charset, data.into(), final_asset_url, fragment);
    data_url.set_data_encoding(DataEncoding::Shortest);
    data_url.set_escaping(Escaping::Minimal);
    serialize_data_url(&data_url)
}

/// Same as create_data_url(), but always uses base64, since srcset gets split on whitespace
/// and has trailing commas removed from URLs, which percent-encoded data could contain.
pub fn create_srcset_data_url(
    media_type: &str,
    charset: &str,
    data: impl Into<Vec<u8>>,
    final_asset_url: &Url,
    fragment: Option<&str>,
) -> String {
    let mut data_url = new_data_url(media_type, charset, data.into(), final_asset_url, fragment);
    data_url.set_data_encoding(DataEncoding::Base64);
    serialize_data_url(&data_url)
}

fn new_data_url(
    media_type: &str,
    charset: &str,
    data: Vec<u8>,
    final_asset_url: &Url,
    fragment: Option<&str>,
) -> DataUrl {
    // TODO: move this block out of this function
    let media_type: String = if media_type.is_empty() {
        detect_media_type(&data, final_asset_url)
    } else {
        media_type.to_string()
    };

    let mut data_url = DataUrl::new();
    data_url.set_data(data);
    data_url.set_fragment(fragment);

    // Invalid media types get omitted, which makes browsers treat the data as text/plain, same as they would anyway
    if !media_type.is_empty() {
        let _ = data_url.set_media_type(Some(&media_type));
    }

    // The charset is kept as declared (US-ASCII is the default, so it's left out);
    // values that can't be represented in a data URL (e.g. containing a comma) get omitted
    let charset: &str = charset.trim().trim_matches('"');
    if !charset.is_empty() && !charset.eq_ignore_ascii_case("US-ASCII") {
        let _ = data_url.set_parameter("charset", Some(charset));
    }

    data_url
}

fn serialize_data_url(data_url: &DataUrl) -> String {
    // Allocating the exact size upfront avoids reallocations (and copies) while the string grows
    let mut result = String::with_capacity(data_url.encoded_len());
    write!(result, "{data_url}").unwrap();
    result
}

pub fn domain_is_within_domain(domain: &str, domain_to_match_against: &str) -> bool {
    if domain_to_match_against.is_empty() {
        return false;
    }

    if domain_to_match_against == "." {
        return true;
    }

    let domain_partials: Vec<&str> = domain.trim_end_matches(".").rsplit(".").collect();
    let domain_to_match_against_partials: Vec<&str> = domain_to_match_against
        .trim_end_matches(".")
        .rsplit(".")
        .collect();
    let domain_to_match_against_starts_with_a_dot = domain_to_match_against.starts_with(".");

    let mut i: usize = 0;
    let l: usize = std::cmp::max(
        domain_partials.len(),
        domain_to_match_against_partials.len(),
    );
    let mut ok: bool = true;

    while i < l {
        // Exit and return false if went out of bounds of domain to match against, and it didn't start with a dot
        if !domain_to_match_against_starts_with_a_dot
            && domain_to_match_against_partials.len() < i + 1
        {
            ok = false;
            break;
        }

        let domain_partial = if domain_partials.len() < i + 1 {
            ""
        } else {
            domain_partials.get(i).unwrap()
        };
        let domain_to_match_against_partial = if domain_to_match_against_partials.len() < i + 1 {
            ""
        } else {
            domain_to_match_against_partials.get(i).unwrap()
        };

        let parts_match = domain_to_match_against_partial.eq_ignore_ascii_case(domain_partial);

        if !parts_match && !domain_to_match_against_partial.is_empty() {
            ok = false;
            break;
        }

        i += 1;
    }

    ok
}

pub fn is_url_and_has_protocol(input: &str) -> bool {
    match Url::parse(input) {
        Ok(parsed_url) => !parsed_url.scheme().is_empty(),
        Err(_) => false,
    }
}

pub fn parse_data_url(url: &Url) -> (String, String, Vec<u8>) {
    match DataUrl::parse(url.as_str()) {
        // The charset is empty if not given, same as for other assets
        Ok(data_url) => (
            data_url.media_type().to_string(),
            data_url
                .charset_no_default()
                .unwrap_or_default()
                .to_string(),
            data_url.into_data(),
        ),
        // Malformed data URLs (no comma, invalid base64) are treated as empty ones
        Err(_) => ("text/plain".to_string(), "".to_string(), vec![]),
    }
}

pub fn get_referer_url(url: Url) -> Url {
    let mut url = url.clone();
    // Spec: https://httpwg.org/specs/rfc9110.html#field.referer
    // Must not include the fragment and userinfo components of the URI
    url.set_fragment(None);
    url.set_username("").unwrap();
    url.set_password(None).unwrap();

    url
}

pub fn resolve_url(from: &Url, to: &str) -> Url {
    match Url::parse(to) {
        Ok(parsed_url) => parsed_url,
        Err(_) => match from.join(to) {
            Ok(joined) => joined,
            Err(_) => Url::parse("data:,").unwrap(),
        },
    }
}
