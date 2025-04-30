use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE, REFERER, USER_AGENT};

use crate::cache::Cache;
use crate::cookies::Cookie;
use crate::core::{
    detect_media_type, parse_content_type, print_error_message, print_info_message, MonolithOptions,
};
use crate::url::{clean_url, domain_is_within_domain, get_referer_url, parse_data_url, Url};

pub struct Session {
    cache: Option<Cache>,
    client: Client,
    cookies: Option<Vec<Cookie>>,
    pub options: MonolithOptions,
    urls: Vec<String>,
}

impl Session {
    pub fn new(
        cache: Option<Cache>,
        cookies: Option<Vec<Cookie>>,
        options: MonolithOptions,
    ) -> Self {
        let mut header_map = HeaderMap::new();
        if let Some(user_agent) = &options.user_agent {
            header_map.insert(
                USER_AGENT,
                HeaderValue::from_str(user_agent).expect("Invalid User-Agent header specified"),
            );
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(if options.timeout > 0 {
                options.timeout
            } else {
                // We have to specify something that eventually makes the program fail
                // (prevent it from hanging forever)
                600 // 10 minutes in seconds
            }))
            .danger_accept_invalid_certs(options.insecure)
            .default_headers(header_map)
            .build()
            .expect("Failed to initialize HTTP client");

        Session {
            cache,
            cookies,
            client,
            options,
            urls: Vec::new(),
        }
    }

    pub fn retrieve_asset(
        &mut self,
        parent_url: &Url,
        url: &Url,
    ) -> Result<(Vec<u8>, Url, String, String), reqwest::Error> {
        let cache_key: String = clean_url(url.clone()).as_str().to_string();

        if !self.urls.contains(&url.as_str().to_string()) {
            self.urls.push(url.as_str().to_string());
        }

        if url.scheme() == "data" {
            let (media_type, charset, data) = parse_data_url(url);
            Ok((data, url.clone(), media_type, charset))
        } else if url.scheme() == "file" {
            // Check if parent_url is also a file:// URL (if not, then we don't embed the asset)
            if parent_url.scheme() != "file" {
                if !self.options.silent {
                    print_error_message(&format!("{} (security error)", &cache_key));
                }

                // Provoke error
                self.client.get("").send()?;
            }

            let path_buf: PathBuf = url.to_file_path().unwrap().clone();
            let path: &Path = path_buf.as_path();
            if path.exists() {
                if path.is_dir() {
                    if !self.options.silent {
                        print_error_message(&format!("{} (is a directory)", &cache_key));
                    }

                    // Provoke error
                    Err(self.client.get("").send().unwrap_err())
                } else {
                    if !self.options.silent {
                        print_info_message(&cache_key.to_string());
                    }

                    let file_blob: Vec<u8> = fs::read(path).expect("unable to read file");

                    Ok((
                        file_blob.clone(),
                        url.clone(),
                        detect_media_type(&file_blob, url),
                        "".to_string(),
                    ))
                }
            } else {
                if !self.options.silent {
                    print_error_message(&format!("{} (file not found)", &url));
                }

                // Provoke error
                Err(self.client.get("").send().unwrap_err())
            }
        } else if self.cache.is_some() && self.cache.as_ref().unwrap().contains_key(&cache_key) {
            // URL is in cache, we get and return it
            if !self.options.silent {
                print_info_message(&format!("{} (from cache)", &cache_key));
            }

            Ok((
                self.cache
                    .as_ref()
                    .unwrap()
                    .get(&cache_key)
                    .unwrap()
                    .0
                    .to_vec(),
                url.clone(),
                self.cache.as_ref().unwrap().get(&cache_key).unwrap().1,
                self.cache.as_ref().unwrap().get(&cache_key).unwrap().2,
            ))
        } else {
            if let Some(domains) = &self.options.domains {
                let domain_matches = domains
                    .iter()
                    .any(|d| domain_is_within_domain(url.host_str().unwrap(), d.trim()));
                if (self.options.blacklist_domains && domain_matches)
                    || (!self.options.blacklist_domains && !domain_matches)
                {
                    return Err(self.client.get("").send().unwrap_err());
                }
            }

            // URL not in cache, we retrieve the file
            let mut headers = HeaderMap::new();
            if self.cookies.is_some() && !self.cookies.as_ref().unwrap().is_empty() {
                for cookie in self.cookies.as_ref().unwrap() {
                    if !cookie.is_expired() && cookie.matches_url(url.as_str()) {
                        let cookie_header_value: String = cookie.name.clone() + "=" + &cookie.value;
                        headers
                            .insert(COOKIE, HeaderValue::from_str(&cookie_header_value).unwrap());
                    }
                }
            }
            // Add referer header for page resource requests
            if ["https", "http"].contains(&parent_url.scheme()) && parent_url != url {
                headers.insert(
                    REFERER,
                    HeaderValue::from_str(get_referer_url(parent_url.clone()).as_str()).unwrap(),
                );
            }
            match self.client.get(url.as_str()).headers(headers).send() {
                Ok(response) => {
                    if !self.options.ignore_errors && response.status() != reqwest::StatusCode::OK {
                        if !self.options.silent {
                            print_error_message(&format!("{} ({})", &cache_key, response.status()));
                        }

                        // Provoke error
                        return Err(self.client.get("").send().unwrap_err());
                    }

                    let response_url: Url = response.url().clone();

                    if !self.options.silent {
                        if url.as_str() == response_url.as_str() {
                            print_info_message(&cache_key.to_string());
                        } else {
                            print_info_message(&format!("{} -> {}", &cache_key, &response_url));
                        }
                    }

                    // Attempt to obtain media type and charset by reading Content-Type header
                    let content_type: &str = response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|header| header.to_str().ok())
                        .unwrap_or("");

                    let (media_type, charset, _is_base64) = parse_content_type(content_type);

                    // Convert response into a byte array
                    let mut data: Vec<u8> = vec![];
                    match response.bytes() {
                        Ok(b) => {
                            data = b.to_vec();
                        }
                        Err(error) => {
                            if !self.options.silent {
                                print_error_message(&format!("{}", error));
                            }
                        }
                    }

                    // Add retrieved resource to cache
                    if self.cache.is_some() {
                        let new_cache_key: String = clean_url(response_url.clone()).to_string();

                        self.cache.as_mut().unwrap().set(
                            &new_cache_key,
                            &data,
                            media_type.clone(),
                            charset.clone(),
                        );
                    }

                    // Return
                    Ok((data, response_url, media_type, charset))
                }
                Err(error) => {
                    if !self.options.silent {
                        print_error_message(&format!("{} ({})", &cache_key, error));
                    }

                    Err(self.client.get("").send().unwrap_err())
                }
            }
        }
    }
}
