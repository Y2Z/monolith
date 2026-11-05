//! Makes saved documents work without a network, by storing responses their scripts have received,
//! and a script that hands those responses back to them.

use std::collections::HashSet;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde_json::json;
use url::Url;

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
}

/// A page that was loaded by something other than Monolith, be that a browser, or a web view of an
/// app. Assets should be in the order they've been received in.
#[derive(Clone, Debug, Default)]
pub struct CapturedPage {
    pub assets: Vec<CapturedAsset>,
    pub content_type: String,
    /// What the page has turned into, for when it gets saved without JavaScript
    pub html: String,
    pub title: String,
    pub url: String,
}

const EMULATOR: &str = include_str!("replay.js");
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
    let mut stored: HashSet<(&String, &String, &Vec<u8>)> = HashSet::new();

    for asset in assets.iter().filter(|asset| is_worth_storing(asset)) {
        let Some(url) = asset.urls.last() else {
            continue;
        };
        // Pages tend to ask for the same things many times, and the import map has its own copy
        if !stored.insert((url, &asset.method, &asset.data))
            || imports
                .iter()
                .any(|(imported_url, _, _)| imported_url == url)
        {
            continue;
        }
        markup.push_str(&format!(
            "<script type=\"text/plain\" data-monolith-url=\"{}\" data-monolith-method=\"{}\" \
             data-monolith-status=\"{}\" data-monolith-type=\"{}\">{}</script>",
            escape_attribute(url),
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
pub fn rewrite_scripts(assets: &mut [CapturedAsset]) -> Vec<(String, String, String)> {
    let mut imported: HashSet<String> = HashSet::new();
    let mut modules = vec![false; assets.len()];

    for (index, asset) in assets.iter_mut().enumerate() {
        let Some(url) = asset.urls.last().and_then(|url| Url::parse(url).ok()) else {
            continue;
        };

        if is_script(asset) {
            let script = rewrite_imports(&String::from_utf8_lossy(&asset.data), &url);
            asset.data = script.code.into_bytes();
            // Scripts are free to be modules without using any of what makes them look like ones
            modules[index] = true;
            imported.extend(script.imports);
        }
    }

    let mut imports: Vec<(String, String, String)> = vec![];
    let mut mapped: HashSet<String> = HashSet::new();
    for (index, asset) in assets.iter().enumerate() {
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
