use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::{MAIN_SEPARATOR, Path};
use std::str::FromStr;
use std::sync::Arc;

use chrono::{SecondsFormat, Utc};
use encoding_rs::{Encoding, UTF_8};
use markup5ever_rcdom::RcDom;
use url::Url;

#[cfg(feature = "browser")]
use crate::browser::{ColorScheme, render_page};
use crate::cors::PageOrigin;
use crate::csp::{Destination, Policies, Request};
use crate::har::{is_har, parse_har};
use crate::html::{
    add_favicon, base_url_allowed, document_policies, enter_document, find_nodes, get_base_url,
    get_charset, get_robots, get_title, has_favicon, hash_named_elements, html_to_dom,
    inline_stylesheets, saved_policies, serialize_document, serialize_document_into, set_base_url,
    set_charset, set_content_security_policies, set_metadata, set_node_attr, set_robots,
    strip_elements, targets_element_anywhere, walk,
};
use crate::mhtml::{
    ArchivedAsset, MhtmlArchive, is_mhtml, parse_mhtml, percent_decode, write_mhtml_document,
};
use crate::replay::{
    CapturedAsset, CapturedPage, create_replay_markup, exchanges_of, is_html, is_script,
    rewrite_scripts, rewrite_stylesheets,
};
use crate::session::Session;
use crate::simple::simplify_document;
use crate::url::{clean_url, create_data_url, is_windows_drive_path, resolve_url};
use crate::warc::{decompress_warc, is_warc, parse_warc};
use crate::webarchive::{is_webarchive, parse_webarchive};

#[derive(Debug)]
pub struct MonolithError {
    details: String,
}

impl MonolithError {
    fn new(msg: &str) -> MonolithError {
        MonolithError {
            details: msg.to_string(),
        }
    }
}

impl From<io::Error> for MonolithError {
    fn from(error: io::Error) -> MonolithError {
        MonolithError::new(&error.to_string())
    }
}

impl fmt::Display for MonolithError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MonolithError {
    fn description(&self) -> &str {
        &self.details
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum MonolithOutputFormat {
    #[default]
    HTML,
    MHTML,
    WARC,
    /// WARC, compressed record by record
    WARCGZ,
    // ZIM,
    // HAR,
}

impl MonolithOutputFormat {
    /// Whether it's an archive of what has been exchanged with servers, rather than a document.
    pub fn is_warc(&self) -> bool {
        matches!(
            self,
            MonolithOutputFormat::WARC | MonolithOutputFormat::WARCGZ
        )
    }
}

impl FromStr for MonolithOutputFormat {
    type Err = String;

    fn from_str(format: &str) -> Result<Self, Self::Err> {
        match format.to_lowercase().as_str() {
            "html" | "htm" => Ok(MonolithOutputFormat::HTML),
            "mhtml" | "mht" => Ok(MonolithOutputFormat::MHTML),
            "warc" => Ok(MonolithOutputFormat::WARC),
            "warcgz" => Ok(MonolithOutputFormat::WARCGZ),
            _ => Err(format!(
                "unknown format \"{}\", use HTML, MHTML, WARC, or WARCGZ",
                format
            )),
        }
    }
}

#[derive(Clone, Default)]
pub struct MonolithOptions {
    pub base_url: Option<String>,
    #[cfg(feature = "browser")]
    pub browser: bool,
    /// Overrides MONOLITH_BROWSER_COLOR_SCHEME, the system decides if neither is set
    #[cfg(feature = "browser")]
    pub browser_color_scheme: Option<ColorScheme>,
    /// Overrides MONOLITH_BROWSER_DISABLE_JS: whether pages get to run JavaScript in the browser
    /// (unlike remove_js, which is about what ends up in the document)
    #[cfg(feature = "browser")]
    pub browser_disable_js: Option<bool>,
    /// Overrides MONOLITH_BROWSER_SCROLL_LIMIT: how many screens to go through, zero for none
    #[cfg(feature = "browser")]
    pub browser_scroll_limit: Option<u32>,
    #[cfg(feature = "browser")]
    pub browser_user_agent: Option<String>,
    /// Overrides MONOLITH_BROWSER_VIEWPORT: width and height of what the page gets to see
    #[cfg(feature = "browser")]
    pub browser_viewport: Option<(u32, u32)>,
    pub domains: Option<Vec<String>>,
    pub whitelist_domains: bool,
    pub encoding: Option<String>,
    pub ignore_errors: bool,
    pub insecure: bool,
    pub isolate: bool,
    pub remove_styles: bool,
    pub remove_fonts: bool,
    pub remove_images: bool,
    pub remove_js: bool,
    pub output_format: MonolithOutputFormat,
    pub remove_elements: Vec<String>,
    /// Set for documents that replay responses, instead of relying on a network
    pub replay: bool,
    pub silent: bool,
    pub simplify: bool,
    pub threads: usize,
    pub timeout: u64,
    pub unwrap_noscript: bool,
    pub user_agent: Option<String>,
    pub wipe_elements: Vec<String>,
}

const ANSI_COLOR_RED: &str = "\x1b[31m";
const ANSI_COLOR_RESET: &str = "\x1b[0m";
// All known non-"text/..." plaintext media types
const PLAINTEXT_MEDIA_TYPES: &[&str] = &[
    "application/javascript",          // .js
    "application/json",                // .json
    "application/ld+json",             // .jsonld
    "application/x-sh",                // .sh
    "application/xhtml+xml",           // .xhtml
    "application/xml",                 // .xml
    "application/vnd.mozilla.xul+xml", // .xul
    "image/svg+xml",                   // .svg
];

type MonolithResult = Result<(Vec<u8>, Option<String>), MonolithError>;

/// Done with what has been recorded, or written in other ways.
fn finish(mut session: Session, title: Option<String>) -> MonolithResult {
    Ok((session.finish_output(title.as_deref())?, title))
}

/// Provides what has been obtained the way it is, without processing it (the way browsers do).
fn finish_as_is(mut session: Session, data: &[u8]) -> MonolithResult {
    session.open_output(None)?;
    session.write_output(data);
    finish(session, None)
}

fn record_captured_page(session: &mut Session, page: &CapturedPage) {
    for asset in &page.assets {
        for exchange in exchanges_of(asset) {
            session.record_exchange(exchange);
        }
    }
}

/// Things that are worth looking into, no matter what they claim to be.
fn is_archive(data: &[u8]) -> bool {
    is_mhtml(data) || is_warc(data) || is_har(data) || is_webarchive(data)
}

/// Pages that won't be replayed still have to get everything from where they came from.
fn create_archive_from_page(page: &CapturedPage) -> MhtmlArchive {
    let assets = page
        .assets
        .iter()
        .filter(|asset| asset.is_reusable)
        .map(|asset| {
            let urls: Vec<Url> = asset
                .urls
                .iter()
                .filter_map(|url| Url::parse(url).ok())
                .collect();
            let (media_type, charset, _) = parse_media_type(&asset.content_type);

            (
                asset.destination != "document",
                (
                    urls.clone(),
                    ArchivedAsset {
                        data: asset.data.clone(),
                        url: urls.last().cloned(),
                        media_type,
                        charset: charset.unwrap_or_default(),
                        response_headers: recorded_headers(asset),
                    },
                ),
            )
        });
    let mut assets: Vec<_> = assets.collect();
    // The document comes first
    assets.sort_by_key(|(is_asset, _)| *is_asset);

    MhtmlArchive::from_assets(assets.into_iter().map(|(_, asset)| asset).collect())
}

pub fn create_monolithic_document_from_data(
    session: Session,
    input_data: Vec<u8>,
    input_encoding: Option<String>,
    input_target: Option<String>,
) -> MonolithResult {
    create_monolithic_document_from_bytes(
        session,
        Cow::Owned(input_data),
        input_encoding,
        input_target,
    )
}

/// Same as create_monolithic_document_from_data(), for data that may be someone else's (archives).
fn create_monolithic_document_from_bytes(
    mut session: Session,
    input_data: Cow<'_, [u8]>,
    input_encoding: Option<String>,
    input_target: Option<String>,
) -> MonolithResult {
    // Simplified documents have no use for JavaScript
    if session.options.simplify {
        session.options.remove_js = true;
    }

    // Validate options
    {
        // Check if custom encoding value is acceptable
        if let Some(custom_output_encoding) = session.options.encoding.clone()
            && Encoding::for_label_no_replacement(custom_output_encoding.as_bytes()).is_none()
        {
            return Err(MonolithError::new(&format!(
                "unknown encoding \"{}\"",
                custom_output_encoding
            )));
        }
    }

    // Archives that hold original responses get their pages replayed
    let mut archive: Option<MhtmlArchive> = None;
    let is_warc_archive = is_warc(&input_data);
    if is_warc_archive || is_har(&input_data) {
        // The first page, unless it's been looked for by its URL
        let wanted_url = session.page_url.take();
        let (page, has_other_pages) = match is_warc_archive {
            true => parse_warc(&input_data, wanted_url.as_deref()),
            false => parse_har(&input_data, wanted_url.as_deref()),
        }
        .map_err(|error| MonolithError::new(&error))?;
        // Whatever isn't in the archive stays missing
        session.go_offline();

        if has_other_pages && wanted_url.is_none() && !session.options.silent {
            print_info_message(&format!(
                "saving {}, the archive may have other pages: -a takes archives, for their URLs to be targets",
                page.url
            ));
        }

        if session.is_recording() {
            record_captured_page(&mut session, &page);
            return finish(session, None);
        }

        if !is_html(&page.content_type) {
            let root = page
                .assets
                .into_iter()
                .find(|asset| asset.destination == "document");
            return finish_as_is(session, &root.map(|root| root.data).unwrap_or_default());
        }
        if !session.options.remove_js
            && !session.options.simplify
            && session.options.output_format == MonolithOutputFormat::HTML
        {
            return create_monolithic_document_from_capture(session, page);
        }

        archive = Some(create_archive_from_page(&page));
    }

    // Other archives get unpacked: the document inside gets processed, the rest of the archive serves as
    // the only source of its assets
    if is_mhtml(&input_data) {
        let archive_url: Option<Url> = input_target.as_deref().and_then(|t| Url::parse(t).ok());
        archive = Some(
            parse_mhtml(&input_data, archive_url.as_ref())
                .ok_or(MonolithError::new("could not unpack MHTML archive"))?,
        );
    } else if is_webarchive(&input_data) {
        archive = Some(
            parse_webarchive(&input_data)
                .ok_or(MonolithError::new("could not unpack WebArchive"))?,
        );
    }

    let (input_data, input_encoding, input_target) = if let Some(archive) = archive {
        let root = archive.root();
        if !root.media_type.eq_ignore_ascii_case("text/html")
            && !root
                .media_type
                .eq_ignore_ascii_case("application/xhtml+xml")
        {
            if let Some(url) = &root.url
                && session.is_recording()
            {
                let (data, media_type, charset) = (
                    root.data.clone(),
                    root.media_type.clone(),
                    root.charset.clone(),
                );
                let url = url.clone();
                session.load_archive(archive);
                session.record_resource(&url, &data, &media_type, &charset);
                return finish(session, None);
            }

            return finish_as_is(session, &root.data);
        }
        let data: Cow<'_, [u8]> = Cow::Owned(root.data.clone());
        let encoding: Option<String> = Some(root.charset.clone()).filter(|c| !c.is_empty());
        // The document's original URL, unless the archive doesn't have it
        let target: Option<String> = match &root.url {
            Some(url) if url.scheme() != "data" => Some(url.to_string()),
            _ => input_target,
        };

        session.load_archive(archive);

        (data, encoding, target)
    } else {
        (input_data, input_encoding, input_target)
    };

    // Archives need to know where the document came from
    if session.is_recording() {
        let usable = |url: &String| Url::parse(url).ok().filter(|url| url.scheme() != "data");
        let document_url = input_target
            .as_ref()
            .and_then(usable)
            .or(session.options.base_url.as_ref().and_then(usable))
            .ok_or(MonolithError::new(
                "document has no URL to be archived under, -b can provide one",
            ))?;
        session.record_resource(
            &document_url,
            &input_data,
            "text/html",
            input_encoding.as_deref().unwrap_or(""),
        );
    }

    let mut base_url: Url = if input_target.is_some() {
        Url::parse(&input_target.clone().unwrap()).unwrap()
    } else {
        Url::parse("data:text/html,").unwrap()
    };
    // A BOM or a charset from the transport layer takes precedence over META
    let authoritative_encoding = Encoding::for_bom(&input_data)
        .map(|(encoding, _)| encoding)
        .or_else(|| input_encoding.as_deref().and_then(declared_encoding));
    let mut document_encoding = authoritative_encoding
        .map(|encoding| encoding.name().to_string())
        .unwrap_or("utf-8".to_string());
    let mut dom: RcDom;

    // Encoding the document declares (if any), used for decoding unlabeled stylesheets
    let mut environment_encoding = authoritative_encoding;

    // Initial parse
    dom = html_to_dom(&input_data, document_encoding.clone());

    // Attempt to determine document's encoding
    if authoritative_encoding.is_none()
        && let Some(html_charset) = get_charset(&dom.document)
        && !html_charset.is_empty()
    {
        // Check if the charset specified inside HTML is valid
        if let Some(document_charset) = Encoding::for_label_no_replacement(html_charset.as_bytes())
        {
            document_encoding = html_charset;
            environment_encoding = Some(document_charset);
            dom = html_to_dom(&input_data, document_charset.name().to_string());
        }
    }

    if authoritative_encoding.is_some() {
        // UTF-16 documents get serialized as UTF-8
        if document_encoding == "UTF-16LE" || document_encoding == "UTF-16BE" {
            document_encoding = "UTF-8".to_string();
        }
        if get_charset(&dom.document).is_some() || !input_data.is_ascii() {
            dom = set_charset(dom, document_encoding.clone());
        }
    }

    // What the document is allowed to do comes with it, and from its META elements (see
    // csp.rs); documents without a URL of their own have no origin
    let document_url: Url = input_target
        .as_deref()
        .and_then(|target| Url::parse(target).ok())
        .unwrap_or_else(|| Url::parse("data:text/html,").unwrap());
    let policies = Policies::of(
        &document_url,
        session.received_policy(&document_url).as_deref(),
    );
    // Assets only get prefetched for what's allowed to load anything
    let is_prefetchable = !document_policies(
        &dom.document,
        &document_url,
        session.received_policy(&document_url).as_deref(),
    )
    .restricts_anything();
    let _ = enter_document(&mut session, &dom, policies);
    if session.options.remove_images {
        // Images get replaced with data URLs
        session.saved_content.data.insert(Destination::Image);
    }

    // Use custom base URL if specified; read and use what's in the DOM otherwise
    let custom_base_url: String = session.options.base_url.clone().unwrap_or_default();
    if custom_base_url.is_empty() {
        // No custom base URL is specified; try to see if document has BASE element
        if get_base_url(&dom.document).is_some() {
            match base_url_allowed(&session, &dom, &base_url) {
                Some(allowed_base_url) => base_url = allowed_base_url,
                // Browsers ignore what policies don't allow
                None => {
                    for base in find_nodes(&dom.document, vec!["html", "head", "base"]) {
                        set_node_attr(&base, "href", None);
                    }
                }
            }
        }
    } else {
        // Custom base URL provided
        match Url::parse(&custom_base_url) {
            Ok(parsed_url) if !is_windows_drive_path(&custom_base_url) => {
                if parsed_url.scheme() == "file" {
                    // File base URLs can only work with documents saved from filesystem
                    if base_url.scheme() == "file" {
                        base_url = parsed_url;
                    }
                } else {
                    base_url = parsed_url;
                }
            }
            _ => {
                // Failed to parse given base URL, perhaps it's a filesystem path?
                if base_url.scheme() == "file" {
                    // Relative paths could work for documents saved from filesystem
                    let path: &Path = Path::new(&custom_base_url);
                    if path.exists() {
                        match Url::from_file_path(fs::canonicalize(path).unwrap()) {
                            Ok(file_url) => {
                                base_url = file_url;
                            }
                            Err(_) => {
                                return Err(MonolithError::new(&format!(
                                    "could not map given path to base URL \"{}\"",
                                    custom_base_url
                                )));
                            }
                        }
                    }
                }
            }
        }
    }

    let had_favicon: bool = has_favicon(&dom.document);
    if session.options.simplify
        && !simplify_document(&dom, !session.options.remove_styles)
        && !session.options.silent
    {
        print_error_message("found no article to simplify, the page may rely on JavaScript");
    }
    strip_elements(
        &dom.document,
        &session.options.remove_elements,
        &session.options.wipe_elements,
    );

    // Relative links would lead elsewhere for documents with a base URL, and MHTML archives
    // have their documents' original URLs as base URLs
    session.links_are_absolute = get_base_url(&dom.document).is_some()
        || session.options.base_url.is_some()
        || session.options.output_format == MonolithOutputFormat::MHTML;

    // Where the document is from decides what other origins allow it to use (base URLs don't
    // change that), documents with no URL have no origin
    session.page_origin = match session.content_security.has_opaque_origin() {
        true => Some(PageOrigin::Opaque),
        false => input_target
            .as_deref()
            .and_then(|target| Url::parse(target).ok())
            .and_then(|url| PageOrigin::of(&url)),
    };

    // The result can start being written as soon as it's known what to call it: what's done
    // with the document doesn't change its title
    let document_title: Option<String> = session.title.take().or_else(|| get_title(&dom.document));
    session.open_output(document_title.as_deref())?;

    if is_prefetchable {
        session.prefetch_assets(&base_url, &dom.document);
    }

    // A byte order mark beats any declaration when it comes to decoding the document
    if let Some((bom_encoding, _)) = Encoding::for_bom(&input_data) {
        environment_encoding = Some(bom_encoding);
    }

    // Traverse through the document and embed remote assets
    // A document can refer to itself by the URL it was requested from, and a base URL can make it
    // get processed under another one; neither one can be taken for a cycle before the walk starts
    let redirected_target: Option<Url> = session.take_redirected_target();
    let input_target_url: Option<Url> = input_target
        .as_deref()
        .and_then(|target| Url::parse(target).ok())
        .filter(|url| url.scheme() != "data");
    let mut input_target_aliases: Vec<String> = vec![];
    for target_url in [redirected_target.as_ref(), input_target_url.as_ref()]
        .into_iter()
        .flatten()
    {
        if let Some(key) = session.enter_expansion_alias(target_url, &base_url) {
            input_target_aliases.push(key);
        }
    }

    session.environment_encoding = environment_encoding;
    walk(&mut session, &base_url, &dom.document);

    for key in input_target_aliases {
        session.leave_expansion_alias(&key);
    }

    if session.is_recording() {
        // What has been put together only served to find out what it's made of
        return finish(session, document_title);
    }

    // Stylesheets go where they can be as long as they need to be
    if session.options.output_format == MonolithOutputFormat::HTML {
        inline_stylesheets(&dom, &dom.document);
    }

    // Update or add new BASE element to reroute network requests and hash-links
    if let Some(new_base_url) = session.options.base_url.clone() {
        dom = set_base_url(&dom.document, new_base_url);
    }

    // Request and embed /favicon.ico (unless it's already linked in the document)
    if !session.options.remove_images
        && !(had_favicon || targets_element_anywhere(&session.options, &["head", "html", "link"]))
        && !session.has_archive()
        && (base_url.scheme() == "http" || base_url.scheme() == "https")
        && (input_target.is_some()
            && (input_target.as_ref().unwrap().starts_with("http:")
                || input_target.as_ref().unwrap().starts_with("https:")))
        && !has_favicon(&dom.document)
    {
        let favicon_ico_url: Url = resolve_url(&base_url, "/favicon.ico");

        match session.retrieve_asset_for(
            &base_url,
            &favicon_ico_url,
            &Request::to(Destination::Image),
        ) {
            Ok((data, final_url, media_type, charset)) => {
                let favicon_data_url: String =
                    create_data_url(&media_type, &charset, data, &final_url, None);
                dom = add_favicon(&dom.document, favicon_data_url);
            }
            Err(_) => {
                // Failed to retrieve /favicon.ico
            }
        }
    }

    // Append noindex META-tag
    let meta_robots_content_value = get_robots(&dom.document).unwrap_or_default();
    if meta_robots_content_value.trim().is_empty() || meta_robots_content_value != "none" {
        dom = set_robots(dom, "none");
    }

    // Save using specified charset, if given
    if let Some(custom_encoding) = session.options.encoding.clone() {
        document_encoding = custom_encoding;
        dom = set_charset(dom, document_encoding.clone());
    }

    if let Some(output_encoding) = &session.output_encoding {
        let _ = output_encoding.set(document_encoding.clone());
    }

    // Say what has saved it, when, and where from: documents without a location of their own
    // (given through stdin) can get one from the base URL
    set_metadata(
        &dom,
        input_target
            .as_deref()
            .or(session.options.base_url.as_deref())
            .and_then(|target| Url::parse(target).ok())
            .as_ref(),
    );

    // Policies of the page, made to allow what it's been made of (see csp.rs), and what Monolith
    // keeps it from doing
    hash_named_elements(&mut session, &dom.document);
    set_content_security_policies(&dom, &saved_policies(&session));

    let mut output = session.take_output(document_title.as_deref())?;

    if session.options.output_format == MonolithOutputFormat::MHTML {
        // Document's own URL (data URLs aren't of much use in archives)
        let document_location: Option<Url> = input_target
            .as_deref()
            .and_then(|target| Url::parse(target).ok())
            .filter(|url| url.scheme() != "data");
        let html: Vec<u8> = serialize_document(dom, document_encoding.clone(), &session.options);
        let result = write_mhtml_document(
            &mut session,
            &mut output,
            html,
            &base_url,
            document_location.as_ref(),
            &document_encoding,
            document_title.as_deref(),
        );
        session.put_output(output);
        result?;
    } else {
        let markup = session.replay_markup.take();
        let result = serialize_document_into(
            dom,
            document_encoding,
            &session.options,
            markup.as_deref().map(str::as_bytes),
            &mut output,
        )
        .and_then(|last_byte| match last_byte {
            // Results end with a newline
            Some(b'\n') => Ok(()),
            _ => output.write_all(b"\n"),
        });
        session.put_output(output);
        result?;
    }

    finish(session, document_title)
}

pub fn create_monolithic_document(mut session: Session, target: String) -> MonolithResult {
    // Check if target was provided
    if target.is_empty() {
        return Err(MonolithError::new("no target specified"));
    }

    // Validate options
    {
        // Check if custom encoding value is acceptable
        if let Some(custom_encoding) = session.options.encoding.clone()
            && Encoding::for_label_no_replacement(custom_encoding.as_bytes()).is_none()
        {
            return Err(MonolithError::new(&format!(
                "unknown encoding \"{}\"",
                custom_encoding
            )));
        }
    }

    let target_with_credentials = parse_target(&target)?;
    // Credentials go along with requests from now on, and never into what gets saved
    let mut target_url = session.take_credentials(&target_with_credentials);

    if session.uses_archives() {
        // Everything comes from archives, nothing from anywhere else
        let archive = archive_with(&session, &target_url)?;
        session.page_url = Some(target_url.to_string());

        return create_monolithic_document_from_bytes(
            session,
            Cow::Borrowed(&archive),
            None,
            Some(target_url.to_string()),
        );
    }

    #[cfg(feature = "browser")]
    if session.options.browser {
        // Browsers deal with credentials on their own
        let page = render_page(target_with_credentials.as_str(), &session)
            .map_err(|error| MonolithError::new(&error))?;

        if is_captured_document(&page) || session.is_recording() {
            return create_monolithic_document_from_capture(session, page);
        }

        // Images, PDFs, and such have nothing to render, they get saved the usual way
        add_captured_assets(&mut session, &page.assets);
    }

    let data: Vec<u8>;
    let document_encoding: Option<String>;

    // Retrieve target document
    if target_url.scheme() == "file"
        || target_url.scheme() == "http"
        || target_url.scheme() == "https"
        || target_url.scheme() == "data"
    {
        match session.retrieve_asset(&target_url, &target_url) {
            Ok((retrieved_data, final_url, media_type, charset)) => {
                if !media_type.eq_ignore_ascii_case("text/html")
                    && !media_type.eq_ignore_ascii_case("application/xhtml+xml")
                    && !is_archive(&retrieved_data)
                {
                    if session.is_recording() {
                        let resource_url = match target_url.scheme() == "data" {
                            true => session
                                .options
                                .base_url
                                .as_deref()
                                .and_then(|url| Url::parse(url).ok())
                                .filter(|url| url.scheme() != "data")
                                .ok_or(MonolithError::new(
                                    "document has no URL to be archived under, -b can provide one",
                                ))?,
                            false => final_url.clone(),
                        };
                        session.record_resource(
                            &resource_url,
                            &retrieved_data,
                            &media_type,
                            &charset,
                        );
                        return finish(session, None);
                    }

                    return finish_as_is(session, &retrieved_data);
                }

                // If got redirected, set target_url to that
                if final_url != target_url {
                    // The document can still refer to itself by the URL it was requested from
                    session.set_redirected_target(target_url.clone());
                    target_url = final_url.clone();
                }

                data = retrieved_data;
                document_encoding = Some(charset);
            }
            Err(_) => {
                return Err(MonolithError::new("could not retrieve target document"));
            }
        }
    } else {
        return Err(MonolithError::new("unsupported target"));
    }

    create_monolithic_document_from_data(
        session,
        data,
        document_encoding,
        Some(target_url.to_string()),
    )
}

/// Turns what's been given as a target into the URL of it: file paths become file URLs,
/// and what's neither a URL nor a path is assumed to be a website.
pub fn parse_target(target: &str) -> Result<Url, MonolithError> {
    if target.is_empty() {
        return Err(MonolithError::new("no target specified"));
    }

    let target_str = target;
    let target_url = match Url::parse(target_str) {
        Ok(target_url) if !is_windows_drive_path(target_str) => match target_url.scheme() {
            "data" | "file" | "http" | "https" => target_url,
            unsupported_scheme => {
                return Err(MonolithError::new(&format!(
                    "unsupported target URL scheme \"{}\"",
                    unsupported_scheme
                )));
            }
        },
        _ => {
            // Failed to parse given base URL (perhaps it's a filesystem path?)
            let path: &Path = Path::new(&target_str);

            match path.exists() {
                true => match path.is_file() {
                    true => {
                        let canonical_path = fs::canonicalize(path).unwrap();

                        match Url::from_file_path(canonical_path) {
                            Ok(url) => url,
                            Err(_) => {
                                return Err(MonolithError::new(&format!(
                                    "could not generate file URL out of given path \"{}\"",
                                    target_str
                                )));
                            }
                        }
                    }
                    false => {
                        return Err(MonolithError::new(&format!(
                            "local target \"{}\" is not a file",
                            target_str
                        )));
                    }
                },
                false => {
                    // It is not a FS path, now we do what browsers do:
                    // prepend "http://" and hope it points to a website
                    Url::parse(&format!("http://{}", target_str)).map_err(|_| {
                        MonolithError::new(&format!(
                            "could not make sense of target \"{}\"",
                            target_str
                        ))
                    })?
                }
            }
        }
    };

    Ok(target_url)
}

/// What a target has turned out to be, before getting saved.
pub struct ProbedTarget {
    /// Title of the page, if it has one
    pub title: Option<String>,
    /// Where it has ended up being, after redirects
    pub url: Url,
}

/// Finds out what a target is called and where it really is, without saving anything: the
/// way it gets done depends on the session, browsers get used only if it says so. What gets
/// retrieved stays in the session's cache, saving it afterwards doesn't have to retrieve it again.
pub fn probe_target(mut session: Session, target: &str) -> Result<ProbedTarget, MonolithError> {
    let target_with_credentials = parse_target(target)?;
    let target_url = session.take_credentials(&target_with_credentials);

    if session.uses_archives() {
        let archive = archive_with(&session, &target_url)?;
        let (title, url) = probe_archive(&archive, target_url.as_str());

        return Ok(ProbedTarget {
            title,
            url: url.unwrap_or(target_url),
        });
    }

    #[cfg(feature = "browser")]
    if session.options.browser {
        let page = render_page(target_with_credentials.as_str(), &session)
            .map_err(|error| MonolithError::new(&error))?;

        return Ok(ProbedTarget {
            title: Some(page.title).filter(|title| !title.trim().is_empty()),
            url: Url::parse(&page.url).unwrap_or(target_url),
        });
    }

    let (data, url, media_type, charset) = session
        .retrieve_asset(&target_url, &target_url)
        .map_err(|_| MonolithError::new("could not retrieve target document"))?;
    let is_document = media_type.eq_ignore_ascii_case("text/html")
        || media_type.eq_ignore_ascii_case("application/xhtml+xml");
    let title = match is_document || is_archive(&data) {
        true => probe_data(&data, Some(&charset), Some(&url)),
        false => None,
    };

    Ok(ProbedTarget { title, url })
}

/// Finds out what a document is called, for documents that have already been obtained:
/// archives are called whatever the page they have in them is called.
pub fn probe_data(data: &[u8], charset: Option<&str>, url: Option<&Url>) -> Option<String> {
    if is_warc(data) || is_har(data) {
        probe_page_in_archive(data, None).0
    } else if is_mhtml(data) || is_webarchive(data) {
        let archive = match is_mhtml(data) {
            true => parse_mhtml(data, url),
            false => parse_webarchive(data),
        }?;
        let root = archive.root();

        is_html(&root.media_type).then(|| title_of(&root.data, Some(&root.charset)))?
    } else {
        title_of(data, charset)
    }
}

/// Title of a page in a WARC or HAR archive (the first one, unless it's looked for by its URL),
/// and where it has ended up being.
fn probe_page_in_archive(data: &[u8], wanted_url: Option<&str>) -> (Option<String>, Option<Url>) {
    let Ok((page, _)) = (match is_warc(data) {
        true => parse_warc(data, wanted_url),
        false => parse_har(data, wanted_url),
    }) else {
        return (None, None);
    };
    let title = page
        .assets
        .iter()
        .find(|asset| asset.destination == "document")
        .filter(|_| is_html(&page.content_type))
        .and_then(|document| {
            let charset = parse_media_type(&document.content_type).1;
            title_of(&document.data, charset.as_deref())
        });

    (title, Url::parse(&page.url).ok())
}

/// Title of the page at given URL in an archive, and where it has ended up being.
fn probe_archive(data: &[u8], url: &str) -> (Option<String>, Option<Url>) {
    if is_warc(data) || is_har(data) {
        return probe_page_in_archive(data, Some(url));
    }

    let archive = match is_mhtml(data) {
        true => parse_mhtml(data, None),
        false => parse_webarchive(data),
    };
    let title = archive.and_then(|archive| {
        let root = archive.root();
        is_html(&root.media_type).then(|| title_of(&root.data, Some(&root.charset)))?
    });

    (title, None)
}

/// Archives to take everything from: WARC archives get put together into one (the way
/// concatenated WARC files are one), pages in them may have their assets in others.
pub fn read_archives(archives: Vec<Vec<u8>>) -> Result<Vec<Arc<Vec<u8>>>, MonolithError> {
    let mut warc: Vec<u8> = vec![];
    let mut others: Vec<Arc<Vec<u8>>> = vec![];

    for (index, data) in archives.into_iter().enumerate() {
        if is_warc(&data) {
            warc.extend_from_slice(&decompress_warc(data));
        } else if is_har(&data) || is_mhtml(&data) || is_webarchive(&data) {
            others.push(Arc::new(data));
        } else {
            return Err(MonolithError::new(&format!(
                "archive #{} is neither WARC, HAR, MHTML, nor WebArchive",
                index + 1
            )));
        }
    }

    if !warc.is_empty() {
        others.insert(0, Arc::new(warc));
    }

    Ok(others)
}

/// Archive that has given URL in it, out of those the session takes everything from.
fn archive_with(session: &Session, url: &Url) -> Result<Arc<Vec<u8>>, MonolithError> {
    let wanted = clean_url(url.clone());

    session
        .source_archives
        .iter()
        .find(|data| {
            if is_warc(data) || is_har(data) {
                return match is_warc(data) {
                    true => parse_warc(data, Some(url.as_str())),
                    false => parse_har(data, Some(url.as_str())),
                }
                .is_ok();
            }

            match is_mhtml(data) {
                true => parse_mhtml(data, None),
                false => parse_webarchive(data),
            }
            .and_then(|archive| archive.root().url.clone())
            .is_some_and(|root_url| clean_url(root_url) == wanted)
        })
        .cloned()
        .ok_or(MonolithError::new(&format!(
            "none of the archives have {}",
            url
        )))
}

/// Title of an HTML document, decoded the way the document would be.
fn title_of(data: &[u8], charset: Option<&str>) -> Option<String> {
    let encoding = Encoding::for_bom(data)
        .map(|(encoding, _)| encoding)
        .or_else(|| charset.and_then(declared_encoding));
    let dom = html_to_dom(data, encoding.unwrap_or(UTF_8).name().to_string());
    let dom = match encoding {
        None => match get_charset(&dom.document)
            .and_then(|charset| Encoding::for_label_no_replacement(charset.as_bytes()))
        {
            Some(encoding) if encoding != UTF_8 => html_to_dom(data, encoding.name().to_string()),
            _ => dom,
        },
        Some(_) => dom,
    };

    get_title(&dom.document).filter(|title| !title.trim().is_empty())
}

/// Something to name a file by, for pages that have no title: the last part of the URL's path,
/// or the host, the way browsers name downloads.
pub fn untitled_name(url: &Url) -> String {
    let segment = url
        .path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
        .map(percent_decode)
        .map(|segment| match segment.rfind('.') {
            Some(dot) if dot > 0 => segment[..dot].to_string(),
            _ => segment,
        })
        .filter(|segment| !segment.trim().is_empty());

    match url.scheme() {
        "http" | "https" | "file" => segment
            .or(url.host_str().map(str::to_string))
            .unwrap_or("Untitled".to_string()),
        _ => "Untitled".to_string(),
    }
}

#[cfg(feature = "browser")]
fn is_captured_document(page: &CapturedPage) -> bool {
    page.content_type == "text/html" || page.content_type == "application/xhtml+xml"
}

/// Saves requests, and works for what servers wouldn't give to anyone but who received it.
fn add_captured_assets(session: &mut Session, assets: &[CapturedAsset]) {
    for asset in assets.iter().filter(|asset| asset.is_reusable) {
        let (media_type, charset, _) = parse_media_type(&asset.content_type);
        let urls: Vec<Url> = asset
            .urls
            .iter()
            .filter_map(|url| Url::parse(url).ok())
            .collect();

        // The last URL is where the request ended up
        for url in &urls {
            session.add_asset_with_final_url(
                url,
                urls.last(),
                &asset.data,
                media_type.clone(),
                charset.clone().unwrap_or_default(),
            );
        }
        if let (Some(headers), Some(final_url)) = (recorded_headers(asset), urls.last()) {
            session.record_headers(&urls, final_url, headers);
        }
    }
}

/// Headers something has been received with, if it was received over HTTP.
fn recorded_headers(asset: &CapturedAsset) -> Option<Vec<(String, String)>> {
    (!asset.http.is_resource).then(|| asset.http.response_headers.clone())
}

/// Creates a document out of a page that has already been loaded by something else.
///
/// If scripts are to be kept, and the capture has the original document (an asset with destination
/// "document" and status 200), the result starts over from it and replays captured responses.
/// Otherwise it's a frozen copy of what the page has turned into.
pub fn create_monolithic_document_from_capture(
    mut session: Session,
    mut page: CapturedPage,
) -> MonolithResult {
    // The browser has seen to what other origins allow
    session.is_captured = true;
    // What browsers report may have credentials in it, those don't get saved
    let mut strip = |url: &mut String| {
        if let Ok(parsed) = Url::parse(url)
            && (!parsed.username().is_empty() || parsed.password().is_some())
        {
            *url = session.take_credentials(&parsed).to_string();
        }
    };
    strip(&mut page.url);
    for asset in page.assets.iter_mut() {
        asset.urls.iter_mut().for_each(&mut strip);
    }

    if session.is_recording() {
        record_captured_page(&mut session, &page);
        // Without the page itself there would be nothing to open, what the browser has ended up
        // with is the closest thing to it
        let has_page = page.assets.iter().any(|asset| {
            asset.destination == "document" && asset.urls.iter().any(|url| url == &page.url)
        });
        if !has_page && let Ok(url) = Url::parse(&page.url) {
            if !session.options.silent {
                print_error_message(&format!(
                    "{} (browser has not given its original, saving what it has made of it)",
                    page.url
                ));
            }
            session.record_resource(&url, page.html.as_bytes(), "text/html", "utf-8");
        }
        let title = Some(page.title).filter(|title| !title.is_empty());
        return finish(session, title);
    }

    let source_document = page
        .assets
        .iter()
        .rfind(|asset| asset.destination == "document" && asset.status == 200)
        .cloned();

    if let Some(source_document) = source_document.filter(|_| {
        !session.options.remove_js
            && !session.options.simplify
            && session.options.output_format == MonolithOutputFormat::HTML
    }) {
        session.options.replay = true;
        // Scripts of the document have to find everything right where they found it in the
        // browser, no other base URL can work for it
        session.options.base_url = None;

        // Integrity is about what things were like before getting rewritten
        for asset in page
            .assets
            .iter()
            .filter(|asset| is_script(asset) || asset.content_type.starts_with("text/css"))
        {
            for url in asset.urls.iter().filter_map(|url| Url::parse(url).ok()) {
                session.add_original_data(&url, asset.data.clone());
            }
        }
        // What doesn't say how it's encoded goes by the document
        let document_encoding = parse_media_type(&source_document.content_type)
            .1
            .and_then(|charset| declared_encoding(&charset))
            .or_else(|| {
                let dom = html_to_dom(&source_document.data, "utf-8".to_string());
                get_charset(&dom.document)
                    .and_then(|charset| Encoding::for_label_no_replacement(charset.as_bytes()))
            });
        let imports = rewrite_scripts(&mut page.assets, document_encoding);
        rewrite_stylesheets(&mut page.assets, document_encoding);
        add_captured_assets(&mut session, &page.assets);

        let markup = create_replay_markup(
            &page.assets,
            &imports,
            &page.url,
            Utc::now().timestamp_millis(),
        );
        let charset = parse_media_type(&source_document.content_type).1;
        let source_url = source_document.urls.last().cloned();
        // What the page has ended up calling itself beats what it started out with
        session.title = Some(page.title).filter(|title| !title.is_empty());
        session.replay_markup = Some(markup);

        return create_monolithic_document_from_data(
            session,
            source_document.data,
            charset,
            source_url,
        );
    }

    add_captured_assets(&mut session, &page.assets);
    session.adopted_style_marker = page.adopted_style_marker.take();

    // What a browser leaves behind has already been through JavaScript, running it again would
    // only break it
    session.options.remove_js = true;

    create_monolithic_document_from_data(
        session,
        page.html.into_bytes(),
        Some("utf-8".to_string()),
        Some(page.url),
    )
}

pub fn detect_media_type(data: &[u8], url: &Url) -> String {
    // Detect by file signature, falling back to the file name's extension
    let file_name: Option<&str> = url.path().rsplit('/').next();
    dataurl::detect_media_type(data, file_name)
        .unwrap_or_default()
        .to_string()
}

// Extensions made of more than one part, the number goes in front of the whole thing
const COMPOUND_EXTENSIONS: [&str; 1] = [".warc.gz"];
// There's a limit to how many files with the same name are worth trying
const MAX_NUMBERED_PATHS: usize = 10_000;

/// Path with a number added to the file's name, the way browsers name downloads that would
/// otherwise replace something: "page.html" becomes "page (2).html".
pub fn numbered_path(path: &str, number: usize) -> String {
    let name_start = path.rfind(['/', MAIN_SEPARATOR]).map_or(0, |i| i + 1);
    let (directory, name) = path.split_at(name_start);
    let lowercase = name.to_lowercase();
    let extension_start = COMPOUND_EXTENSIONS
        .iter()
        .find(|extension| lowercase.ends_with(*extension) && name.len() > extension.len())
        .map(|extension| name.len() - extension.len())
        // Names that start with a dot don't have an extension because of it
        .or_else(|| name.rfind('.').filter(|i| *i > 0))
        .unwrap_or(name.len());
    let (stem, extension) = name.split_at(extension_start);

    format!("{}{} ({}){}", directory, stem, number, extension)
}

/// Creates the file to save to, without ever replacing anything: if there's something there
/// already, a number gets added to the new file's name. Directories it's supposed to be in get
/// created too. Provides the file, and where it is.
pub fn create_output_file(path: &str) -> io::Result<(File, String)> {
    if let Some(directory) = Path::new(path)
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
    {
        fs::create_dir_all(directory).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "could not create directory {}: {}",
                    directory.display(),
                    error
                ),
            )
        })?;
    }

    for number in 1..=MAX_NUMBERED_PATHS {
        let candidate = match number {
            1 => path.to_string(),
            _ => numbered_path(path, number),
        };
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((file, candidate)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(io::Error::new(
                    error.kind(),
                    format!("could not create file {}: {}", candidate, error),
                ));
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("there are too many files named like {} already", path),
    ))
}

/// Endings that make output paths name files, rather than directories to save files in.
const FILE_EXTENSIONS: [&str; 8] = [
    ".%ext%",
    ".%extension%",
    ".htm",
    ".html",
    ".mht",
    ".mhtml",
    ".warc",
    ".warc.gz",
];

/// Makes sure output paths name files: paths that end with a slash (or a backslash, on
/// Windows), or with no file extension (.html, .%extension%…), are directories to save files in,
/// those get named with given pattern. Doesn't touch paths that name files already.
pub fn output_file_pattern(path: &str, file_name: &str) -> String {
    let is_separator = |c: char| c == '/' || (cfg!(windows) && c == '\\');
    if path.ends_with(is_separator) {
        return format!("{}{}", path, file_name);
    }

    let last_part = path
        .rsplit(is_separator)
        .next()
        .unwrap_or(path)
        .to_lowercase();
    if FILE_EXTENSIONS
        .iter()
        .any(|extension| last_part.ends_with(extension))
    {
        path.to_string()
    } else {
        format!("{}{}{}", path, MAIN_SEPARATOR, file_name)
    }
}

/// Makes a path out of a pattern: %title% and %url% get replaced with what the document is
/// called and where it's from (made fit for file names), %timestamp% with the current time,
/// %extension% and %ext% with what files of given format are named with.
pub fn format_output_path(
    path: &str,
    document_title: &str,
    document_url: Option<&Url>,
    output_format: MonolithOutputFormat,
) -> String {
    let datetime: String = Utc::now()
        .to_rfc3339_opts(SecondsFormat::Secs, true)
        .replace(':', "_");
    let title: String = file_name_part(
        &document_title
            .replace(['/', '\\'], "_")
            .replace('<', "[")
            .replace('>', "]")
            .replace(':', " - ")
            .replace('\"', "")
            .replace('|', "-")
            .replace('?', ""),
    );
    let url: String = document_url.map_or(String::new(), |url| file_name_part(&url_as_name(url)));
    let (extension, ext) = match output_format {
        MonolithOutputFormat::HTML => ("html", "htm"),
        MonolithOutputFormat::MHTML => ("mhtml", "mht"),
        MonolithOutputFormat::WARC => ("warc", "warc"),
        MonolithOutputFormat::WARCGZ => ("warc.gz", "warc.gz"),
    };
    // Files named after nothing would end up hidden, or nameless
    let or_untitled = |name: String| match name.is_empty() {
        true => "Untitled".to_string(),
        false => name,
    };
    let placeholders: [(&str, String); 5] = [
        ("%timestamp%", datetime),
        ("%title%", or_untitled(title)),
        ("%url%", or_untitled(url)),
        ("%extension%", extension.to_string()),
        ("%ext%", ext.to_string()),
    ];

    // All at once, what titles and URLs have in them doesn't get replaced
    let mut result = String::new();
    let mut rest: &str = path;
    while let Some(start) = rest.find('%') {
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        match placeholders
            .iter()
            .find(|(placeholder, _)| rest.starts_with(placeholder))
        {
            Some((placeholder, value)) => {
                result.push_str(value);
                rest = &rest[placeholder.len()..];
            }
            None => {
                result.push('%');
                rest = &rest[1..];
            }
        }
    }
    result.push_str(rest);

    result
}

/// How long names that come from documents can get (in bytes): file systems allow 255,
/// some of it has to stay for numbers that keep names unique, and for extensions.
const MAX_FILE_NAME_PART_LENGTH: usize = 200;

/// Makes something fit for file names, the way browsers do when saving pages: newlines, tabs,
/// other whitespace and control characters become single spaces, with no spaces or dots at either
/// end (those would make for hidden files), and names that are too long get cut short.
fn file_name_part(text: &str) -> String {
    let text: String = text
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|word| !word.is_empty())
        .collect::<Vec<&str>>()
        .join(" ");
    let mut text: &str = text.trim_matches(['.', ' ']);

    if text.len() > MAX_FILE_NAME_PART_LENGTH {
        let mut end = MAX_FILE_NAME_PART_LENGTH;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text = text[..end].trim_end_matches(['.', ' ']);
    }

    text.to_string()
}

/// What a URL is, as a name: its host and path (and query, if it has one), with "--" where
/// slashes were, and without what file names can't have (github.com--Y2Z--monolith--issues--169).
/// User names and passwords never make it in.
pub fn url_as_name(url: &Url) -> String {
    let mut parts: Vec<String> = vec![];

    if let Some(host) = url.host_str() {
        parts.push(match url.port() {
            Some(port) => format!("{}_{}", host, port),
            None => host.to_string(),
        });
    }
    match url.path_segments() {
        Some(segments) => parts.extend(
            segments
                .filter(|segment| !segment.is_empty())
                .map(percent_decode),
        ),
        // Data URLs and such are too much to name files after
        None => parts.push(url.scheme().to_string()),
    }
    if let Some(query) = url.query().filter(|query| !query.is_empty()) {
        parts.push(percent_decode(query));
    }

    parts
        .iter()
        .map(|part| part.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_"))
        .collect::<Vec<String>>()
        .join("--")
}

/// Maps a charset label (Content-Type header, data URL, META element) onto an encoding.
/// Returns None for labels that don't declare anything usable: empty (no charset given) and unknown ones.
/// Like in browsers, "US-ASCII" is a valid label, and means windows-1252.
pub fn declared_encoding(charset: &str) -> Option<&'static Encoding> {
    let charset = charset.trim();

    if charset.is_empty() {
        return None;
    }

    Encoding::for_label_no_replacement(charset.as_bytes())
}

pub fn is_plaintext_media_type(media_type: &str) -> bool {
    media_type.to_lowercase().as_str().starts_with("text/")
        || PLAINTEXT_MEDIA_TYPES.contains(&media_type.to_lowercase().as_str())
}

/// Parses a Content-Type header (or META content), returning its media type, charset (None if not given),
/// and whether it has a base64 marker. Parameters are parsed the way the WHATWG MIME Sniffing Standard
/// does it (https://mimesniff.spec.whatwg.org/#parsing-a-mime-type): quoted values may contain semicolons
/// and escaped characters, whitespace around "=" makes a parameter invalid, and the first valid charset wins.
pub fn parse_media_type(content_type: &str) -> (String, Option<String>, bool) {
    let is_http_whitespace = |c: char| matches!(c, '\t' | '\n' | '\r' | ' ');
    let is_quoted_string_token = |c: char| matches!(c, '\t' | ' '..='~' | '\u{80}'..='\u{ff}');

    let s: &str = content_type.trim_matches(is_http_whitespace);
    let b: &[u8] = s.as_bytes();

    let media_type_end: usize = s.find(';').unwrap_or(s.len());
    let media_type: &str = s[..media_type_end].trim();
    let media_type: String = if media_type.is_empty() {
        "text/plain".to_string()
    } else {
        media_type.to_string()
    };
    let mut charset: Option<String> = None;
    let mut is_base64: bool = false;

    // All delimiters are ASCII, so byte offsets always land on char boundaries
    let mut pos: usize = media_type_end;
    while pos < b.len() {
        // Skip past the semicolon and any whitespace after it
        pos += 1;
        while pos < b.len() && is_http_whitespace(b[pos] as char) {
            pos += 1;
        }

        let name_start: usize = pos;
        while pos < b.len() && b[pos] != b';' && b[pos] != b'=' {
            pos += 1;
        }
        let name: &str = &s[name_start..pos];

        if pos >= b.len() || b[pos] == b';' {
            // A parameter without a value, only meaningful as data URLs' base64 marker
            if name.trim().eq_ignore_ascii_case("base64") {
                is_base64 = true;
            }
            continue;
        }
        pos += 1; // Skip past "="

        let value: String = if pos < b.len() && b[pos] == b'"' {
            // Quoted string: backslash escapes the next character, anything after the closing quote is ignored
            let mut value = String::new();
            pos += 1;
            while let Some(c) = s[pos..].chars().next() {
                pos += c.len_utf8();
                match c {
                    '"' => break,
                    '\\' => match s[pos..].chars().next() {
                        Some(escaped) => {
                            value.push(escaped);
                            pos += escaped.len_utf8();
                        }
                        None => value.push('\\'),
                    },
                    _ => value.push(c),
                }
            }
            while pos < b.len() && b[pos] != b';' {
                pos += 1;
            }
            value
        } else {
            let value_start: usize = pos;
            while pos < b.len() && b[pos] != b';' {
                pos += 1;
            }
            let value: &str = s[value_start..pos].trim_end_matches(is_http_whitespace);
            if value.is_empty() {
                continue;
            }
            value.to_string()
        };

        if charset.is_none()
            && name.eq_ignore_ascii_case("charset")
            && value.chars().all(is_quoted_string_token)
        {
            charset = Some(value);
        }
    }

    (media_type, charset, is_base64)
}

pub fn print_error_message(text: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    const ENV_VAR_NO_COLOR: &str = "NO_COLOR";
    const ENV_VAR_TERM: &str = "TERM";

    let mut no_color = env::var_os(ENV_VAR_NO_COLOR).is_some() || !std::io::stderr().is_terminal();
    if let Some(term) = env::var_os(ENV_VAR_TERM)
        && term == "dumb"
    {
        no_color = true;
    }

    if handle
        .write_all(
            format!(
                "{}{}{}\n",
                if no_color { "" } else { ANSI_COLOR_RED },
                text,
                if no_color { "" } else { ANSI_COLOR_RESET },
            )
            .as_bytes(),
        )
        .is_ok()
    {}
}

pub fn print_info_message(text: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    if handle.write_all(format!("{}\n", text).as_bytes()).is_ok() {}
}
