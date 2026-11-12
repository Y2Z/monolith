use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

use chrono::Utc;
use cssparser::{ParseError, Parser, Token};
use markup5ever_rcdom::{Handle, NodeData};
use rayon::{Scope, ThreadPoolBuilder};
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue, LOCATION, REFERER};
use reqwest::{StatusCode, Version};

use crate::cache::Exchange;
use crate::cookies::Cookie;
use crate::core::{MonolithOptions, parse_media_type, print_error_message, print_info_message};
use crate::css::{decode_css, is_image_url_prop};
use crate::html::{
    LinkType, get_node_attr, html_to_dom, parse_link_type, parse_srcset, strip_elements,
};
use crate::mhtml::percent_decode;
use crate::url::{Url, clean_url, domain_is_within_domain, get_referer_url, resolve_url};

/// What Session should put into its cache.
pub struct CacheEntry {
    pub key: String,
    pub data: Vec<u8>,
    pub media_type: String,
    pub charset: String,
    /// What it took to get it, for archives that keep that
    pub exchanges: Vec<Exchange>,
}

/// How far along prefetching is; meant to be shared with whoever displays the progress.
/// The total may keep growing, since stylesheets and frames can reveal more assets.
#[derive(Default)]
pub struct PrefetchProgress {
    done: AtomicUsize,
    total: AtomicUsize,
}

impl PrefetchProgress {
    /// Number of assets processed so far, and how many have been found overall.
    pub fn counts(&self) -> (usize, usize) {
        (
            self.done.load(Ordering::SeqCst),
            self.total.load(Ordering::SeqCst),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AssetKind {
    Css,   // Will be parsed for further url()s and @imports
    Html,  // (i)frame document; will be parsed for further assets
    Other, // Images, scripts, fonts, media...
}

struct Job {
    url: Url,
    parent_url: Url,
    kind: AssetKind,
}

struct Fetched {
    request_key: String,
    data: Vec<u8>,
    final_url: Url,
    media_type: String,
    charset: String,
    kind: AssetKind,
    exchanges: Vec<Exchange>,
}

/// Shared, read-only state handed to every worker.
struct Ctx<'a> {
    client: &'a Client,
    cookies: Option<&'a [Cookie]>,
    credentials: &'a Credentials,
    options: &'a MonolithOptions,
    progress: Option<&'a PrefetchProgress>,
    seen: Mutex<HashSet<String>>,
    results_tx: mpsc::Sender<Fetched>,
}

impl Ctx<'_> {
    fn count_job_found(&self) {
        if let Some(progress) = self.progress {
            progress.total.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn count_job_done(&self) {
        if let Some(progress) = self.progress {
            progress.done.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// Credentials for HTTP authentication, by origin: once given for a site, they go along with
/// everything that gets retrieved from it, the way browsers do it. URLs never keep them,
/// they'd end up in saved documents otherwise.
#[derive(Clone, Default)]
pub struct Credentials {
    by_origin: HashMap<String, (String, Option<String>)>,
}

impl Credentials {
    fn origin_of(url: &Url) -> Option<String> {
        matches!(url.scheme(), "http" | "https").then(|| url.origin().ascii_serialization())
    }

    /// Takes credentials out of given URL, keeping them for its origin.
    pub fn take_from(&mut self, url: &Url) -> Url {
        if url.username().is_empty() && url.password().is_none() {
            return url.clone();
        }
        if let Some(origin) = Credentials::origin_of(url) {
            self.by_origin.insert(
                origin,
                (
                    percent_decode(url.username()),
                    url.password().map(percent_decode),
                ),
            );
        }

        without_credentials(url)
    }

    /// User name and password to authenticate with, for given URL.
    pub fn of(&self, url: &Url) -> Option<&(String, Option<String>)> {
        self.by_origin.get(&Credentials::origin_of(url)?)
    }

    /// What given URL gets cached under: what's retrieved as someone isn't what's retrieved
    /// as someone else (or as nobody), user names keep them apart; passwords stay out of it.
    pub fn cache_key(&self, url: &Url) -> String {
        let mut url = clean_url(without_credentials(url));
        match self.of(&url) {
            // Password alone is still someone, URLs can't show that
            Some((user, _)) if user.is_empty() => format!("(password) {}", url),
            Some((user, _)) => {
                let _ = url.set_username(user);
                url.to_string()
            }
            None => url.to_string(),
        }
    }
}

/// URL without user name and password.
pub fn without_credentials(url: &Url) -> Url {
    let mut url = url.clone();
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url
}

pub fn is_domain_allowed(url: &Url, options: &MonolithOptions) -> bool {
    match (&options.domains, url.host_str()) {
        (Some(domains), Some(host)) => {
            let matches = domains
                .iter()
                .any(|d| domain_is_within_domain(host, d.trim()));
            matches == options.whitelist_domains
        }
        _ => true,
    }
}

const MAX_REDIRECTS: usize = 10;

#[doc(hidden)]
pub fn request_headers(cookies: Option<&[Cookie]>, parent_url: &Url, url: &Url) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(cookies) = cookies {
        let cookie_values: Vec<String> = cookies
            .iter()
            .filter(|cookie| !cookie.is_expired() && cookie.matches_url(url.as_str()))
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect();
        if !cookie_values.is_empty() {
            headers.insert(
                COOKIE,
                HeaderValue::from_str(&cookie_values.join("; ")).unwrap(),
            );
        }
    }
    // Add referer header for page resource requests, but never send where secure pages are over
    // connections that aren't
    let is_downgrade = parent_url.scheme() == "https" && url.scheme() != "https";
    if ["https", "http"].contains(&parent_url.scheme()) && parent_url != url && !is_downgrade {
        headers.insert(
            REFERER,
            HeaderValue::from_str(get_referer_url(parent_url.clone()).as_str()).unwrap(),
        );
    }

    headers
}

fn header_pairs(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect()
}

/// What was sent and received, the way archives keep it.
fn create_exchange(
    options: &MonolithOptions,
    url: &Url,
    sent_headers: &HeaderMap,
    response: &Response,
) -> Exchange {
    let mut request_headers = vec![(
        "Host".to_string(),
        url[::url::Position::BeforeHost..::url::Position::AfterPort].to_string(),
    )];
    if let Some(user_agent) = &options.user_agent {
        request_headers.push(("User-Agent".to_string(), user_agent.clone()));
    }
    request_headers.push(("Accept".to_string(), "*/*".to_string()));
    request_headers.extend(header_pairs(sent_headers));

    Exchange {
        date: Utc::now(),
        http_version: match response.version() {
            Version::HTTP_09 => "HTTP/0.9",
            Version::HTTP_10 => "HTTP/1.0",
            Version::HTTP_2 => "HTTP/2",
            Version::HTTP_3 => "HTTP/3",
            _ => "HTTP/1.1",
        }
        .to_string(),
        method: "GET".to_string(),
        request_headers,
        response_headers: header_pairs(response.headers()),
        status: Some(response.status().as_u16()),
        status_text: response
            .status()
            .canonical_reason()
            .unwrap_or_default()
            .to_string(),
        url: url.to_string(),
        ..Default::default()
    }
}

/// Performs the HTTP(S) request for an asset.
/// It doesn't touch the cache or the Session, so prefetch workers can call it concurrently.
///
/// Whatever gets exchanged ends up in exchanges, if given; the client has to be the one that
/// doesn't follow redirects then, they get followed here, so that each one gets recorded.
pub fn fetch_remote_asset(
    client: &Client,
    cookies: Option<&[Cookie]>,
    credentials: &Credentials,
    options: &MonolithOptions,
    parent_url: &Url,
    url: &Url,
    mut exchanges: Option<&mut Vec<Exchange>>,
) -> Result<(Vec<u8>, Url, String, String), reqwest::Error> {
    // Credentials that come with the URL itself are for that URL's origin
    let mut credentials = credentials.clone();
    let url: &Url = &credentials.take_from(url);
    let parent_url: &Url = &without_credentials(parent_url);
    let cache_key: String = clean_url(url.clone()).as_str().to_string();

    if !is_domain_allowed(url, options) {
        if !options.silent {
            print_error_message(&format!("{} (domain not allowed)", cache_key));
        }

        return Err(client.get("").send().unwrap_err());
    }

    let mut current_url: Url = url.clone();
    let mut redirects: usize = 0;
    let (response, sent_headers) = loop {
        let sent_headers = request_headers(cookies, parent_url, &current_url);
        let mut request = client
            .get(current_url.as_str())
            .headers(sent_headers.clone());
        if let Some((user, password)) = credentials.of(&current_url) {
            request = request.basic_auth(user, password.as_ref());
        }
        let response = match request.send() {
            Ok(response) => response,
            Err(error) => {
                if !options.silent {
                    print_error_message(&format!("{} ({})", cache_key, error));
                }

                return Err(client.get("").send().unwrap_err());
            }
        };

        let next_url: Option<Url> = response
            .status()
            .is_redirection()
            .then(|| response.headers().get(LOCATION)?.to_str().ok())
            .flatten()
            .and_then(|location| current_url.join(location.trim()).ok());
        let Some(exchanges) = exchanges.as_deref_mut() else {
            break (response, sent_headers);
        };
        let Some(next_url) = next_url.filter(|_| response.status() != StatusCode::NOT_MODIFIED)
        else {
            break (response, sent_headers);
        };

        let mut exchange = create_exchange(options, &current_url, &sent_headers, &response);
        exchange.body = response.bytes().map(|b| b.to_vec()).unwrap_or_default();
        exchanges.push(exchange);

        redirects += 1;
        let reason = if !is_domain_allowed(&next_url, options) {
            Some("redirect target is blocked")
        } else if redirects > MAX_REDIRECTS {
            Some("too many redirects")
        } else {
            None
        };
        if let Some(reason) = reason {
            if !options.silent {
                print_error_message(&format!("{} ({})", cache_key, reason));
            }

            return Err(client.get("").send().unwrap_err());
        }
        current_url = next_url;
    };

    let exchange: Option<Exchange> = exchanges
        .is_some()
        .then(|| create_exchange(options, &current_url, &sent_headers, &response));

    if !options.ignore_errors && response.status() != reqwest::StatusCode::OK {
        if !options.silent {
            print_error_message(&format!("{} ({})", cache_key, response.status()));
        }

        // Archives keep what went wrong too
        if let (Some(exchanges), Some(mut exchange)) = (exchanges, exchange) {
            exchange.body = response.bytes().map(|b| b.to_vec()).unwrap_or_default();
            exchanges.push(exchange);
        }

        // Provoke error
        return Err(client.get("").send().unwrap_err());
    }

    let response_url: Url = response.url().clone();

    if !options.silent {
        if url.as_str() == response_url.as_str() {
            print_info_message(&cache_key.to_string());
        } else {
            print_info_message(&format!("{} -> {}", cache_key, response_url));
        }
    }

    // Attempt to obtain media type and charset by reading Content-Type header
    let content_type: &str = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .unwrap_or("");

    // Leave charset empty if not given, so that the document's META can be used
    let (media_type, charset, _is_base64) = parse_media_type(content_type);
    let charset: String = charset.unwrap_or_default();

    // Convert response into a byte array, what broke off partway is no use
    let data: Vec<u8> = match response.bytes() {
        Ok(b) => b.to_vec(),
        Err(error) => {
            if !options.silent {
                print_error_message(&format!("{} ({})", cache_key, error));
            }

            return Err(error);
        }
    };

    if let (Some(exchanges), Some(mut exchange)) = (exchanges, exchange) {
        exchange.body = data.clone();
        exchanges.push(exchange);
    }

    Ok((data, response_url, media_type, charset))
}

/// What requests get made with.
pub struct Requester<'a> {
    pub client: &'a Client,
    pub cookies: Option<&'a [Cookie]>,
    pub credentials: &'a Credentials,
}

pub fn prefetch_assets(
    requester: Requester,
    options: &MonolithOptions,
    document_url: &Url,
    document: &Handle,
    progress: Option<&PrefetchProgress>,
    is_cached: impl Fn(&str) -> bool,
) -> Vec<CacheEntry> {
    let mut jobs: Vec<Job> = vec![];
    collect_html(document, "", document_url, options, &mut jobs);
    let Requester {
        client,
        cookies,
        credentials,
    } = requester;
    jobs.retain(|job| !is_cached(&credentials.cache_key(&job.url)));
    if jobs.is_empty() {
        return vec![];
    }

    let pool = match ThreadPoolBuilder::new()
        .num_threads(options.threads)
        .build()
    {
        Ok(pool) => pool,
        Err(_) => return vec![], // Not fatal: the walk will just fetch everything itself
    };

    let (tx, rx) = mpsc::channel();
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(credentials.cache_key(document_url));
    let ctx = Ctx {
        client,
        cookies,
        credentials,
        options,
        progress,
        seen: Mutex::new(seen),
        results_tx: tx,
    };

    pool.scope(|s| {
        for job in jobs {
            spawn_job(s, &ctx, job);
        }
    });

    // Drop the sender so the receiver knows when all workers are done
    drop(ctx.results_tx);

    let mut entries: Vec<CacheEntry> = vec![];
    for fetched in rx {
        let final_key = credentials.cache_key(&fetched.final_url);

        // Session::retrieve_asset() looks assets up by their *request* URL and, on a
        // cache hit, reports that same URL as the final one. That's harmless for leaf
        // assets, but stylesheets and frames use their final URL as the base for
        // resolving relative URLs, so redirected CSS/HTML is left for the walk to
        // re-request.
        if fetched.request_key != final_key && fetched.kind == AssetKind::Other {
            entries.push(CacheEntry {
                key: fetched.request_key,
                data: fetched.data.clone(),
                media_type: fetched.media_type.clone(),
                charset: fetched.charset.clone(),
                exchanges: vec![],
            });
        }

        entries.push(CacheEntry {
            key: final_key,
            data: fetched.data,
            media_type: fetched.media_type,
            charset: fetched.charset,
            exchanges: fetched.exchanges,
        });
    }

    entries
}

fn spawn_job<'s>(s: &Scope<'s>, ctx: &'s Ctx<'s>, job: Job) {
    let request_key = ctx.credentials.cache_key(&job.url);
    if !ctx.seen.lock().unwrap().insert(request_key.clone()) {
        return; // Already fetched or in flight
    }
    ctx.count_job_found();

    s.spawn(move |s| {
        let mut exchanges: Vec<Exchange> = vec![];
        let is_recording = ctx.options.output_format.is_warc();
        let (data, final_url, media_type, charset) = match fetch_remote_asset(
            ctx.client,
            ctx.cookies,
            ctx.credentials,
            ctx.options,
            &job.parent_url,
            &job.url,
            is_recording.then_some(&mut exchanges),
        ) {
            Ok(result) => result,
            Err(_) => {
                ctx.count_job_done();
                return; // The walk will retry and report the error as usual
            }
        };

        // Discover nested assets while we're still on a worker thread
        let mut more_jobs: Vec<Job> = vec![];
        match job.kind {
            AssetKind::Css => {
                // TODO: check integrity here to avoid dealing with bad files

                // Decode exactly the way the walk will, so both discover the same URLs
                let stylesheet = decode_css(&data, &charset);
                collect_css_str(&stylesheet, &final_url, ctx.options, &mut more_jobs);
            }
            AssetKind::Html => {
                // RcDom is !Send, but it's created and dropped right here on this thread
                let frame_dom = html_to_dom(&data, charset.clone());
                strip_elements(
                    &frame_dom.document,
                    &ctx.options.remove_elements,
                    &ctx.options.wipe_elements,
                );
                collect_html(
                    &frame_dom.document,
                    "",
                    &final_url,
                    ctx.options,
                    &mut more_jobs,
                );
            }
            AssetKind::Other => {}
        }

        let _ = ctx.results_tx.send(Fetched {
            request_key,
            data,
            final_url,
            media_type,
            charset,
            kind: job.kind,
            exchanges,
        });

        // Nested jobs get counted first, so that the total can't be reached too early
        for more_job in more_jobs {
            spawn_job(s, ctx, more_job);
        }
        ctx.count_job_done();
    });
}

fn push_job(jobs: &mut Vec<Job>, base_url: &Url, value: &str, kind: AssetKind) {
    let value = value.trim();
    if value.is_empty() || value.starts_with('#') {
        return;
    }

    let url: Url = resolve_url(base_url, value);

    // data: needs no fetching, file: is cheap and has its own security rules
    if url.scheme() == "http" || url.scheme() == "https" {
        jobs.push(Job {
            url,
            parent_url: base_url.clone(),
            kind,
        });
    }
}

fn push_srcset(jobs: &mut Vec<Job>, base_url: &Url, srcset: &str) {
    for item in parse_srcset(srcset) {
        push_job(jobs, base_url, item.path, AssetKind::Other);
    }
}

fn node_text(node: &Handle) -> Vec<String> {
    node.children
        .borrow()
        .iter()
        .filter_map(|child| match child.data {
            NodeData::Text { ref contents } => Some(contents.borrow().to_string()),
            _ => None,
        })
        .collect()
}

/// Read-only mirror of html::walk(): must request the same things.
/// If the walk learns to embed something new, add it here too (or it just won't be
/// prefetched; nothing breaks).
fn collect_html(
    node: &Handle,
    parent_name: &str,
    base_url: &Url,
    options: &MonolithOptions,
    jobs: &mut Vec<Job>,
) {
    match node.data {
        NodeData::Document => {
            for child in node.children.borrow().iter() {
                collect_html(child, "", base_url, options, jobs);
            }
        }
        NodeData::Element { ref name, .. } => {
            let name: &str = name.local.as_ref();

            match name {
                "link" => {
                    let types: Vec<LinkType> =
                        parse_link_type(&get_node_attr(node, "rel").unwrap_or_default());
                    if let Some(href) = get_node_attr(node, "href") {
                        if types.contains(&LinkType::Favicon)
                            || types.contains(&LinkType::AppleTouchIcon)
                        {
                            if !options.no_images {
                                push_job(jobs, base_url, &href, AssetKind::Other);
                            }
                        } else if types.contains(&LinkType::Stylesheet) && !options.no_styles {
                            push_job(jobs, base_url, &href, AssetKind::Css);
                        } else if types.contains(&LinkType::Manifest) {
                            push_job(jobs, base_url, &href, AssetKind::Other);
                        }
                    }
                }
                "body" => {
                    if !options.no_images
                        && let Some(background) = get_node_attr(node, "background")
                    {
                        push_job(jobs, base_url, &background, AssetKind::Other);
                    }
                }
                "img" => {
                    if !options.no_images {
                        let src = get_node_attr(node, "src").unwrap_or_default();
                        push_job(jobs, base_url, &src, AssetKind::Other);

                        if let Some(srcset) = get_node_attr(node, "srcset") {
                            push_srcset(jobs, base_url, &srcset);
                        }
                    }
                }
                "input" => {
                    if !options.no_images
                        && get_node_attr(node, "type")
                            .unwrap_or_default()
                            .eq_ignore_ascii_case("image")
                        && let Some(src) = get_node_attr(node, "src")
                    {
                        push_job(jobs, base_url, &src, AssetKind::Other);
                    }
                }
                "image" | "use" => {
                    if !options.no_images {
                        for attr_name in ["href", "xlink:href"] {
                            if let Some(href) = get_node_attr(node, attr_name) {
                                push_job(jobs, base_url, &href, AssetKind::Other);
                            }
                        }
                    }
                }
                "source" => {
                    // Note: html::get_parent_node() take()s the parent pointer,
                    // so we track the parent's name ourselves instead of calling it
                    if let Some(src) = get_node_attr(node, "src") {
                        push_job(jobs, base_url, &src, AssetKind::Other);
                    }
                    if parent_name == "picture"
                        && !options.no_images
                        && let Some(srcset) = get_node_attr(node, "srcset")
                    {
                        push_srcset(jobs, base_url, &srcset);
                    }
                }
                "script" => {
                    if !options.no_js
                        && let Some(src) = get_node_attr(node, "src")
                    {
                        push_job(jobs, base_url, &src, AssetKind::Other);
                    }
                }
                "style" => {
                    if !options.no_styles {
                        for css in node_text(node) {
                            collect_css_str(&css, base_url, options, jobs);
                        }
                    }
                }
                "frame" | "iframe" => {
                    if let Some(src) = get_node_attr(node, "src") {
                        push_job(jobs, base_url, &src, AssetKind::Html);
                    }
                }
                "audio" => {
                    if let Some(src) = get_node_attr(node, "src") {
                        push_job(jobs, base_url, &src, AssetKind::Other);
                    }
                }
                "video" => {
                    if let Some(src) = get_node_attr(node, "src") {
                        push_job(jobs, base_url, &src, AssetKind::Other);
                    }
                    if !options.no_images
                        && let Some(poster) = get_node_attr(node, "poster")
                    {
                        push_job(jobs, base_url, &poster, AssetKind::Other);
                    }
                }
                "noscript" => {
                    for contents in node_text(node) {
                        let noscript_dom = html_to_dom(&contents.into_bytes(), "".to_string());
                        collect_html(&noscript_dom.document, "", base_url, options, jobs);
                    }
                }
                _ => {}
            }

            if !options.no_styles
                && let Some(style) = get_node_attr(node, "style")
            {
                collect_css_str(&style, base_url, options, jobs);
            }

            for child in node.children.borrow().iter() {
                collect_html(child, name, base_url, options, jobs);
            }
        }
        _ => {}
    }
}

fn collect_css_str(css: &str, base_url: &Url, options: &MonolithOptions, jobs: &mut Vec<Job>) {
    let mut parser = Parser::new(css);
    collect_css(&mut parser, base_url, options, "", "", "", jobs);
}

/// Read-only mirror of css::process_css(): tracks the same rule/property/function
/// state to decide which URLs would be requested.
fn collect_css(
    parser: &mut Parser,
    base_url: &Url,
    options: &MonolithOptions,
    rule_name: &str,
    prop_name: &str,
    func_name: &str,
    jobs: &mut Vec<Job>,
) {
    let mut curr_rule: String = rule_name.to_string();
    let mut curr_prop: String = prop_name.to_string();

    while let Ok(token) = parser.next_including_whitespace_and_comments() {
        let token = token.clone();

        match token {
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                if options.no_fonts && curr_rule == "font-face" {
                    continue; // Unconsumed block gets skipped by the parser
                }
                let prop = curr_prop.clone();
                let _ = parser.parse_nested_block(|parser| -> Result<(), ParseError<()>> {
                    collect_css(parser, base_url, options, rule_name, &prop, func_name, jobs);
                    Ok(())
                });
            }
            Token::Function(ref name) => {
                let name = name.to_string();
                let (rule, prop) = (curr_rule.clone(), curr_prop.clone());
                let _ = parser.parse_nested_block(|parser| -> Result<(), ParseError<()>> {
                    collect_css(parser, base_url, options, &rule, &prop, &name, jobs);
                    Ok(())
                });
            }
            Token::Ident(ref value) => {
                curr_rule = "".to_string();
                curr_prop = value.to_string();
            }
            Token::IDHash(_) => {
                curr_rule = "".to_string();
            }
            Token::AtKeyword(ref value) => {
                curr_rule = value.to_string();
            }
            Token::QuotedString(ref value) => {
                if curr_rule == "import" {
                    curr_rule = "".to_string();
                    push_job(jobs, base_url, value, AssetKind::Css);
                } else if func_name == "url"
                    && !(options.no_images && is_image_url_prop(&curr_prop))
                {
                    push_job(jobs, base_url, value, AssetKind::Other);
                }
            }
            Token::UnquotedUrl(ref value) => {
                if curr_rule == "import" {
                    curr_rule = "".to_string();
                    push_job(jobs, base_url, value, AssetKind::Css);
                } else if !(options.no_images && is_image_url_prop(&curr_prop)) {
                    push_job(jobs, base_url, value, AssetKind::Other);
                }
            }
            _ => {}
        }
    }
}
