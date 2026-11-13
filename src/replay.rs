//! Makes saved documents work without a network, by storing responses their scripts have received,
//! and a script that hands those responses back to them.

use std::collections::{HashMap, HashSet, VecDeque};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::{DateTime, TimeZone, Utc};
use encoding_rs::Encoding;
use serde_json::json;
use url::Url;

use crate::cache::Exchange;
use crate::core::parse_media_type;
use crate::cors::was_blocked;
use crate::css::decode_stylesheet;
use crate::source_maps::remove_source_map_reference_from_text;

/// What else there is to know about a response, for archives that keep track of it.
#[derive(Clone, Debug, Default)]
pub struct HttpDetails {
    /// "HTTP/1.1", "HTTP/2"... (empty if unknown)
    pub http_version: String,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    pub status_text: String,
    /// When it was received, in milliseconds since the epoch (zero if unknown)
    pub time: i64,
    /// Responses that have redirected to it
    pub redirects: Vec<Redirect>,
    /// Whether it wasn't received over HTTP at all, the way local files and records of archives
    /// that say so weren't
    pub is_resource: bool,
}

/// A response that has sent the request somewhere else.
#[derive(Clone, Debug, Default)]
pub struct Redirect {
    /// Method of the request that got redirected, the next one may use another one
    pub method: String,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    pub status: u64,
    pub status_text: String,
    pub url: String,
}

/// Something a page has received while it was being loaded.
#[derive(Clone, Debug, Default)]
pub struct CapturedAsset {
    pub content_type: String,
    pub data: Vec<u8>,
    /// What it was requested for, in terms of the Fetch standard: "document", "script", "style",
    /// "image", "font", "json", "worker", or nothing for what scripts asked for
    pub destination: String,
    /// Whether it's something that can stand in for retrieving the URL
    pub is_reusable: bool,
    pub method: String,
    pub status: u64,
    /// Where it was requested from, followed by the ones it got redirected to
    pub urls: Vec<String>,
    pub http: HttpDetails,
}

/// A page that was loaded by something other than Monolith, be that a browser, or a web view of an
/// app. Assets should be in the order they've been received in.
#[derive(Clone, Debug, Default)]
pub struct CapturedPage {
    pub assets: Vec<CapturedAsset>,
    pub content_type: String,
    /// What the page has turned into, for when it gets saved without JavaScript
    pub html: String,
    /// What STYLE elements made out of constructed stylesheets are marked with in there (those
    /// aren't up to policies, see csp.rs)
    pub adopted_style_marker: Option<String>,
    pub title: String,
    pub url: String,
}

/// A response stored in an archive, the way it left the server.
#[derive(Clone, Default)]
pub struct ArchivedResponse {
    pub content_type: String,
    pub data: Vec<u8>,
    pub http: HttpDetails,
    /// Where it redirects to, as given by the server
    pub location: Option<String>,
    pub method: String,
    pub status: u64,
    pub url: String,
}

const EMULATOR: &str = include_str!("../assets/scripts/replay.js");
const LOCATION_REPLACEMENT: &str = "__monolithLocation";

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Text inside of a SCRIPT element can't be allowed to close it.
pub(crate) fn to_script_literal(value: &str) -> String {
    json!(value).to_string().replace('<', "\\u003c")
}

/// Tells whether the page's scripts could ask for this.
/// Fonts only ever get requested by stylesheets, and the document itself has already been saved.
fn is_worth_storing(asset: &CapturedAsset) -> bool {
    !matches!(asset.destination.as_str(), "document" | "font")
}

/// Creates what has to go in the beginning of the document for it to work on its own.
#[doc(hidden)]
pub fn create_replay_markup(
    assets: &[CapturedAsset],
    imports: &[(String, String, String)],
    page_url: &str,
    captured_at: i64,
) -> String {
    let mut markup = String::new();
    // Pages tend to ask for the same things many times, each of those gets stored once, along with
    // everywhere it's been asked for from before getting redirected
    let mut stored: Vec<(&CapturedAsset, Vec<&str>)> = vec![];
    let mut positions: HashMap<(&String, &String, &Vec<u8>), usize> = HashMap::new();

    for asset in assets.iter().filter(|asset| is_worth_storing(asset)) {
        let Some(url) = asset.urls.last() else {
            continue;
        };
        // The import map has its own copy
        if imports
            .iter()
            .any(|(imported_url, _, _)| imported_url == url)
        {
            continue;
        }
        let aliases = asset.urls[..asset.urls.len() - 1]
            .iter()
            .map(String::as_str);
        match positions.get(&(url, &asset.method, &asset.data)) {
            Some(position) => {
                let known = &mut stored[*position].1;
                for alias in aliases {
                    if !known.contains(&alias) {
                        known.push(alias);
                    }
                }
            }
            None => {
                positions.insert((url, &asset.method, &asset.data), stored.len());
                stored.push((asset, aliases.collect()));
            }
        }
    }

    for (asset, aliases) in stored {
        let url = asset.urls.last().unwrap();
        markup.push_str(&format!(
            "<script type=\"text/plain\" data-monolith-url=\"{}\"{}{} data-monolith-method=\"{}\" \
             data-monolith-status=\"{}\" data-monolith-type=\"{}\">{}</script>",
            escape_attribute(url),
            match aliases.is_empty() {
                true => String::new(),
                false => format!(
                    " data-monolith-aliases=\"{}\"",
                    escape_attribute(&aliases.join(" "))
                ),
            },
            cors_attributes(asset),
            escape_attribute(&asset.method),
            asset.status,
            escape_attribute(asset.content_type.split(';').next().unwrap_or("")),
            BASE64_STANDARD.encode(&asset.data),
        ));
    }

    if !imports.is_empty() {
        let modules = json!(imports).to_string().replace('<', "\\u003c");
        markup.push_str(&format!(
            "<script type=\"application/json\" data-monolith-modules>{}</script>",
            modules
        ));
    }

    markup.push_str(&format!(
        "<script>const CAPTURED_AT={},PAGE_URL={};{}</script>",
        captured_at,
        to_script_literal(page_url),
        EMULATOR,
    ));

    markup
}

/// Puts given markup at the very start of HEAD, it has to get ahead of the page's own scripts.
#[doc(hidden)]
pub fn insert_replay_markup(document: Vec<u8>, markup: &str) -> Vec<u8> {
    let Some(start) = document
        .windows(5)
        .position(|part| part.eq_ignore_ascii_case(b"<head"))
    else {
        return document;
    };
    let Some(end) = document[start..].iter().position(|byte| *byte == b'>') else {
        return document;
    };
    let insertion_point = start + end + 1;

    let mut result = Vec::with_capacity(document.len() + markup.len());
    result.extend_from_slice(&document[..insertion_point]);
    result.extend_from_slice(markup.as_bytes());
    result.extend_from_slice(&document[insertion_point..]);

    result
}

/// Script that got prepared to work from inside of a document.
#[doc(hidden)]
pub struct RewrittenScript {
    pub code: String,
    /// Full URLs of everything it's known to import
    pub imports: Vec<String>,
    pub is_module: bool,
}

/// Scripts inside of a document no longer know where they came from, which makes it impossible for
/// them to find what they import, and for web apps to tell what they're supposed to show.
/// This gives them full URLs to look for, and a stand-in for the location of the document that
/// remembers the original URL, and leaves every other part of the code as it was.
#[doc(hidden)]
pub fn rewrite_imports(source: &str, script_url: &Url) -> RewrittenScript {
    let bytes = source.as_bytes();
    let script_url_literal = to_script_literal(script_url.as_str());
    let mut code = String::with_capacity(source.len());
    let mut copied_until = 0;
    let mut imports = vec![];
    let mut is_module = false;
    let mut position = 0;
    // Last thing seen that wasn't whitespace, it tells regular expressions from division
    let mut previous: Option<u8> = None;
    let mut previous_word = "";
    // What's currently open, it tells objects from everything else that has commas in it
    let mut brackets: Vec<u8> = vec![];

    while position < bytes.len() {
        let byte = bytes[position];

        if byte.is_ascii_whitespace() {
            position += 1;
            continue;
        }
        if let Some(end) = skip_comment(bytes, position) {
            position = end;
            continue;
        }
        if byte == b'"' || byte == b'\'' {
            position = skip_string(bytes, position);
            (previous, previous_word) = (Some(byte), "");
            continue;
        }
        if byte == b'`' {
            position = skip_template(bytes, position);
            (previous, previous_word) = (Some(byte), "");
            continue;
        }
        if byte == b'/' && is_regular_expression_allowed(previous, previous_word) {
            position = skip_regular_expression(bytes, position);
            (previous, previous_word) = (Some(byte), "");
            continue;
        }
        if !is_word_byte(byte) {
            match byte {
                b'(' | b'[' | b'{' => brackets.push(byte),
                b')' | b']' | b'}' => {
                    brackets.pop();
                }
                _ => {}
            }
            position += 1;
            (previous, previous_word) = (Some(byte), "");
            continue;
        }

        let start = position;
        while position < bytes.len() && is_word_byte(bytes[position]) {
            position += 1;
        }
        let word = &source[start..position];
        let is_property = previous == Some(b'.');
        let is_statement_start =
            matches!(previous, None | Some(b';' | b'}' | b'{' | b')')) || starts_line(bytes, start);

        if word == "import" && !is_property {
            let next = skip_whitespace(bytes, position);
            match bytes.get(next) {
                // import("...")
                Some(b'(') if !is_method_definition(bytes, next) => {
                    code.push_str(&source[copied_until..start]);
                    code.push_str(&format!("__monolithImport({},", script_url_literal));
                    position = next + 1;
                    copied_until = position;

                    let argument = skip_whitespace(bytes, position);
                    if matches!(bytes.get(argument), Some(b'"' | b'\'')) {
                        let end = skip_string(bytes, argument);
                        let specifier =
                            &source[argument + 1..end.saturating_sub(1).max(argument + 1)];
                        imports.extend(resolve_specifier(specifier, script_url));
                    }
                }
                // import.meta.url
                Some(b'.') => {
                    is_module = true;
                    let after_meta = skip_whitespace(bytes, next + 1);
                    if source[after_meta..].starts_with("meta") {
                        let after_dot = skip_whitespace(bytes, after_meta + 4);
                        let url_start = skip_whitespace(bytes, after_dot + 1);
                        if bytes.get(after_dot) == Some(&b'.')
                            && source[url_start..].starts_with("url")
                            && !bytes.get(url_start + 3).copied().is_some_and(is_word_byte)
                        {
                            code.push_str(&source[copied_until..start]);
                            code.push_str(&script_url_literal);
                            position = url_start + 3;
                            copied_until = position;
                        }
                    }
                }
                // import "..."
                Some(b'"' | b'\'') if is_statement_start => {
                    is_module = true;
                    position = rewrite_specifier(
                        source,
                        next,
                        script_url,
                        &mut code,
                        &mut copied_until,
                        &mut imports,
                    );
                }
                // import something from "..."
                Some(b'{' | b'*') | Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$')
                    if is_statement_start =>
                {
                    if let Some(specifier) = find_from_clause(source, next) {
                        is_module = true;
                        position = rewrite_specifier(
                            source,
                            specifier,
                            script_url,
                            &mut code,
                            &mut copied_until,
                            &mut imports,
                        );
                    }
                }
                _ => {}
            }
        } else if word == "export" && !is_property && is_statement_start {
            let next = skip_whitespace(bytes, position);
            if matches!(bytes.get(next), Some(b'{' | b'*')) {
                is_module = true;
                if let Some(specifier) = find_from_clause(source, next) {
                    position = rewrite_specifier(
                        source,
                        specifier,
                        script_url,
                        &mut code,
                        &mut copied_until,
                        &mut imports,
                    );
                }
            } else if matches!(bytes.get(next), Some(b'a'..=b'z')) {
                is_module = true;
            }
        }

        if word == "location" {
            let next = bytes.get(skip_whitespace(bytes, position));
            let is_object_entry =
                brackets.last() == Some(&b'{') && matches!(previous, Some(b'{' | b','));
            let is_method = matches!(previous_word, "async" | "get" | "set" | "static");

            if is_property {
                code.push_str(&source[copied_until..start]);
                code.push_str(LOCATION_REPLACEMENT);
                copied_until = position;
            } else if is_object_entry && matches!(next, Some(b':' | b'(')) {
                // Regular keys and methods have nothing to do with it
            } else if is_object_entry && matches!(next, Some(b',' | b'}' | b'=')) {
                // {location} has to keep its key
                code.push_str(&source[copied_until..start]);
                code.push_str(&format!("location:{}", LOCATION_REPLACEMENT));
                copied_until = position;
            } else if !is_method {
                code.push_str(&source[copied_until..start]);
                code.push_str(LOCATION_REPLACEMENT);
                copied_until = position;
            }
        }

        previous = Some(b'a');
        previous_word = word;
    }

    code.push_str(&source[copied_until..]);

    RewrittenScript {
        code,
        imports,
        is_module,
    }
}

/// Replaces the string that's found at given position, and tells where it ended.
fn rewrite_specifier(
    source: &str,
    start: usize,
    script_url: &Url,
    code: &mut String,
    copied_until: &mut usize,
    imports: &mut Vec<String>,
) -> usize {
    let end = skip_string(source.as_bytes(), start);
    let specifier = &source[start + 1..end.saturating_sub(1).max(start + 1)];

    if let Some(url) = resolve_specifier(specifier, script_url) {
        code.push_str(&source[*copied_until..start]);
        code.push_str(&to_script_literal(&url));
        *copied_until = end;
        imports.push(url);
    }

    end
}

/// Names without a path are for the page's own import maps to figure out.
fn resolve_specifier(specifier: &str, script_url: &Url) -> Option<String> {
    let is_path =
        specifier.starts_with("./") || specifier.starts_with("../") || specifier.starts_with('/');

    if is_path || specifier.starts_with("http:") || specifier.starts_with("https:") {
        script_url.join(specifier).ok().map(|url| url.to_string())
    } else {
        None
    }
}

/// Classes and objects are free to have methods called "import", which is told by what comes after.
fn is_method_definition(bytes: &[u8], parenthesis: usize) -> bool {
    let mut depth = 0;
    let mut position = parenthesis;

    while position < bytes.len() {
        if let Some(end) = skip_comment(bytes, position) {
            position = end;
            continue;
        }
        match bytes[position] {
            b'"' | b'\'' => position = skip_string(bytes, position),
            b'`' => position = skip_template(bytes, position),
            b'(' => {
                depth += 1;
                position += 1;
            }
            b')' => {
                depth -= 1;
                position += 1;
                if depth == 0 {
                    return bytes.get(skip_whitespace(bytes, position)) == Some(&b'{');
                }
            }
            _ => position += 1,
        }
    }

    false
}

/// Looks for the string that comes after the word "from".
fn find_from_clause(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut position = start;

    while position < bytes.len() {
        match bytes[position] {
            b';' | b'(' | b'=' => return None,
            b'"' | b'\'' => {
                position = skip_string(bytes, position);
            }
            b'f' if source[position..].starts_with("from")
                && !is_word_byte(bytes[position - 1])
                && !bytes.get(position + 4).copied().is_some_and(is_word_byte) =>
            {
                let specifier = skip_whitespace(bytes, position + 4);
                return matches!(bytes.get(specifier), Some(b'"' | b'\'')).then_some(specifier);
            }
            _ => position += 1,
        }
    }

    None
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte >= 0x80
}

fn starts_line(bytes: &[u8], position: usize) -> bool {
    bytes[..position]
        .iter()
        .rev()
        .find(|byte| **byte != b' ' && **byte != b'\t')
        .is_none_or(|byte| *byte == b'\n' || *byte == b'\r')
}

fn skip_whitespace(bytes: &[u8], mut position: usize) -> usize {
    loop {
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        match skip_comment(bytes, position) {
            Some(end) => position = end,
            None => return position,
        }
    }
}

fn skip_comment(bytes: &[u8], position: usize) -> Option<usize> {
    match bytes.get(position..position + 2)? {
        b"//" => Some(
            bytes[position..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |end| position + end),
        ),
        b"/*" => Some(
            bytes[position + 2..]
                .windows(2)
                .position(|part| part == b"*/")
                .map_or(bytes.len(), |end| position + end + 4),
        ),
        _ => None,
    }
}

fn skip_string(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut position = start + 1;

    while position < bytes.len() {
        match bytes[position] {
            b'\\' => position += 2,
            byte if byte == quote => return position + 1,
            b'\n' => return position,
            _ => position += 1,
        }
    }

    bytes.len()
}

fn skip_template(bytes: &[u8], start: usize) -> usize {
    let mut position = start + 1;

    while position < bytes.len() {
        match bytes[position] {
            b'\\' => position += 2,
            b'`' => return position + 1,
            b'$' if bytes.get(position + 1) == Some(&b'{') => {
                position = skip_template_expression(bytes, position + 2);
            }
            _ => position += 1,
        }
    }

    bytes.len()
}

fn skip_template_expression(bytes: &[u8], start: usize) -> usize {
    let mut depth = 1;
    let mut position = start;

    while position < bytes.len() {
        match bytes[position] {
            b'"' | b'\'' => position = skip_string(bytes, position),
            b'`' => position = skip_template(bytes, position),
            b'{' => {
                depth += 1;
                position += 1;
            }
            b'}' => {
                depth -= 1;
                position += 1;
                if depth == 0 {
                    return position;
                }
            }
            _ => position += 1,
        }
    }

    bytes.len()
}

fn is_regular_expression_allowed(previous: Option<u8>, previous_word: &str) -> bool {
    if !previous_word.is_empty() {
        return matches!(
            previous_word,
            "await"
                | "case"
                | "delete"
                | "in"
                | "instanceof"
                | "of"
                | "return"
                | "throw"
                | "typeof"
                | "void"
                | "yield"
        );
    }

    !matches!(previous, Some(b')' | b']' | b'}' | b'"' | b'\'' | b'`'))
}

fn skip_regular_expression(bytes: &[u8], start: usize) -> usize {
    let mut is_in_class = false;
    let mut position = start + 1;

    while position < bytes.len() {
        match bytes[position] {
            b'\\' => position += 2,
            b'[' => {
                is_in_class = true;
                position += 1;
            }
            b']' => {
                is_in_class = false;
                position += 1;
            }
            b'/' if !is_in_class => return position + 1,
            // That wasn't a regular expression after all
            b'\n' => return start + 1,
            _ => position += 1,
        }
    }

    start + 1
}

/// Workers load scripts of their own, those don't come with a destination that says "script".
pub(crate) fn is_script(asset: &CapturedAsset) -> bool {
    asset.destination == "script"
        || (matches!(
            asset.destination.as_str(),
            "" | "serviceworker" | "sharedworker" | "worker"
        ) && (asset.content_type.contains("javascript")
            || asset.content_type.contains("ecmascript")))
}

/// Prepares scripts to work from inside of a document, and provides what modules can import:
/// URL, media type, and contents encoded using base64.
#[doc(hidden)]
///
/// Scripts that don't say how they're encoded get the document's encoding, the way classic scripts
/// do, unless they're UTF-8: modules always are.
pub fn rewrite_scripts(
    assets: &mut [CapturedAsset],
    document_encoding: Option<&'static Encoding>,
) -> Vec<(String, String, String)> {
    let mut imported: HashSet<String> = HashSet::new();
    let mut modules = vec![false; assets.len()];

    for (index, asset) in assets.iter_mut().enumerate() {
        let Some(url) = asset.urls.last().and_then(|url| Url::parse(url).ok()) else {
            continue;
        };

        if is_script(asset) {
            let text =
                decode_text_with_fallback(&asset.data, &asset.content_type, document_encoding);
            // Source maps don't come along (see source_maps.rs)
            let script = rewrite_imports(remove_source_map_reference_from_text(&text), &url);
            store_as_utf8(asset, script.code);
            // Scripts are free to be modules without using any of what makes them look like ones
            modules[index] = true;
            imported.extend(script.imports);
        }
    }

    let mut imports: Vec<(String, String, String)> = vec![];
    let mut mapped: HashSet<String> = HashSet::new();
    for (index, asset) in assets.iter().enumerate() {
        // Modules other origins didn't allow pages to use didn't get to run
        if is_blocked(asset) {
            continue;
        }
        // Not everything that gets imported is written as a module
        if modules[index] || asset.urls.iter().any(|url| imported.contains(url)) {
            let media_type = match is_script(asset) {
                true => "text/javascript",
                false => asset.content_type.split(';').next().unwrap_or(""),
            };
            let body = BASE64_STANDARD.encode(&asset.data);
            for url in &asset.urls {
                if mapped.insert(url.clone()) {
                    imports.push((url.clone(), media_type.to_string(), body.clone()));
                }
            }
        }
    }

    imports
}

/// Whether what has been captured was kept from the page by CORS (see cors.rs).
fn is_blocked(asset: &CapturedAsset) -> bool {
    let Some(url) = asset.urls.last().and_then(|url| Url::parse(url).ok()) else {
        return false;
    };

    !asset.http.is_resource
        && was_blocked(
            &url,
            &asset.http.request_headers,
            &asset.http.response_headers,
        )
}

/// What other origins have said about letting pages use the response, for the replay emulator
/// to go by the way browsers do; nothing for what hasn't been received over HTTP.
fn cors_attributes(asset: &CapturedAsset) -> String {
    if asset.http.is_resource {
        return String::new();
    }
    let values = |name: &str| -> Vec<&str> {
        asset
            .http
            .response_headers
            .iter()
            .filter(|(header, _)| header.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
            .collect()
    };
    // More than one isn't allowed, those can't match anything
    let origin = values("access-control-allow-origin").join(", ");
    let credentials = values("access-control-allow-credentials").join(", ");

    format!(
        " data-monolith-allow-origin=\"{}\" data-monolith-allow-credentials=\"{}\"",
        escape_attribute(&origin),
        escape_attribute(&credentials)
    )
}

/// Stylesheets that pages load on their own don't get to go through what Monolith does to the rest
/// of them, and whatever they point at can't be found once they're inside of a document.
#[doc(hidden)]
pub fn rewrite_stylesheets(
    assets: &mut [CapturedAsset],
    document_encoding: Option<&'static Encoding>,
) {
    let mut data_urls: HashMap<String, String> = HashMap::new();
    for asset in assets.iter().filter(|asset| {
        asset.is_reusable
            && matches!(asset.destination.as_str(), "font" | "image")
            // Fonts other origins didn't allow pages to use didn't get used
            && !is_blocked(asset)
    }) {
        let data_url = format!(
            "data:{};base64,{}",
            asset.content_type.split(';').next().unwrap_or(""),
            BASE64_STANDARD.encode(&asset.data)
        );
        for url in &asset.urls {
            data_urls.insert(url.clone(), data_url.clone());
        }
    }

    for asset in assets
        .iter_mut()
        .filter(|asset| asset.content_type.starts_with("text/css"))
    {
        if let Some(url) = asset.urls.last().and_then(|url| Url::parse(url).ok()) {
            // Stylesheets have rules of their own: @charset is there, and comes before guessing
            let charset = parse_media_type(&asset.content_type).1.unwrap_or_default();
            let stylesheet = decode_stylesheet(&asset.data, &charset, document_encoding).0;
            let stylesheet = embed_stylesheet_urls(
                remove_source_map_reference_from_text(&stylesheet),
                &url,
                &data_urls,
            );
            store_as_utf8(asset, stylesheet);
        }
    }
}

/// Replaces what's inside of url() with what it has been found to point at.
#[doc(hidden)]
pub fn embed_stylesheet_urls(
    stylesheet: &str,
    stylesheet_url: &Url,
    data_urls: &HashMap<String, String>,
) -> String {
    let mut result = String::with_capacity(stylesheet.len());
    let mut remainder = stylesheet;

    while let Some(start) = remainder.find("url(") {
        let (before, after) = remainder.split_at(start + 4);
        result.push_str(before);
        remainder = after;

        let Some(end) = remainder.find(')') else {
            break;
        };
        let value = remainder[..end].trim().trim_matches(['"', '\'']);
        let found = stylesheet_url
            .join(value)
            .ok()
            .map(|mut url| {
                url.set_fragment(None);
                url
            })
            .and_then(|url| data_urls.get(url.as_str()));

        if let Some(data_url) = found.filter(|_| !value.starts_with("data:")) {
            result.push('"');
            result.push_str(data_url);
            result.push('"');
            remainder = &remainder[end..];
        }
    }
    result.push_str(remainder);

    result
}

/// Content type that says what text has been turned into.
pub(crate) fn with_utf8_charset(content_type: &str) -> String {
    let mut parts = content_type.split(';');
    let mut result = parts.next().unwrap_or("").trim().to_string();
    for parameter in parts
        .map(str::trim)
        .filter(|parameter| !parameter.is_empty())
    {
        let name = parameter.split('=').next().unwrap_or("").trim();
        if !name.eq_ignore_ascii_case("charset") {
            result.push_str("; ");
            result.push_str(parameter);
        }
    }
    result.push_str("; charset=utf-8");

    result
}

/// Makes headers agree with the content type, there's one of them in there at most.
pub(crate) fn set_content_type(headers: &mut Vec<(String, String)>, content_type: &str) {
    headers.retain(|(name, _)| !name.eq_ignore_ascii_case("content-type"));
    headers.push(("Content-Type".to_string(), content_type.to_string()));
}

/// Text, decoded the way its content type says it's encoded.
pub(crate) fn decode_text(data: &[u8], content_type: &str) -> String {
    decode_text_with_fallback(data, content_type, None)
}

/// Text that says nothing about how it's encoded is UTF-8 if it can be, and in the fallback
/// encoding otherwise.
pub(crate) fn decode_text_with_fallback(
    data: &[u8],
    content_type: &str,
    fallback: Option<&'static Encoding>,
) -> String {
    let (_, charset, _) = parse_media_type(content_type);
    // Byte order marks beat whatever's been declared
    let encoding = Encoding::for_bom(data)
        .map(|(encoding, _)| encoding)
        .or_else(|| charset.and_then(|charset| Encoding::for_label(charset.as_bytes())));

    match (encoding, std::str::from_utf8(data), fallback) {
        (Some(encoding), _, _) => encoding.decode_with_bom_removal(data).0.into_owned(),
        (None, Ok(text), _) => text.to_string(),
        (None, Err(_), Some(fallback)) => fallback.decode_without_bom_handling(data).0.into_owned(),
        (None, Err(_), None) => String::from_utf8_lossy(data).into_owned(),
    }
}

/// Text that has been re-encoded as UTF-8 has to say so.
fn store_as_utf8(asset: &mut CapturedAsset, text: String) {
    asset.data = text.into_bytes();
    asset.content_type = with_utf8_charset(&asset.content_type);
    if asset
        .http
        .response_headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    {
        set_content_type(&mut asset.http.response_headers, &asset.content_type);
    }
}

fn without_fragment(url: &str) -> String {
    url.split('#').next().unwrap_or(url).to_string()
}

pub(crate) fn is_html(content_type: &str) -> bool {
    let media_type = content_type.split(';').next().unwrap_or("").trim();

    media_type.eq_ignore_ascii_case("text/html")
        || media_type.eq_ignore_ascii_case("application/xhtml+xml")
}

/// Archives don't tell what things were requested for, what they are is a good enough hint.
fn destination_by_content_type(content_type: &str) -> &'static str {
    let media_type = content_type.to_lowercase();

    if is_html(&media_type) {
        "iframe"
    } else if media_type.contains("javascript") || media_type.contains("ecmascript") {
        "script"
    } else if media_type.starts_with("text/css") {
        "style"
    } else if media_type.starts_with("image/") {
        "image"
    } else if media_type.starts_with("font/") || media_type.contains("font-") {
        "font"
    } else if media_type.starts_with("video/") || media_type.starts_with("audio/") {
        "video"
    } else {
        ""
    }
}

/// Puts a page together out of what an archive has stored: the page is either the one that was
/// asked for, or the first document in there.
pub fn create_page_from_responses(
    responses: Vec<ArchivedResponse>,
    wanted_url: Option<&str>,
) -> Result<CapturedPage, String> {
    // Where each URL has first redirected to, that's what's used for finding the page
    let mut redirects: HashMap<String, String> = HashMap::new();
    // Redirects that have led somewhere, waiting for the response that came from there: the same
    // URL may redirect somewhere else every time, they get paired up in the order they came in
    let mut pending: HashMap<String, VecDeque<Vec<Redirect>>> = HashMap::new();
    let mut kept: Vec<(ArchivedResponse, Vec<Redirect>)> = vec![];

    for response in responses {
        let key = without_fragment(&response.url);

        if (300..400).contains(&response.status) && response.status != 304 {
            let target = response
                .location
                .as_ref()
                .and_then(|location| Url::parse(&response.url).ok()?.join(location.trim()).ok())
                .map(|target| without_fragment(target.as_str()));
            if let Some(target) = target {
                let mut chain = pending
                    .get_mut(&key)
                    .and_then(VecDeque::pop_front)
                    .unwrap_or_default();
                chain.push(Redirect {
                    method: response.method.clone(),
                    request_headers: response.http.request_headers.clone(),
                    response_headers: response.http.response_headers.clone(),
                    status: response.status,
                    status_text: response.http.status_text.clone(),
                    url: response.url.clone(),
                });
                pending.entry(target.clone()).or_default().push_back(chain);
                redirects.entry(key).or_insert(target);
            }
        } else if response.status >= 200 && response.status != 304 {
            // Pages may ask for the same thing many times, and get something else every time
            let chain = pending
                .get_mut(&key)
                .and_then(VecDeque::pop_front)
                .unwrap_or_default();
            kept.push((response, chain));
        }
    }

    // Archives may have stored responses ahead of what has redirected to them
    for (target, chains) in pending {
        if let Some((_, chain)) = kept
            .iter_mut()
            .find(|(response, _)| without_fragment(&response.url) == target)
        {
            chain.extend(chains.into_iter().flatten());
        }
    }

    let final_url_of = |url: &str| {
        let mut url = without_fragment(url);
        let mut hops = 0;
        while let Some(target) = redirects.get(&url).filter(|_| hops < 32) {
            url = target.clone();
            hops += 1;
        }
        url
    };

    let root = match wanted_url {
        Some(wanted_url) => {
            let final_url = final_url_of(wanted_url);
            kept.iter()
                .position(|(response, _)| {
                    response.method == "GET"
                        && response.status == 200
                        && without_fragment(&response.url) == final_url
                })
                .ok_or(format!("archive has no page at {}", wanted_url))?
        }
        None => kept
            .iter()
            .position(|(response, _)| {
                response.method == "GET"
                    && response.status == 200
                    && is_html(&response.content_type)
            })
            .ok_or("archive has no pages in it")?,
    };

    let root_response = &kept[root].0;
    let url = root_response.url.clone();
    let html = decode_text(&root_response.data, &root_response.content_type);
    let content_type = root_response.content_type.clone();
    let assets = kept
        .into_iter()
        .enumerate()
        .map(|(index, (response, chain))| {
            let mut urls: Vec<String> = vec![];
            for redirect in &chain {
                if !urls.contains(&redirect.url) {
                    urls.push(redirect.url.clone());
                }
            }
            urls.retain(|url| *url != response.url);
            urls.push(response.url.clone());
            let mut http = response.http;
            http.redirects = chain;

            CapturedAsset {
                destination: match index == root {
                    true => "document",
                    false => destination_by_content_type(&response.content_type),
                }
                .to_string(),
                is_reusable: response.method == "GET" && response.status == 200,
                content_type: response.content_type,
                data: response.data,
                method: response.method,
                status: response.status,
                urls,
                http,
            }
        })
        .collect();

    Ok(CapturedPage {
        assets,
        content_type,
        html,
        adopted_style_marker: None,
        title: String::new(),
        url,
    })
}

fn exchange_time(time: i64) -> DateTime<Utc> {
    match time > 0 {
        true => Utc
            .timestamp_millis_opt(time)
            .single()
            .unwrap_or_else(Utc::now),
        false => Utc::now(),
    }
}

/// What it took to receive an asset, redirects included.
pub fn exchanges_of(asset: &CapturedAsset) -> Vec<Exchange> {
    let Some(url) = asset.urls.last() else {
        return vec![];
    };
    let date = exchange_time(asset.http.time);
    let mut exchanges: Vec<Exchange> = asset
        .http
        .redirects
        .iter()
        .map(|redirect| Exchange {
            date,
            http_version: asset.http.http_version.clone(),
            method: match redirect.method.is_empty() {
                true => asset.method.clone(),
                false => redirect.method.clone(),
            },
            request_headers: redirect.request_headers.clone(),
            response_headers: redirect.response_headers.clone(),
            status: Some(redirect.status as u16),
            status_text: redirect.status_text.clone(),
            url: redirect.url.clone(),
            ..Default::default()
        })
        .collect();

    if asset.http.is_resource {
        exchanges.push(Exchange {
            body: asset.data.clone(),
            content_type: asset.content_type.clone(),
            date,
            method: asset.method.clone(),
            url: url.clone(),
            ..Default::default()
        });
        return exchanges;
    }

    let mut response_headers = asset.http.response_headers.clone();
    let has_content_type = response_headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("content-type"));
    if !has_content_type && !asset.content_type.is_empty() {
        response_headers.push(("Content-Type".to_string(), asset.content_type.clone()));
    }

    exchanges.push(Exchange {
        body: asset.data.clone(),
        content_type: asset.content_type.clone(),
        date,
        http_version: asset.http.http_version.clone(),
        method: asset.method.clone(),
        request_headers: asset.http.request_headers.clone(),
        response_headers,
        status: Some(asset.status as u16),
        status_text: asset.http.status_text.clone(),
        url: url.clone(),
    });

    exchanges
}
