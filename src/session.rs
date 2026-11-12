use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::Utc;
use encoding_rs::Encoding;
use markup5ever_rcdom::Handle;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

use crate::cache::{Cache, Exchange};
use crate::cookies::Cookie;
use crate::core::{
    MonolithOptions, MonolithOutputFormat, detect_media_type, print_error_message,
    print_info_message,
};
use crate::mhtml::MhtmlArchive;
use crate::net::{self, PrefetchProgress};
use crate::url::{Url, clean_url, domain_is_within_domain, parse_data_url};
use crate::warc::WarcStream;

/// Provides what the result gets written into, given the title of the document (if it has one).
pub type OutputOpener = Box<dyn FnOnce(Option<&str>) -> io::Result<Box<dyn Write>>>;

/// Where the result goes.
pub(crate) enum Output {
    Memory(Vec<u8>),
    Stream(Box<dyn Write>),
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::Memory(data) => data.write(buf),
            Output::Stream(stream) => stream.write(buf),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Output::Memory(data) => data.write_all(buf),
            Output::Stream(stream) => stream.write_all(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::Memory(_) => Ok(()),
            Output::Stream(stream) => stream.flush(),
        }
    }
}

pub struct Session {
    /// MHTML archive the document came from; when loaded, assets get retrieved from it (and only from it)
    archive: Option<MhtmlArchive>,
    /// Whether the document came from an archive that only has original responses in it
    is_offline: bool,
    pub asset_urls: Vec<String>,
    cache: Option<Cache>,
    client: Client,
    pub(crate) cookies: Option<Vec<Cookie>>,
    /// Where the result gets written, the result is provided as is if there's nowhere to write it
    destination: Option<OutputOpener>,
    /// Encoding of the document (or stylesheet) currently being processed, if it declares one.
    /// Stylesheets that don't specify their own encoding get decoded using it.
    pub environment_encoding: Option<&'static Encoding>,
    pub options: MonolithOptions,
    /// What the result has been written into so far, None until it's known what to call it
    output: Option<Output>,
    /// What went wrong while writing the result, if anything did
    output_error: Option<io::Error>,
    /// Contents of assets that got modified, integrity of which is yet to be checked
    original_data: HashMap<String, Vec<u8>>,
    /// Receives the encoding the document ends up being saved in, if set
    pub output_encoding: Option<Arc<OnceLock<String>>>,
    /// Gets updated as assets are being prefetched, if set
    pub prefetch_progress: Option<Arc<PrefetchProgress>>,
    /// URLs of everything that has been recorded
    recorded_urls: HashSet<String>,
    /// Markup to put right after the opening tag of HEAD
    pub(crate) replay_markup: Option<String>,
    /// Title to name the result by, instead of what the document says
    pub(crate) title: Option<String>,
    /// Records exchanges as they come, once the result has somewhere to go
    warc: Option<WarcStream>,
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
                let is_recording = options.output_format.is_warc();
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
                    // Archives keep every redirect, they get followed by hand to be recorded
                    if is_recording {
                        attempt.stop()
                    } else if !allowed {
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
            is_offline: false,
            cache,
            cookies,
            client,
            destination: None,
            environment_encoding: None,
            options,
            output: None,
            output_encoding: None,
            output_error: None,
            original_data: HashMap::new(),
            prefetch_progress: None,
            recorded_urls: HashSet::new(),
            replay_markup: None,
            title: None,
            warc: None,
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

    pub fn is_recording(&self) -> bool {
        self.options.output_format.is_warc()
    }

    /// Makes the result get written as it's being made, into whatever given function provides
    /// once it's known what the document is called. Without it, the result is provided as is.
    pub fn set_destination(&mut self, open: OutputOpener) {
        self.destination = Some(open);
    }

    /// Gets the result somewhere to go, under given title. Whatever has been recorded so far
    /// goes there first.
    pub(crate) fn open_output(&mut self, title: Option<&str>) -> io::Result<()> {
        if self.output.is_some() {
            return Ok(());
        }
        self.output = Some(match self.destination.take() {
            Some(open) => Output::Stream(open(title)?),
            None => Output::Memory(vec![]),
        });

        if self.is_recording() {
            let mut warc =
                WarcStream::new(self.options.output_format == MonolithOutputFormat::WARCGZ);
            let count = self
                .cache
                .as_ref()
                .map_or(0, |cache| cache.exchange_count());
            for index in 0..count {
                if let Some(exchange) = self.cache.as_ref().and_then(|cache| cache.exchange(index))
                {
                    warc.add(&exchange);
                }
                let data = warc.take();
                self.write_output(&data);
            }
            let data = warc.take();
            self.write_output(&data);
            if let Some(cache) = self.cache.as_mut() {
                cache.clear_exchanges();
            }
            self.flush_output();
            self.warc = Some(warc);
        }

        Ok(())
    }

    /// Adds to the result, the first thing that fails to get written is what finish_output() reports.
    pub(crate) fn write_output(&mut self, data: &[u8]) {
        if self.output_error.is_some() || data.is_empty() {
            return;
        }
        if let Some(Err(error)) = self.output.as_mut().map(|output| output.write_all(data)) {
            self.output_error = Some(error);
        }
    }

    /// Makes what has been written so far show up wherever it's going.
    fn flush_output(&mut self) {
        if self.output_error.is_some() {
            return;
        }
        if let Some(Err(error)) = self.output.as_mut().map(|output| output.flush()) {
            self.output_error = Some(error);
        }
    }

    /// For writing into the result directly, put_output() has to give it back.
    pub(crate) fn take_output(&mut self, title: Option<&str>) -> io::Result<Output> {
        self.open_output(title)?;
        Ok(self.output.take().unwrap())
    }

    pub(crate) fn put_output(&mut self, output: Output) {
        self.output = Some(output);
    }

    /// Makes sure all of the result has been written, provides it if it had nowhere to go.
    pub(crate) fn finish_output(&mut self, title: Option<&str>) -> io::Result<Vec<u8>> {
        self.open_output(title)?;
        if let Some(error) = self.output_error.take() {
            return Err(error);
        }

        match self.output.take() {
            Some(Output::Memory(data)) => Ok(data),
            Some(Output::Stream(mut stream)) => {
                stream.flush()?;
                Ok(vec![])
            }
            None => Ok(vec![]),
        }
    }

    /// Keeps a record of how something was obtained, for formats that keep that.
    pub fn record_exchange(&mut self, exchange: Exchange) {
        if !self.is_recording() {
            return;
        }
        self.recorded_urls.insert(exchange.url.clone());

        if let Some(warc) = self.warc.as_mut() {
            // The result has somewhere to go already, there's no need to keep it around
            warc.add(&exchange);
            let data = warc.take();
            self.write_output(&data);
            // Records are whole, and big enough to not be written any other way
            self.flush_output();
        } else {
            self.cache
                .get_or_insert_with(|| Cache::new(0, None))
                .add_exchange(exchange);
        }
    }

    /// Records something that has been obtained without asking a server for it.
    pub fn record_resource(&mut self, url: &Url, data: &[u8], media_type: &str, charset: &str) {
        let url = clean_url(url.clone()).to_string();
        if !self.is_recording() || self.has_exchange(&url) {
            return;
        }

        self.record_exchange(Exchange {
            body: data.to_vec(),
            content_type: match charset.is_empty() {
                true => media_type.to_string(),
                false => format!("{}; charset={}", media_type, charset),
            },
            date: Utc::now(),
            method: "GET".to_string(),
            url,
            ..Default::default()
        });
    }

    pub fn has_exchange(&self, url: &str) -> bool {
        self.recorded_urls.contains(url)
    }

    pub fn cache(&self) -> Option<&Cache> {
        self.cache.as_ref()
    }

    /// Makes the session retrieve assets from the given MHTML archive, instead of the network or filesystem.
    pub fn load_archive(&mut self, archive: MhtmlArchive) {
        self.archive = Some(archive);
    }

    pub fn has_archive(&self) -> bool {
        self.archive.is_some() || self.is_offline
    }

    /// Keeps the session from going online: what it needs has to be in the cache already.
    pub fn go_offline(&mut self) {
        self.is_offline = true;
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
            || self.options.output_format == MonolithOutputFormat::MHTML
            || self.cache.is_none()
            || self.archive.is_some()
            || self.is_offline
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

        for entry in entries {
            if let Some(cache) = self.cache.as_mut() {
                cache.set(&entry.key, &entry.data, entry.media_type, entry.charset);
            }
            for exchange in entry.exchanges {
                self.record_exchange(exchange);
            }
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
                    self.record_resource(&asset.1, &asset.0, &asset.2, &asset.3);

                    Ok(asset)
                }
                Some(_) => {
                    // Provoke error
                    Err(self.client.get("").send().unwrap_err())
                }
                None => {
                    if !self.options.silent {
                        print_error_message(&format!("{} (not found in archive)", cache_key));
                    }

                    // Provoke error
                    Err(self.client.get("").send().unwrap_err())
                }
            }
        } else if url.scheme() == "file" && !self.is_offline {
            // Check if parent_url is also a file:// URL (if not, then we don't embed the asset)
            if parent_url.scheme() != "file" {
                if !self.options.silent {
                    print_error_message(&format!("{} (security error)", cache_key));
                }

                // Provoke error
                self.client.get("").send()?;
            }

            let path_buf: PathBuf = url.to_file_path().unwrap().clone();
            let path: &Path = path_buf.as_path();
            if path.exists() {
                if path.is_dir() {
                    if !self.options.silent {
                        print_error_message(&format!("{} (is a directory)", cache_key));
                    }

                    // Provoke error
                    Err(self.client.get("").send().unwrap_err())
                } else {
                    if !self.options.silent {
                        print_info_message(&cache_key.to_string());
                    }

                    let file_blob: Vec<u8> = fs::read(path).expect("unable to read file");
                    let media_type = detect_media_type(&file_blob, url);
                    // Targets get recorded for what's in them, they may be archives
                    if parent_url != url {
                        self.record_resource(url, &file_blob, &media_type, "");
                    }

                    Ok((file_blob, url.clone(), media_type, "".to_string()))
                }
            } else {
                if !self.options.silent {
                    print_error_message(&format!("{} (file not found)", url));
                }

                // Provoke error
                Err(self.client.get("").send().unwrap_err())
            }
        } else if !net::is_domain_allowed(url, &self.options) {
            // Provoke error
            Err(self.client.get("").send().unwrap_err())
        } else if let Some((data, media_type, charset)) = self
            .cache
            .as_ref()
            .filter(|cache| cache.contains_key(&cache_key))
            .and_then(|cache| cache.get(&cache_key).ok())
        {
            // URL is in cache, we get and return it
            Ok((data, url.clone(), media_type, charset))
        } else if self.is_offline {
            if !self.options.silent {
                print_error_message(&format!("{} (not found in archive)", cache_key));
            }

            // Provoke error
            Err(self.client.get("").send().unwrap_err())
        } else {
            if let Some(domains) = &self.options.domains {
                let domain_matches = domains
                    .iter()
                    .any(|d| domain_is_within_domain(url.host_str().unwrap(), d.trim()));
                if domain_matches != self.options.whitelist_domains {
                    return Err(self.client.get("").send().unwrap_err());
                }
            }

            let mut exchanges: Vec<Exchange> = vec![];
            let is_recording = self.is_recording();
            let result = net::fetch_remote_asset(
                &self.client,
                self.cookies.as_deref(),
                &self.options,
                parent_url,
                url,
                is_recording.then_some(&mut exchanges),
            );
            for exchange in exchanges {
                self.record_exchange(exchange);
            }
            let (data, response_url, media_type, charset) = result?;

            // Add retrieved resource to cache
            if let Some(cache) = self.cache.as_mut() {
                let new_cache_key: String = clean_url(response_url.clone()).to_string();
                cache.set(&new_cache_key, &data, media_type.clone(), charset.clone());
            }

            Ok((data, response_url, media_type, charset))
        }
    }
}
