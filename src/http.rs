extern crate regex;
extern crate reqwest;
extern crate url;

use self::reqwest::header::CONTENT_TYPE;
use self::regex::Regex;
use self::url::{Url, ParseError};
use utils::data_to_dataurl;

pub fn is_url(path: &str) -> bool {
    let re = Regex::new(r"^https?://").unwrap();
    re.is_match(path)
}

pub fn resolve_url(from: &str, to: &str) -> Result<String, ParseError> {
    let mut result = "".to_string();

    if is_url(to) { // (anything, http://site.com/css/main.css)
        result = to.to_string();
    } else {
        if is_url(from) { // It's a remote resource (HTTP)
            if to.chars().nth(0) == Some('/') { // (http://site.com/article/1, /...?)
                let from_url = Url::parse(from)?;

                if to.chars().nth(1) == Some('/') { // (http://site.com/article/1, //images/1.png)
                    result.push_str(from_url.scheme());
                    result.push_str(":");
                    result.push_str(to);
                } else { // (http://site.com/article/1, /css/main.css)
                    result.push_str(from_url.scheme());
                    result.push_str("://");
                    result.push_str(from_url.host_str().unwrap());
                    result.push_str(to);
                }
            } else { // (http://site.com, css/main.css)
                // TODO improve to ensure no // or /// ever happen
                result.push_str(from);
                result.push_str("/");
                result.push_str(to);
            }
        } else { // It's a local resource (fs)
            // TODO improve to ensure no // or /// ever happen
            // TODO for fs use basepath instead of $from
            result.push_str(from);
            result.push_str("/");
            result.push_str(to);
        }
    }

    Ok(result)
}

pub fn url_is_data(url: &str) -> Result<bool, String> {
    match Url::parse(&url) {
        Ok(parsed_url) => Ok(parsed_url.scheme() == "data"),
        Err(err) => return Err(format!("{}", err.to_string())),
    }
}

pub fn retrieve_asset(url: &str, as_dataurl: bool, as_mime: &str) -> Result<String, reqwest::Error> {
    if url_is_data(&url).unwrap() {
        Ok(url.to_string())
    } else {
        let mut response = reqwest::get(url)?;

        if as_dataurl {
            // Convert response into a byte array
            let mut data: Vec<u8> = vec![];
            response.copy_to(&mut data)?;

            // Attempt to obtain MIME type by reading the Content-Type header
            let mut mimetype = as_mime;
            if as_mime == "" {
                mimetype = response.headers()
                    .get(CONTENT_TYPE)
                    .and_then(|header| header.to_str().ok())
                    .unwrap_or(&as_mime);
            }

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
        assert!(is_url("https://www.rust-lang.org/"));
        assert!(is_url("http://kernel.org"));
        assert!(!is_url("./index.html"));
        assert!(!is_url("some-local-page.htm"));
        assert!(!is_url("ftp://1.2.3.4/www/index.html"));
        assert!(!is_url("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"));
    }

    #[test]
    fn test_resolve_url() -> Result<(), ParseError> {
        let resolved_url = resolve_url("https://www.kernel.org", "../category/signatures.html")?;
        assert_eq!(resolved_url.as_str(), "https://www.kernel.org/../category/signatures.html");

        let resolved_url = resolve_url("https://www.kernel.org", "category/signatures.html")?;
        assert_eq!(resolved_url.as_str(), "https://www.kernel.org/category/signatures.html");

        let resolved_url = resolve_url("saved_page.htm", "https://www.kernel.org/category/signatures.html")?;
        assert_eq!(resolved_url.as_str(), "https://www.kernel.org/category/signatures.html");

        let resolved_url = resolve_url("https://www.kernel.org", "//www.kernel.org/theme/images/logos/tux.png")?;
        assert_eq!(resolved_url.as_str(), "https://www.kernel.org/theme/images/logos/tux.png");

        let resolved_url = resolve_url("https://www.kernel.org/category/signatures.html", "/theme/images/logos/tux.png")?;
        assert_eq!(resolved_url.as_str(), "https://www.kernel.org/theme/images/logos/tux.png");

        Ok(())
    }

    #[test]
    fn test_url_is_data() {
        assert!(url_is_data("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h").unwrap_or(false));
        assert!(!url_is_data("https://kernel.org").unwrap_or(false));
        assert!(!url_is_data("//kernel.org").unwrap_or(false));
    }
}
