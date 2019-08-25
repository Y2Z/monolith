use regex::Regex;
use reqwest::Client;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use std::time::Duration;
use url::{ParseError, Url};
use utils::data_to_dataurl;

lazy_static! {
    static ref REGEX_URL: Regex = Regex::new(r"^https?://").unwrap();
}

pub fn is_data_url(url: &str) -> Result<bool, ParseError> {
    match Url::parse(url) {
        Ok(parsed_url) => Ok(parsed_url.scheme() == "data"),
        Err(err) => Err(err),
    }
}

pub fn is_valid_url(path: &str) -> bool {
    REGEX_URL.is_match(path)
}

pub fn resolve_url(from: &str, to: &str) -> Result<String, ParseError> {
    let result = if is_valid_url(to) {
        // (anything, http://site.com/css/main.css)
        to.to_string()
    } else {
        Url::parse(from)?.join(to)?.to_string()
    };

    Ok(result)
}

pub fn retrieve_asset(
    url: &str,
    as_dataurl: bool,
    as_mime: &str,
    opt_user_agent: &str,
) -> Result<String, reqwest::Error> {
    if is_data_url(&url).unwrap() {
        Ok(url.to_string())
    } else {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        let mut response = client
            .get(url)
            .header(USER_AGENT, opt_user_agent)
            .send()?;
        let final_url = response.url().as_str();

        if url == final_url {
            eprintln!("[ {} ]", &url);
        } else {
            eprintln!("[ {} -> {} ]", &url, &final_url);
        }

        if as_dataurl {
            // Convert response into a byte array
            let mut data: Vec<u8> = vec![];
            response.copy_to(&mut data)?;

            // Attempt to obtain MIME type by reading the Content-Type header
            let mimetype = if as_mime == "" {
                response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|header| header.to_str().ok())
                    .unwrap_or(&as_mime)
            } else {
                as_mime
            };

            Ok(data_to_dataurl(&mimetype, &data))
        } else {
            Ok(response.text().unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_url() {
        assert!(is_valid_url("https://www.rust-lang.org/"));
        assert!(is_valid_url("http://kernel.org"));
        assert!(!is_valid_url("./index.html"));
        assert!(!is_valid_url("some-local-page.htm"));
        assert!(!is_valid_url("ftp://1.2.3.4/www/index.html"));
        assert!(!is_valid_url(
            "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
        ));
    }

    #[test]
    fn test_resolve_url() -> Result<(), ParseError> {
        let resolved_url = resolve_url(
            "https://www.kernel.org",
            "../category/signatures.html",
        )?;
        assert_eq!(
            resolved_url.as_str(),
            "https://www.kernel.org/category/signatures.html"
        );

        let resolved_url = resolve_url(
            "https://www.kernel.org",
            "category/signatures.html",
        )?;
        assert_eq!(
            resolved_url.as_str(),
            "https://www.kernel.org/category/signatures.html"
        );

        let resolved_url = resolve_url(
            "saved_page.htm",
            "https://www.kernel.org/category/signatures.html",
        )?;
        assert_eq!(
            resolved_url.as_str(),
            "https://www.kernel.org/category/signatures.html"
        );

        let resolved_url = resolve_url(
            "https://www.kernel.org",
            "//www.kernel.org/theme/images/logos/tux.png",
        )?;
        assert_eq!(
            resolved_url.as_str(),
            "https://www.kernel.org/theme/images/logos/tux.png"
        );

        let resolved_url = resolve_url(
            "https://www.kernel.org",
            "//another-host.org/theme/images/logos/tux.png",
        )?;
        assert_eq!(
            resolved_url.as_str(),
            "https://another-host.org/theme/images/logos/tux.png"
        );

        let resolved_url = resolve_url(
            "https://www.kernel.org/category/signatures.html",
            "/theme/images/logos/tux.png",
        )?;
        assert_eq!(
            resolved_url.as_str(),
            "https://www.kernel.org/theme/images/logos/tux.png"
        );

        let resolved_url = resolve_url(
            "https://www.w3schools.com/html/html_iframe.asp",
            "default.asp",
        )?;
        assert_eq!(
            resolved_url.as_str(),
            "https://www.w3schools.com/html/default.asp"
        );

        Ok(())
    }

    #[test]
    fn test_is_data_url() {
        assert!(
            is_data_url("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h")
                .unwrap_or(false)
        );
        assert!(!is_data_url("https://kernel.org").unwrap_or(false));
        assert!(!is_data_url("//kernel.org").unwrap_or(false));
    }
}
