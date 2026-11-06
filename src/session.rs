use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use encoding_rs::Encoding;
use markup5ever_rcdom::Handle;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

use crate::cache::Cache;
use crate::cookies::Cookie;
use crate::core::{
    MonolithOptions, MonolithOutputFormat, detect_media_type, print_error_message,
    print_info_message,
};
use crate::mhtml::MhtmlArchive;
use crate::net::{self, PrefetchProgress};
use crate::url::{Url, clean_url, domain_is_within_domain, parse_data_url};

pub struct Session {
    /// MHTML archive the document came from; when loaded, assets get retrieved from it (and only from it)
    archive: Option<MhtmlArchive>,
    pub asset_urls: Vec<String>,
    cache: Option<Cache>,
    client: Client,
    pub(crate) cookies: Option<Vec<Cookie>>,
    /// Encoding of the document (or stylesheet) currently being processed, if it declares one.
    /// Stylesheets that don't specify their own encoding get decoded using it.
    pub environment_encoding: Option<&'static Encoding>,
    pub options: MonolithOptions,
    /// Contents of assets that got modified, integrity of which is yet to be checked
    original_data: HashMap<String, Vec<u8>>,
    /// Receives the encoding the document ends up being saved in, if set
    pub output_encoding: Option<Arc<OnceLock<String>>>,
    /// Gets updated as assets are being prefetched, if set
    pub prefetch_progress: Option<Arc<PrefetchProgress>>,
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
            .redirect({
                // Stop redirects into blocked domains before the request is even made
                let domains = options.domains.clone();
                let whitelist_domains = options.whitelist_domains;
                reqwest::redirect::Policy::custom(move |attempt| {
                    let allowed = match (&domains, attempt.url().host_str()) {
                        (Some(domains), Some(host)) => {
                            domains
                                .iter()
                                .any(|d| domain_is_within_domain(host, d.trim()))
                                == whitelist_domains
                        }
                        _ => true,
                    };
                    if !allowed {
                        attempt.error("redirect target is blocked")
                    } else if attempt.previous().len() >= 10 {
                        attempt.error("too many redirects")
                    } else {
                        attempt.follow()
                    }
                })
            })
            .build()
            .expect("Failed to initialize HTTP client");

        Session {
            archive: None,
            asset_urls: Vec::new(),
            cache,
            cookies,
            client,
            environment_encoding: None,
            options,
            output_encoding: None,
            original_data: HashMap::new(),
            prefetch_progress: None,
        }
    }

    /// Makes the given asset available without requesting it: pages rendered by a browser come with
    /// everything the browser has already downloaded, and that may be all it can be obtained from.
    /// Keeps what an asset was before it got modified, for checking its integrity later on.
    pub fn add_original_data(&mut self, url: &Url, data: Vec<u8>) {
        self.original_data
            .insert(clean_url(url.clone()).to_string(), data);
    }

    pub fn original_data(&self, url: &Url) -> Option<&Vec<u8>> {
        self.original_data.get(clean_url(url.clone()).as_str())
    }

    pub fn add_asset(&mut self, url: &Url, data: &Vec<u8>, media_type: String, charset: String) {
        let key: String = clean_url(url.clone()).as_str().to_string();

        self.cache
            .get_or_insert_with(|| Cache::new(0, None))
            .set(&key, data, media_type, charset);
    }

    /// Makes the session retrieve assets from the given MHTML archive, instead of the network or filesystem.
    pub fn load_archive(&mut self, archive: MhtmlArchive) {
        self.archive = Some(archive);
    }

    pub fn has_archive(&self) -> bool {
        self.archive.is_some()
    }

    pub fn log_asset_url(&mut self, url: &Url) {
        if !self.asset_urls.contains(&url.as_str().to_string()) {
            self.asset_urls.push(url.as_str().to_string());
        }
    }

    /// Downloads every asset the walk is about to request on a thread pool and puts
    /// the results into the cache. A no-op without a cache, with fewer than 2 threads,
    /// for non-HTML output (MHTML handles assets differently, see css.rs), or with an MHTML archive loaded
    /// (everything is already in there).
    pub fn prefetch_assets(&mut self, document_url: &Url, document: &Handle) {
        if self.options.threads < 2
            || self.options.output_format != MonolithOutputFormat::HTML
            || self.cache.is_none()
            || self.archive.is_some()
        {
            return;
        }

        let cache = self.cache.as_ref().unwrap();
        let entries = net::prefetch_assets(
            &self.client,
            self.cookies.as_deref(),
            &self.options,
            document_url,
            document,
            self.prefetch_progress.as_deref(),
            |key| cache.contains_key(key),
        );

        let cache = self.cache.as_mut().unwrap();
        for entry in entries {
            cache.set(&entry.key, &entry.data, entry.media_type, entry.charset);
        }
    }

    pub fn retrieve_asset(
        &mut self,
        parent_url: &Url,
        url: &Url,
    ) -> Result<(Vec<u8>, Url, String, String), reqwest::Error> {
        let cache_key: String = clean_url(url.clone()).as_str().to_string();

        if url.scheme() == "data" {
            let (media_type, charset, data) = parse_data_url(url);
            Ok((data, url.clone(), media_type, charset))
        } else if let Some(archive) = &self.archive {
            // Documents that come from MHTML archives get assets from the archive, never from the network or filesystem
            let asset = archive.get(url).map(|asset| {
                (
                    asset.data.clone(),
                    asset.url.clone().unwrap_or_else(|| url.clone()),
                    asset.media_type.clone(),
                    asset.charset.clone(),
                )
            });

            match asset {
                Some(asset) if net::is_domain_allowed(url, &self.options) => {
                    if !self.options.silent {
                        print_info_message(&cache_key);
                    }

                    Ok(asset)
                }
                Some(_) => {
                    // Provoke error
                    Err(self.client.get("").send().unwrap_err())
                }
                None => {
                    if !self.options.silent {
                        print_error_message(&format!("{} (not found in archive)", &cache_key));
                    }

                    // Provoke error
                    Err(self.client.get("").send().unwrap_err())
                }
            }
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
        } else if !net::is_domain_allowed(url, &self.options) {
            // Provoke error
            Err(self.client.get("").send().unwrap_err())
        } else if self.cache.is_some() && self.cache.as_ref().unwrap().contains_key(&cache_key) {
            // URL is in cache, we get and return it

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
                if domain_matches != self.options.whitelist_domains {
                    return Err(self.client.get("").send().unwrap_err());
                }
            }

            let (data, response_url, media_type, charset) = net::fetch_remote_asset(
                &self.client,
                self.cookies.as_deref(),
                &self.options,
                parent_url,
                url,
            )?;

            // Add retrieved resource to cache
            if let Some(cache) = self.cache.as_mut() {
                let new_cache_key: String = clean_url(response_url.clone()).to_string();
                cache.set(&new_cache_key, &data, media_type.clone(), charset.clone());
            }

            Ok((data, response_url, media_type, charset))
        }
    }
}
