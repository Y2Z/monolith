use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

pub struct Cookie {
    pub domain: String,
    pub include_subdomains: bool,
    pub path: String,
    pub https_only: bool,
    pub expires: u64,
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub enum CookieFileContentsParseError {
    InvalidHeader,
}

impl Cookie {
    pub fn is_expired(&self) -> bool {
        if self.expires == 0 {
            return false; // Session, never expires
        }

        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        self.expires < since_the_epoch.as_secs()
    }

    pub fn matches_url(&self, url: &str) -> bool {
        match Url::parse(&url) {
            Ok(url) => {
                // Check protocol scheme
                match url.scheme() {
                    "http" => {
                        if self.https_only {
                            return false;
                        }
                    }
                    "https" => {}
                    _ => {
                        // Should never match URLs of protocols other than HTTP(S)
                        return false;
                    }
                }

                // Check host
                if let Some(url_host) = url.host_str() {
                    if self.domain.starts_with(".") && self.include_subdomains {
                        if !url_host.to_lowercase().ends_with(&self.domain)
                            && !url_host
                                .eq_ignore_ascii_case(&self.domain[1..self.domain.len() - 1])
                        {
                            return false;
                        }
                    } else {
                        if !url_host.eq_ignore_ascii_case(&self.domain) {
                            return false;
                        }
                    }
                } else {
                    return false;
                }

                // Check path
                if !url.path().eq_ignore_ascii_case(&self.path)
                    && !url.path().starts_with(&self.path)
                {
                    return false;
                }
            }
            Err(_) => {
                return false;
            }
        }

        true
    }
}

pub fn parse_cookie_file_contents(
    cookie_file_contents: &str,
) -> Result<Vec<Cookie>, CookieFileContentsParseError> {
    let mut cookies: Vec<Cookie> = Vec::new();

    for (i, line) in cookie_file_contents.lines().enumerate() {
        if i == 0 {
            // Parsing first line
            if !line.eq("# HTTP Cookie File") && !line.eq("# Netscape HTTP Cookie File") {
                return Err(CookieFileContentsParseError::InvalidHeader);
            }
        } else {
            // Ignore comment lines
            if line.starts_with("#") {
                continue;
            }

            // Attempt to parse values
            let mut fields = line.split("\t");
            if fields.clone().count() != 7 {
                continue;
            }
            cookies.push(Cookie {
                domain: fields.next().unwrap().to_string().to_lowercase(),
                include_subdomains: fields.next().unwrap().to_string() == "TRUE",
                path: fields.next().unwrap().to_string(),
                https_only: fields.next().unwrap().to_string() == "TRUE",
                expires: fields.next().unwrap().parse::<u64>().unwrap(),
                name: fields.next().unwrap().to_string(),
                value: fields.next().unwrap().to_string(),
            });
        }
    }

    Ok(cookies)
}
