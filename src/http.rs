use regex::Regex;
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use std::time::Duration;
use url::{ParseError, Url};
use utils::data_to_dataurl;

lazy_static! {
    static ref REGEX_URL: Regex = Regex::new(r"^https?://").unwrap();
}

pub fn is_valid_url(path: &str) -> bool {
    REGEX_URL.is_match(path)
}

pub fn resolve_url(from: &str, to: &str) -> Result<String, ParseError> {
    let result = if is_valid_url(to) {
        // (anything, http://site.com/css/main.css)
        to.to_string()
    } else {
        let mut re = String::new();
        if is_valid_url(from) {
            // It's a remote resource (HTTP)
            if to.chars().nth(0) == Some('/') {
                // (http://site.com/article/1, /...?)
                let from_url = Url::parse(from)?;

                if to.chars().nth(1) == Some('/') {
                    // (http://site.com/article/1, //images/1.png)
                    re.push_str(from_url.scheme());
                    re.push_str(":");
                    re.push_str(to);
                } else {
                    // (http://site.com/article/1, /css/main.css)
                    re.push_str(from_url.scheme());
                    re.push_str("://");
                    re.push_str(from_url.host_str().unwrap());
                    re.push_str(to);
                }
            } else {
                // (http://site.com, css/main.css)
                // TODO improve to ensure no // or /// ever happen
                re.push_str(from);
                re.push_str("/");
                re.push_str(to);
            }
        } else {
            // It's a local resource (fs)
            // TODO improve to ensure no // or /// ever happen
            // TODO for fs use basepath instead of $from
            re.push_str(from);
            re.push_str("/");
            re.push_str(to);
        }
        re
    };

    Ok(result)
}

pub fn url_is_data(url: &str) -> Result<bool, String> {
    match Url::parse(url) {
        Ok(parsed_url) => Ok(parsed_url.scheme() == "data"),
        Err(err) => Err(format!("{}", err)),
    }
}

pub fn retrieve_asset(
    url: &str,
    as_dataurl: bool,
    as_mime: &str,
) -> Result<String, reqwest::Error> {
    if url_is_data(&url).unwrap() {
        Ok(url.to_string())
    } else {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let mut response = client.get(url).send().unwrap();

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
    fn test_is_url() {
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
        let resolved_url = resolve_url("https://www.kernel.org", "../category/signatures.html")?;
        assert_eq!(
            resolved_url.as_str(),
            "https://www.kernel.org/../category/signatures.html"
        );

        let resolved_url = resolve_url("https://www.kernel.org", "category/signatures.html")?;
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
            "https://www.kernel.org/category/signatures.html",
            "/theme/images/logos/tux.png",
        )?;
        assert_eq!(
            resolved_url.as_str(),
            "https://www.kernel.org/theme/images/logos/tux.png"
        );

        Ok(())
    }

    #[test]
    fn test_url_is_data() {
        assert!(
            url_is_data("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h")
                .unwrap_or(false)
        );
        assert!(!url_is_data("https://kernel.org").unwrap_or(false));
        assert!(!url_is_data("//kernel.org").unwrap_or(false));
    }
}
