use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
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
use crate::cors::{CorsRequest, PageOrigin, RequestMode};
use crate::csp::{Policies, Request, Saved};
use crate::mhtml::MhtmlArchive;
use crate::net::{
    self, Credentials, PrefetchProgress, recorded_content_security_policy, without_credentials,
};
use crate::url::{Url, clean_url, domain_is_within_domain, parse_data_url};
use crate::warc::WarcStream;

/// URLs of what has been recorded into an archive: sessions that write into the same archive
/// share them (see Session::share_records()), for nothing to get recorded more than once.
#[derive(Clone, Default)]
pub struct Records(Arc<Mutex<HashSet<String>>>);

impl Records {
    fn urls(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.0.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// Whether it's new, it's what has been recorded from now on either way.
    fn insert(&self, url: &str) -> bool {
        self.urls().insert(url.to_string())
    }

    pub fn contains(&self, url: &str) -> bool {
        self.urls().contains(url)
    }
}

/// Provides what the result gets written into, given the title of the document (if it has one).
pub type OutputOpener = Box<dyn FnOnce(Option<&str>) -> io::Result<Box<dyn Write + Send>> + Send>;

/// Where the result goes.
pub(crate) enum Output {
    Memory(Vec<u8>),
    Stream(Box<dyn Write + Send>),
}

/// How deeply assets can be nested before expansion gets cut off. Distinct URLs can always be
/// produced dynamically, so tracking what's being expanded is not enough on its own.
pub(crate) const MAX_EXPANSION_DEPTH: usize = 32;

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
    /// How logged assets get requested, for those that don't get requested the usual way
    asset_modes: HashMap<String, RequestMode>,
    cache: Option<Cache>,
    client: Client,
    pub(crate) cookies: Option<Vec<Cookie>>,
    /// Credentials for HTTP authentication, given along with URLs
    pub(crate) credentials: Credentials,
    /// Where the result gets written, the result is provided as is if there's nowhere to write it
    destination: Option<OutputOpener>,
    /// Files that other documents get saved in, by their URLs (without fragments)
    linked_files: HashMap<String, Url>,
    /// File this document gets saved in, for linking to other documents relative to it
    own_file: Option<Url>,
    /// Whether links to other documents have to be absolute: relative ones wouldn't work for
    /// documents that have a base URL of their own
    pub(crate) links_are_absolute: bool,
    /// Encoding of the document (or stylesheet) currently being processed, if it declares one.
    /// Stylesheets that don't specify their own encoding get decoded using it.
    pub environment_encoding: Option<&'static Encoding>,
    /// Normalized URLs of documents, stylesheets, and SVGs that are being expanded right now;
    /// coming across one of them again means going in circles.
    expanding_urls: HashSet<String>,
    /// How deeply assets are being expanded right now.
    expansion_depth: usize,
    /// URL the document was requested by before it redirected, if it did; the document may still
    /// refer to itself by it.
    redirected_target: Option<Url>,
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
    recorded_urls: Records,
    /// URL of the page to save out of an archive (the first page, if there's none)
    pub(crate) page_url: Option<String>,
    /// Archives to take everything from, instead of going online
    pub(crate) source_archives: Vec<Arc<Vec<u8>>>,
    /// Origin of the document being saved, None for documents that have none (CORS doesn't
    /// apply to those)
    pub(crate) page_origin: Option<PageOrigin>,
    /// Whether the document has been put together by a browser (its assets are what the browser
    /// has received)
    pub(crate) is_captured: bool,
    /// What captured and archived assets have been received with, by URL (see record_headers())
    recorded_headers: HashMap<String, (Url, Vec<(String, String)>)>,
    /// Markup to put right after the opening tag of HEAD
    pub(crate) replay_markup: Option<String>,
    /// Policies the document being processed right now has to abide by (see csp.rs)
    pub(crate) content_security: Policies,
    /// What the saved document is made of, that its policies have to allow
    pub(crate) saved_content: Saved,
    /// Whether elements of the document being processed get named by nonce: the document (or
    /// what it's in) has policies
    pub(crate) names_elements: bool,
    /// What STYLE elements a browser has made out of constructed stylesheets are marked with,
    /// those aren't up to policies
    pub(crate) adopted_style_marker: Option<String>,
    /// What Content-Security-Policy headers have said, by URL (see received_policy())
    received_policies: HashMap<String, String>,
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
            asset_modes: HashMap::new(),
            is_offline: false,
            cache,
            cookies,
            client,
            credentials: Credentials::default(),
            destination: None,
            environment_encoding: None,
            expanding_urls: HashSet::new(),
            expansion_depth: 0,
            redirected_target: None,
            linked_files: HashMap::new(),
            links_are_absolute: false,
            own_file: None,
            options,
            output: None,
            output_encoding: None,
            output_error: None,
            original_data: HashMap::new(),
            prefetch_progress: None,
            page_url: None,
            recorded_urls: Records::default(),
            source_archives: vec![],
            page_origin: None,
            is_captured: false,
            recorded_headers: HashMap::new(),
            replay_markup: None,
            content_security: Policies::default(),
            saved_content: Saved::new(),
            names_elements: false,
            adopted_style_marker: None,
            received_policies: HashMap::new(),
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
        self.add_asset_with_final_url(url, None, data, media_type, charset);
    }

    /// Makes the given asset available without requesting it, remembering the URL it ended up at
    /// if it was requested by another one (after a redirect).
    pub fn add_asset_with_final_url(
        &mut self,
        url: &Url,
        final_url: Option<&Url>,
        data: &Vec<u8>,
        media_type: String,
        charset: String,
    ) {
        let key: String = self.credentials.cache_key(url);
        let final_url: Option<String> =
            final_url.map(|final_url| clean_url(without_credentials(final_url)).to_string());

        self.cache
            .get_or_insert_with(|| Cache::new(0, None))
            .set_with_final_url(&key, data, media_type, charset, final_url);
    }

    /// What an asset is tracked by while it's being expanded: fragments don't make a different
    /// asset, and neither do credentials, which don't change what a URL points at.
    fn expansion_key(&self, url: &Url) -> String {
        clean_url(without_credentials(url)).to_string()
    }

    /// Whether assets are nested too deeply to go any deeper.
    pub(crate) fn is_too_deep(&self) -> bool {
        self.expansion_depth >= MAX_EXPANSION_DEPTH
    }

    /// Whether `url` is being expanded right now, or assets are nested too deeply already.
    pub(crate) fn is_expanding(&self, url: &Url) -> bool {
        self.expansion_depth >= MAX_EXPANSION_DEPTH
            || self.expanding_urls.contains(&self.expansion_key(url))
    }

    /// Marks `url` as being expanded until `leave_expansion` gets called with the returned key.
    /// Returns None when it's being expanded already, or assets are nested too deeply.
    /// The key is returned rather than recomputed on the way out, because taking credentials from
    /// a nested URL may change what `url` is tracked by.
    pub(crate) fn enter_expansion(&mut self, url: &Url) -> Option<String> {
        if self.is_expanding(url) {
            return None;
        }

        let key = self.expansion_key(url);
        self.expanding_urls.insert(key.clone());
        self.expansion_depth += 1;
        Some(key)
    }

    /// Marks a URL from `enter_expansion` as done being expanded.
    pub(crate) fn leave_expansion(&mut self, key: &str) {
        self.expanding_urls.remove(key);
        self.expansion_depth -= 1;
    }

    /// Keeps the URL an asset was requested by marked as being expanded, in addition to the URL
    /// it ended up at, until `leave_expansion_alias` gets called with the returned key.
    /// Returns None if both URLs are the same, or the requested one is marked already.
    pub(crate) fn enter_expansion_alias(
        &mut self,
        requested_url: &Url,
        final_url: &Url,
    ) -> Option<String> {
        let key = self.expansion_key(requested_url);
        if key == self.expansion_key(final_url) {
            return None;
        }

        self.expanding_urls.insert(key.clone()).then_some(key)
    }

    /// Marks a URL from `enter_expansion_alias` as done being expanded.
    pub(crate) fn leave_expansion_alias(&mut self, key: &str) {
        self.expanding_urls.remove(key);
    }

    /// Remembers where the document was requested from, if that's not where it ended up.
    pub(crate) fn set_redirected_target(&mut self, url: Url) {
        self.redirected_target = Some(url);
    }

    /// Takes where the document was requested from, to be kept from looking like a cycle.
    pub(crate) fn take_redirected_target(&mut self) -> Option<Url> {
        self.redirected_target.take()
    }

    /// Keeps what something was received with, under URLs it's been received for (the last one
    /// being where it ended up): what other origins allowed to be done with it depends on it.
    pub fn record_headers(
        &mut self,
        urls: &[Url],
        final_url: &Url,
        headers: Vec<(String, String)>,
    ) {
        for url in urls {
            self.recorded_headers.insert(
                clean_url(url.clone()).to_string(),
                (final_url.clone(), headers.clone()),
            );
        }
    }

    /// What Content-Security-Policy headers given URL has been received with said, wherever it
    /// has been received from.
    pub fn received_policy(&self, url: &Url) -> Option<String> {
        let key = clean_url(without_credentials(url)).to_string();
        if let Some((_, headers)) = self.recorded_headers.get(&key) {
            return recorded_content_security_policy(headers);
        }
        if let Some(archived) = self.archive.as_ref().and_then(|archive| archive.get(url)) {
            return archived
                .response_headers
                .as_deref()
                .and_then(recorded_content_security_policy);
        }
        self.received_policies.get(&key).cloned().or_else(|| {
            self.cache
                .as_ref()
                .and_then(|cache| cache.policy(&self.credentials.cache_key(url)))
        })
    }

    /// Whether the document's policies allow the request, saying so if they don't.
    pub fn is_allowed(&self, url: &Url, request: &Request) -> bool {
        let is_allowed = self.content_security.allows(
            request.destination,
            url,
            request.nonce,
            request.integrity,
            false,
        );
        if !is_allowed && !self.options.silent {
            print_error_message(&format!(
                "{} (blocked by Content Security Policy)",
                clean_url(without_credentials(url))
            ));
        }
        is_allowed
    }

    /// Same as retrieve(), for what the document's policies have to allow (see csp.rs): None
    /// for what they don't, browsers don't load that.
    pub fn retrieve_for(
        &mut self,
        parent_url: &Url,
        url: &Url,
        mode: RequestMode,
        request: &Request,
    ) -> Result<Option<net::Asset>, reqwest::Error> {
        let url: Url = self.content_security.upgrade(url, request.destination);
        if !self.is_allowed(&url, request) {
            return Ok(None);
        }

        let asset = self.retrieve(parent_url, &url, mode)?;
        // Where it's been redirected to has to be allowed too
        if let Some((_, final_url, _, _)) = &asset
            && clean_url(final_url.clone()) != clean_url(url.clone())
            && !self.content_security.allows(
                request.destination,
                final_url,
                request.nonce,
                request.integrity,
                true,
            )
        {
            if !self.options.silent {
                print_error_message(&format!(
                    "{} (blocked by Content Security Policy)",
                    clean_url(without_credentials(final_url))
                ));
            }
            return Ok(None);
        }
        if asset.is_some() {
            self.saved_content.data.insert(request.destination);
        }

        Ok(asset)
    }

    /// Same as retrieve_asset(), for what the document's policies have to allow.
    pub fn retrieve_asset_for(
        &mut self,
        parent_url: &Url,
        url: &Url,
        request: &Request,
    ) -> Result<net::Asset, reqwest::Error> {
        match self.retrieve_for(parent_url, url, RequestMode::NoCors, request)? {
            Some(asset) => Ok(asset),
            None => Err(self.client.get("").send().unwrap_err()),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.options.output_format.is_warc()
    }

    /// Takes user name and password out of given URL, they go along with everything that gets
    /// retrieved from its origin from now on.
    pub fn take_credentials(&mut self, url: &Url) -> Url {
        self.credentials.take_from(url)
    }

    /// Makes links to other documents lead to files they get saved in: given the file this
    /// document gets saved in, and files of other documents by their URLs.
    pub fn link_files(&mut self, own_file: &Path, files: &[(Url, PathBuf)]) {
        let file_url = |path: &Path| {
            fs::canonicalize(path)
                .ok()
                .and_then(|path| Url::from_file_path(path).ok())
        };

        self.own_file = file_url(own_file);
        self.linked_files = files
            .iter()
            .filter_map(|(url, path)| {
                Some((
                    clean_url(without_credentials(url)).to_string(),
                    file_url(path)?,
                ))
            })
            .collect();
    }

    /// Where a link has to lead, if it's a link to some other document that gets saved.
    pub(crate) fn link_to(&self, url: &Url) -> Option<String> {
        let file = self
            .linked_files
            .get(clean_url(without_credentials(url)).as_str())?;
        let mut file = file.clone();
        file.set_fragment(url.fragment());

        match (&self.own_file, self.links_are_absolute) {
            (Some(own_file), false) => own_file.make_relative(&file),
            _ => Some(file.to_string()),
        }
    }

    /// Makes the session know what other sessions have recorded into the archive they write into,
    /// and the other way around: what's in there already doesn't get recorded again.
    pub fn share_records(&mut self, records: &Records) {
        self.recorded_urls = records.clone();
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
        if !self.recorded_urls.insert(&exchange.url) {
            // It's in the archive already (another session may have put it there)
            return;
        }

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

    /// Makes the session take everything from given archives (see read_archives()), and
    /// nothing from anywhere else: targets are URLs of pages in them.
    pub fn use_archives(&mut self, archives: &[Arc<Vec<u8>>]) {
        self.source_archives = archives.to_vec();
        self.go_offline();
    }

    pub fn uses_archives(&self) -> bool {
        !self.source_archives.is_empty()
    }

    /// Keeps the session from going online: what it needs has to be in the cache already.
    pub fn go_offline(&mut self) {
        self.is_offline = true;
    }

    pub fn log_asset_url(&mut self, url: &Url) {
        self.log_asset_url_as(url, RequestMode::NoCors);
    }

    /// Same as log_asset_url(), for assets that get requested in given mode (see cors.rs).
    pub fn log_asset_url_as(&mut self, url: &Url, mode: RequestMode) {
        if !self.asset_urls.contains(&url.as_str().to_string()) {
            self.asset_urls.push(url.as_str().to_string());
        }
        if mode != RequestMode::NoCors {
            self.asset_modes.insert(url.as_str().to_string(), mode);
        }
    }

    /// How a logged asset gets requested.
    pub(crate) fn asset_mode(&self, url: &str) -> RequestMode {
        self.asset_modes
            .get(url)
            .copied()
            .unwrap_or(RequestMode::NoCors)
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
            net::Requester {
                client: &self.client,
                cookies: self.cookies.as_deref(),
                credentials: &self.credentials,
                cors: None,
            },
            &self.options,
            document_url,
            document,
            self.page_origin.as_ref(),
            self.prefetch_progress.as_deref(),
            |key| cache.contains_key(key),
        );

        for entry in entries {
            if let Some(cache) = self.cache.as_mut() {
                cache.set_with_final_url(
                    &entry.key,
                    &entry.data,
                    entry.media_type,
                    entry.charset,
                    entry.final_url,
                );
                cache.set_policy(&entry.key, entry.policy);
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
        match self.retrieve(parent_url, url, RequestMode::NoCors)? {
            Some(asset) => Ok(asset),
            // Only CORS requests get blocked
            None => Err(self.client.get("").send().unwrap_err()),
        }
    }

    /// Whether what gets retrieved has to be allowed by other origins (see cors.rs): only for
    /// documents that have an origin.
    pub(crate) fn checks_cors(&self) -> bool {
        self.page_origin.is_some()
    }

    /// Same as retrieve_asset(), for requests made in given mode: None for what the page isn't
    /// allowed to use (see cors.rs), browsers drop that.
    pub fn retrieve(
        &mut self,
        parent_url: &Url,
        url: &Url,
        mode: RequestMode,
    ) -> Result<Option<net::Asset>, reqwest::Error> {
        // CORS is about origins, pages without one have nothing to check
        let cors: Option<CorsRequest> = match (mode, &self.page_origin) {
            (RequestMode::Cors { with_credentials }, Some(origin)) => Some(CorsRequest {
                origin: origin.clone(),
                with_credentials,
            }),
            _ => None,
        };
        // Credentials only ever go along with requests, never with URLs
        let url: &Url = &self.credentials.take_from(url);
        let parent_url: &Url = &without_credentials(parent_url);
        let cache_key: String = clean_url(url.clone()).as_str().to_string();
        let cors_suffix: String = cors
            .as_ref()
            .map(CorsRequest::cache_key_suffix)
            .unwrap_or_default();
        let identity_key: String = self.credentials.cache_key(url) + &cors_suffix;

        if url.scheme() == "data" {
            let (media_type, charset, data) = parse_data_url(url);
            Ok(Some((data, url.clone(), media_type, charset)))
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
            // What other origins allowed is known only for what has been received over HTTP
            let is_blocked = match (cors.as_ref(), archive.get(url)) {
                (Some(cors), Some(archived)) => {
                    archived.response_headers.as_ref().is_some_and(|headers| {
                        !cors.allows_recorded(archived.url.as_ref().unwrap_or(url), headers)
                    })
                }
                _ => false,
            };

            match asset {
                Some(_) if is_blocked => {
                    if !self.options.silent {
                        print_error_message(&format!("{} (blocked by CORS)", cache_key));
                    }

                    Ok(None)
                }
                Some(asset) if net::is_domain_allowed(url, &self.options) => {
                    if !self.options.silent {
                        print_info_message(&cache_key);
                    }
                    self.record_resource(&asset.1, &asset.0, &asset.2, &asset.3);

                    Ok(Some(asset))
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

            // Files outside of the page's directory aren't the page's own (the way Firefox has it)
            if let Some(cors) = cors.as_ref()
                && !cors.origin.is_same_origin(url)
            {
                if !self.options.silent {
                    print_error_message(&format!("{} (blocked by CORS)", cache_key));
                }

                return Ok(None);
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

                    Ok(Some((file_blob, url.clone(), media_type, "".to_string())))
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
        } else if let Some(cors) = cors.as_ref()
            && (self.is_captured || self.is_offline)
            && let Some((data, media_type, charset, final_url)) =
                self.cache.as_ref().and_then(|cache| {
                    cache
                        .get_with_final_url(&self.credentials.cache_key(url))
                        .ok()
                })
        {
            // What's been captured or archived is what was received, along with what it was
            // received with
            let is_blocked = self
                .recorded_headers
                .get(&cache_key)
                .is_some_and(|(final_url, headers)| !cors.allows_recorded(final_url, headers));
            if is_blocked {
                if !self.options.silent {
                    print_error_message(&format!("{} (blocked by CORS)", cache_key));
                }

                return Ok(None);
            }
            let final_url = final_url
                .and_then(|final_url| Url::parse(&final_url).ok())
                .unwrap_or_else(|| url.clone());

            Ok(Some((data, final_url, media_type, charset)))
        } else if let Some((data, media_type, charset, final_url)) = self
            .cache
            .as_ref()
            .filter(|cache| cache.contains_key(&identity_key))
            .and_then(|cache| cache.get_with_final_url(&identity_key).ok())
        {
            // URL is in cache, we get and return it
            let final_url = final_url
                .and_then(|final_url| Url::parse(&final_url).ok())
                .unwrap_or_else(|| url.clone());

            Ok(Some((data, final_url, media_type, charset)))
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
            let result = net::fetch_cors_asset(
                &net::Requester {
                    client: &self.client,
                    cookies: self.cookies.as_deref(),
                    credentials: &self.credentials,
                    cors: cors.as_ref(),
                },
                &self.options,
                parent_url,
                url,
                is_recording.then_some(&mut exchanges),
            );
            for exchange in exchanges {
                self.record_exchange(exchange);
            }
            let Some(((data, response_url, media_type, charset), policy)) = result? else {
                return Ok(None);
            };
            if let Some(policy) = &policy {
                self.received_policies.insert(
                    clean_url(without_credentials(&response_url)).to_string(),
                    policy.clone(),
                );
            }

            // Add retrieved resource to cache
            if let Some(cache) = self.cache.as_mut() {
                let new_cache_key: String =
                    self.credentials.cache_key(&response_url) + &cors_suffix;
                let final_url: String = clean_url(without_credentials(&response_url)).to_string();
                // What was asked for is worth keeping along with where it ended up
                if new_cache_key != identity_key {
                    cache.set_with_final_url(
                        &identity_key,
                        &data,
                        media_type.clone(),
                        charset.clone(),
                        Some(final_url.clone()),
                    );
                    cache.set_policy(&identity_key, policy.clone());
                }
                cache.set_policy(&new_cache_key, policy);
                cache.set_with_final_url(
                    &new_cache_key,
                    &data,
                    media_type.clone(),
                    charset.clone(),
                    Some(final_url),
                );
            }

            Ok(Some((data, response_url, media_type, charset)))
        }
    }
}
