use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::mpsc;

use cssparser::{ParseError, Parser, Token};
use markup5ever_rcdom::{Handle, NodeData};
use rayon::{Scope, ThreadPoolBuilder};
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue, REFERER};

use crate::cookies::Cookie;
use crate::core::{MonolithOptions, parse_content_type, print_error_message, print_info_message};
use crate::css::is_image_url_prop;
use crate::html::{LinkType, get_node_attr, html_to_dom, parse_link_type, parse_srcset};
use crate::url::{Url, clean_url, domain_is_within_domain, get_referer_url, resolve_url};

/// What Session should put into its cache.
pub struct CacheEntry {
    pub key: String,
    pub data: Vec<u8>,
    pub media_type: String,
    pub charset: String,
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
}

/// Shared, read-only state handed to every worker.
struct Ctx<'a> {
    client: &'a Client,
    cookies: Option<&'a [Cookie]>,
    options: &'a MonolithOptions,
    seen: Mutex<HashSet<String>>,
    results_tx: mpsc::Sender<Fetched>,
}

fn cache_key(url: &Url) -> String {
    clean_url(url.clone()).as_str().to_string()
}

pub fn is_domain_allowed(url: &Url, options: &MonolithOptions) -> bool {
    match (&options.domains, url.host_str()) {
        (Some(domains), Some(host)) => {
            let matches = domains
                .iter()
                .any(|d| domain_is_within_domain(host, d.trim()));
            matches != options.blacklist_domains
        }
        _ => true,
    }
}

/// Performs the HTTP(S) request for an asset.
/// It doesn't touch the cache or the Session, so prefetch workers can call it concurrently.
pub fn fetch_remote_asset(
    client: &Client,
    cookies: Option<&[Cookie]>,
    options: &MonolithOptions,
    parent_url: &Url,
    url: &Url,
) -> Result<(Vec<u8>, Url, String, String), reqwest::Error> {
    let cache_key: String = clean_url(url.clone()).as_str().to_string();

    if !is_domain_allowed(url, options) {
        return Err(client.get("").send().unwrap_err());
    }

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
    // Add referer header for page resource requests
    if ["https", "http"].contains(&parent_url.scheme()) && parent_url != url {
        headers.insert(
            REFERER,
            HeaderValue::from_str(get_referer_url(parent_url.clone()).as_str()).unwrap(),
        );
    }

    match client.get(url.as_str()).headers(headers).send() {
        Ok(response) => {
            if !options.ignore_errors && response.status() != reqwest::StatusCode::OK {
                if !options.silent {
                    print_error_message(&format!("{} ({})", &cache_key, response.status()));
                }

                // Provoke error
                return Err(client.get("").send().unwrap_err());
            }

            let response_url: Url = response.url().clone();

            if !options.silent {
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
                    if !options.silent {
                        print_error_message(&format!("{}", error));
                    }
                }
            }

            Ok((data, response_url, media_type, charset))
        }
        Err(error) => {
            if !options.silent {
                print_error_message(&format!("{} ({})", &cache_key, error));
            }

            Err(client.get("").send().unwrap_err())
        }
    }
}

pub fn prefetch_assets(
    client: &Client,
    cookies: Option<&[Cookie]>,
    options: &MonolithOptions,
    document_url: &Url,
    document: &Handle,
    is_cached: impl Fn(&str) -> bool,
) -> Vec<CacheEntry> {
    let mut jobs: Vec<Job> = vec![];
    collect_html(document, "", document_url, options, &mut jobs);
    jobs.retain(|job| !is_cached(&cache_key(&job.url)));
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
    seen.insert(cache_key(document_url));
    let ctx = Ctx {
        client,
        cookies,
        options,
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
        let final_key = cache_key(&fetched.final_url);

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
            });
        }

        entries.push(CacheEntry {
            key: final_key,
            data: fetched.data,
            media_type: fetched.media_type,
            charset: fetched.charset,
        });
    }

    entries
}

fn spawn_job<'s>(s: &Scope<'s>, ctx: &'s Ctx<'s>, job: Job) {
    let request_key = cache_key(&job.url);
    if !ctx.seen.lock().unwrap().insert(request_key.clone()) {
        return; // Already fetched or in flight
    }

    s.spawn(move |s| {
        let (data, final_url, media_type, charset) = match fetch_remote_asset(
            ctx.client,
            ctx.cookies,
            ctx.options,
            &job.parent_url,
            &job.url,
        ) {
            Ok(result) => result,
            Err(_) => return, // The walk will retry and report the error as usual
        };

        // Discover nested assets while we're still on a worker thread
        let mut more_jobs: Vec<Job> = vec![];
        match job.kind {
            AssetKind::Css => {
                // TODO: check integrity here to avoid dealing with bad files

                let stylesheet =
                    if let Some(encoding) = encoding_rs::Encoding::for_label(charset.as_bytes()) {
                        let (decoded, _, _) = encoding.decode(&data);
                        decoded.into_owned()
                    } else {
                        String::from_utf8_lossy(&data).into_owned()
                    };
                collect_css_str(&stylesheet, &final_url, ctx.options, &mut more_jobs);
            }
            AssetKind::Html => {
                // RcDom is !Send, but it's created and dropped right here on this thread
                let frame_dom = html_to_dom(&data, charset.clone());
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
        });

        for more_job in more_jobs {
            spawn_job(s, ctx, more_job);
        }
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
                        } else if types.contains(&LinkType::Stylesheet) && !options.no_css {
                            push_job(jobs, base_url, &href, AssetKind::Css);
                        } else if types.contains(&LinkType::Manifest) {
                            push_job(jobs, base_url, &href, AssetKind::Other);
                        }
                    }
                }
                "body" => {
                    if !options.no_images {
                        if let Some(background) = get_node_attr(node, "background") {
                            push_job(jobs, base_url, &background, AssetKind::Other);
                        }
                    }
                }
                "img" => {
                    if !options.no_images {
                        let data_src = get_node_attr(node, "data-src").unwrap_or_default();
                        let src = if !data_src.is_empty() {
                            data_src
                        } else {
                            get_node_attr(node, "src").unwrap_or_default()
                        };
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
                    {
                        if let Some(src) = get_node_attr(node, "src") {
                            push_job(jobs, base_url, &src, AssetKind::Other);
                        }
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
                        if !((parent_name == "audio" && options.no_audio)
                            || (parent_name == "video" && options.no_video))
                        {
                            push_job(jobs, base_url, &src, AssetKind::Other);
                        }
                    }
                    if parent_name == "picture" && !options.no_images {
                        if let Some(srcset) = get_node_attr(node, "srcset") {
                            push_srcset(jobs, base_url, &srcset);
                        }
                    }
                }
                "script" => {
                    if !options.no_js {
                        if let Some(src) = get_node_attr(node, "src") {
                            push_job(jobs, base_url, &src, AssetKind::Other);
                        }
                    }
                }
                "style" => {
                    if !options.no_css {
                        for css in node_text(node) {
                            collect_css_str(&css, base_url, options, jobs);
                        }
                    }
                }
                "frame" | "iframe" => {
                    if !options.no_frames {
                        if let Some(src) = get_node_attr(node, "src") {
                            push_job(jobs, base_url, &src, AssetKind::Html);
                        }
                    }
                }
                "audio" => {
                    if !options.no_audio {
                        if let Some(src) = get_node_attr(node, "src") {
                            push_job(jobs, base_url, &src, AssetKind::Other);
                        }
                    }
                }
                "video" => {
                    if !options.no_video {
                        if let Some(src) = get_node_attr(node, "src") {
                            push_job(jobs, base_url, &src, AssetKind::Other);
                        }
                    }
                    if !options.no_images {
                        if let Some(poster) = get_node_attr(node, "poster") {
                            push_job(jobs, base_url, &poster, AssetKind::Other);
                        }
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

            if !options.no_css {
                if let Some(style) = get_node_attr(node, "style") {
                    collect_css_str(&style, base_url, options, jobs);
                }
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

    loop {
        let token = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => break,
        };

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
