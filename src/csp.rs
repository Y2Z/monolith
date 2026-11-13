//! Content Security Policy, the way browsers do it (https://www.w3.org/TR/CSP3/): documents
//! say what they're allowed to load, and what code and styles of theirs are allowed to apply,
//! through response headers and META elements. Browsers don't load or apply what isn't allowed,
//! neither does Monolith.
//!
//! What gets saved is put together out of what was elsewhere (data URLs, inline code), which is
//! what policies of pages usually don't allow, so they get rewritten to allow exactly that and
//! nothing else, in their own terms: what was allowed to happen afterwards still is, what wasn't
//! still isn't. Restrictions Monolith adds of its own (-I, -j, and others) go into the same
//! policy.

use std::collections::BTreeSet;
use std::hash::{BuildHasher, Hasher, RandomState};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::url::Url;

/// What a request is for, when it comes to which directive governs it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Destination {
    Script,
    Style,
    Image,
    Font,
    Media,
    Frame,
    Object,
    Manifest,
    Worker,
}

/// What's being requested, by what.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub destination: Destination,
    /// Of the element that requests it
    pub nonce: Option<&'a str>,
    /// Of the element that requests it
    pub integrity: Option<&'a str>,
}

impl Request<'_> {
    pub fn to(destination: Destination) -> Request<'static> {
        Request {
            destination,
            nonce: None,
            integrity: None,
        }
    }
}

/// Code and styles that come with documents, rather than from elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Inline {
    /// SCRIPT elements
    Script,
    /// STYLE elements
    Style,
    /// Event handlers (onclick and the like)
    ScriptAttribute,
    /// STYLE attributes
    StyleAttribute,
}

/// Directives that govern given destination, the first one a policy has is the one that does.
fn directive_names(destination: Destination) -> &'static [&'static str] {
    match destination {
        Destination::Script => &["script-src-elem", "script-src", "default-src"],
        Destination::Style => &["style-src-elem", "style-src", "default-src"],
        Destination::Image => &["img-src", "default-src"],
        Destination::Font => &["font-src", "default-src"],
        Destination::Media => &["media-src", "default-src"],
        Destination::Frame => &["frame-src", "child-src", "default-src"],
        Destination::Object => &["object-src", "default-src"],
        Destination::Manifest => &["manifest-src", "default-src"],
        Destination::Worker => &["worker-src", "child-src", "script-src", "default-src"],
    }
}

fn inline_directive_names(inline: Inline) -> &'static [&'static str] {
    match inline {
        Inline::Script => directive_names(Destination::Script),
        Inline::Style => directive_names(Destination::Style),
        Inline::ScriptAttribute => &["script-src-attr", "script-src", "default-src"],
        Inline::StyleAttribute => &["style-src-attr", "style-src", "default-src"],
    }
}

/// Directives that are about what gets loaded (and what code runs), the ones that keep documents
/// from reaching out to anything.
const FETCH_DIRECTIVES: &[&str] = &[
    "default-src",
    "child-src",
    "connect-src",
    "fenced-frame-src",
    "font-src",
    "frame-src",
    "img-src",
    "manifest-src",
    "media-src",
    "object-src",
    "prefetch-src",
    "script-src",
    "script-src-attr",
    "script-src-elem",
    "style-src",
    "style-src-attr",
    "style-src-elem",
    "worker-src",
];

/// Directives that do nothing when delivered through META elements.
const HEADER_ONLY_DIRECTIVES: &[&str] = &["frame-ancestors", "report-to", "report-uri", "sandbox"];

#[derive(Clone, Debug, PartialEq)]
struct Directive {
    name: String,
    sources: Vec<String>,
}

/// One policy, everything it says has to be abided by.
#[derive(Clone, Debug, PartialEq)]
pub struct Policy {
    directives: Vec<Directive>,
}

impl Policy {
    /// Policy as written (https://www.w3.org/TR/CSP3/#parse-serialized-policy): directives that
    /// appear more than once count the first time.
    pub fn parse(serialized: &str) -> Policy {
        let mut directives: Vec<Directive> = vec![];
        for token in serialized.split(';') {
            let mut parts = token.split_ascii_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            if !name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
            {
                continue;
            }
            let name = name.to_ascii_lowercase();
            if directives.iter().any(|directive| directive.name == name) {
                continue;
            }
            directives.push(Directive {
                name,
                sources: parts.map(str::to_string).collect(),
            });
        }

        Policy { directives }
    }

    /// Policies of a Content-Security-Policy header (or of several, joined), they're separated by
    /// commas.
    pub fn parse_header(value: &str) -> Vec<Policy> {
        value
            .split(',')
            .map(Policy::parse)
            .filter(|policy| !policy.directives.is_empty())
            .collect()
    }

    /// Policy of a META element, which can't do everything a header can.
    pub fn parse_meta(content: &str) -> Option<Policy> {
        let mut policy = Policy::parse(content);
        policy
            .directives
            .retain(|directive| !HEADER_ONLY_DIRECTIVES.contains(&directive.name.as_str()));
        (!policy.directives.is_empty()).then_some(policy)
    }

    pub fn serialize(&self) -> String {
        self.directives
            .iter()
            .map(|directive| {
                std::iter::once(directive.name.as_str())
                    .chain(directive.sources.iter().map(String::as_str))
                    .collect::<Vec<&str>>()
                    .join(" ")
            })
            .map(|directive| directive + ";")
            .collect::<Vec<String>>()
            .join(" ")
    }

    fn get(&self, name: &str) -> Option<&Directive> {
        self.directives
            .iter()
            .find(|directive| directive.name == name)
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut Directive> {
        self.directives
            .iter_mut()
            .find(|directive| directive.name == name)
    }

    fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    fn effective(&self, names: &[&str]) -> Option<&Directive> {
        names.iter().find_map(|name| self.get(name))
    }

    /// Sets what given directive allows, replacing what it allowed before.
    fn set(&mut self, name: &str, sources: &[&str]) {
        let sources: Vec<String> = sources.iter().map(|source| source.to_string()).collect();
        match self.get_mut(name) {
            Some(directive) => directive.sources = sources,
            None => self.directives.push(Directive {
                name: name.to_string(),
                sources,
            }),
        }
    }

    fn remove(&mut self, name: &str) {
        self.directives.retain(|directive| directive.name != name);
    }

    /// The most specific of the given directives, made out of the one in effect if it isn't
    /// there yet; None if none is in effect (everything is allowed).
    fn specific(&mut self, names: &[&str]) -> Option<&mut Directive> {
        let sources = self.effective(names)?.sources.clone();
        if !self.has(names[0]) {
            self.directives.push(Directive {
                name: names[0].to_string(),
                sources,
            });
        }
        self.get_mut(names[0])
    }
}

/// Where a document is from, when it comes to what 'self' and sources without schemes mean.
#[derive(Clone, Debug, Default, PartialEq)]
struct Origin {
    scheme: String,
    host: Option<String>,
    port: Option<u16>,
}

impl Origin {
    fn of(url: &Url) -> Origin {
        Origin {
            scheme: url.scheme().to_ascii_lowercase(),
            host: url.host_str().map(str::to_ascii_lowercase),
            port: url.port_or_known_default(),
        }
    }

    fn is_http(&self) -> bool {
        matches!(self.scheme.as_str(), "http" | "https")
    }

    /// What 'self' stands for, written out (for http and https documents).
    fn serialized(&self) -> Option<String> {
        let host = self.host.as_ref().filter(|_| self.is_http())?;
        Some(match (self.scheme.as_str(), self.port) {
            ("http", Some(80)) | ("https", Some(443)) | (_, None) => {
                format!("{}://{}", self.scheme, host)
            }
            (scheme, Some(port)) => format!("{}://{}:{}", scheme, host, port),
        })
    }
}

/// Policies a document has to abide by: those it came with (headers), and those its META
/// elements have added so far.
#[derive(Clone, Debug, Default)]
pub struct Policies {
    list: Vec<Policy>,
    origin: Origin,
    /// What the document is allowed to do in its sandbox, if it's in one (headers only)
    sandbox: Option<Vec<String>>,
}

impl Policies {
    /// Policies of a document at given URL, received with given Content-Security-Policy headers
    /// (all of them, joined by commas).
    pub fn of(url: &Url, header: Option<&str>) -> Policies {
        let list = header.map(Policy::parse_header).unwrap_or_default();
        let sandbox = list.iter().find_map(|policy| {
            policy.get("sandbox").map(|directive| {
                directive
                    .sources
                    .iter()
                    .map(|flag| flag.to_ascii_lowercase())
                    .collect()
            })
        });

        Policies {
            list,
            origin: Origin::of(url),
            sandbox,
        }
    }

    /// Same policies, for a document at another URL that has them too (srcdoc documents).
    pub fn inherited_by(&self, url: &Url) -> Policies {
        Policies {
            origin: Origin::of(url),
            ..self.clone()
        }
    }

    /// What a META element of the document's HEAD says, from then on.
    pub fn add_meta(&mut self, content: &str) {
        if let Some(policy) = Policy::parse_meta(content) {
            self.list.push(policy);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn list(&self) -> &[Policy] {
        &self.list
    }

    /// How many policies there are (those that come after the first n can be told apart).
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether what the document loads is up to policies at all, or where it loads it from.
    pub fn restricts_anything(&self) -> bool {
        self.list.iter().any(|policy| {
            policy.has("upgrade-insecure-requests")
                || policy
                    .directives
                    .iter()
                    .any(|directive| FETCH_DIRECTIVES.contains(&directive.name.as_str()))
        })
    }

    /// Whether the document can't run scripts: it's in a sandbox that doesn't allow them.
    pub fn forbids_scripts(&self) -> bool {
        self.sandbox
            .as_ref()
            .is_some_and(|flags| !flags.iter().any(|flag| flag == "allow-scripts"))
    }

    /// Whether the document has no origin of its own: it's in a sandbox that doesn't allow it.
    pub fn has_opaque_origin(&self) -> bool {
        self.sandbox
            .as_ref()
            .is_some_and(|flags| !flags.iter().any(|flag| flag == "allow-same-origin"))
    }

    /// Where requests for given URL go: insecure ones may have to become secure.
    pub fn upgrade(&self, url: &Url, destination: Destination) -> Url {
        let upgrades = self
            .list
            .iter()
            .any(|policy| policy.has("upgrade-insecure-requests"));
        // Frames get to be loaded insecurely, unless they're of the document's own host
        let is_other_host = destination == Destination::Frame
            && url.host_str().map(str::to_ascii_lowercase) != self.origin.host;
        if !upgrades || url.scheme() != "http" || is_other_host {
            return url.clone();
        }

        let mut upgraded = url.clone();
        let port = upgraded.port();
        if upgraded.set_scheme("https").is_ok() && port == Some(80) {
            let _ = upgraded.set_port(None);
        }
        upgraded
    }

    /// Whether given URL can be loaded for given destination: by an element with given nonce and
    /// integrity, after having been redirected or not.
    pub fn allows(
        &self,
        destination: Destination,
        url: &Url,
        nonce: Option<&str>,
        integrity: Option<&str>,
        is_redirected: bool,
    ) -> bool {
        self.list.iter().all(|policy| {
            let Some(directive) = policy.effective(directive_names(destination)) else {
                return true;
            };
            let sources = &directive.sources;
            let has_nonce = |nonce: Option<&str>| {
                nonce.is_some_and(|nonce| {
                    !nonce.is_empty()
                        && sources.iter().any(|source| nonce_of(source) == Some(nonce))
                })
            };

            match destination {
                Destination::Script => {
                    if has_nonce(nonce) || integrity_matches(sources, integrity) {
                        return true;
                    }
                    // What's in the document has to be named, by nonce or by hash
                    if has_keyword(sources, "'strict-dynamic'") {
                        return false;
                    }
                }
                Destination::Style if has_nonce(nonce) => return true,
                _ => {}
            }

            sources
                .iter()
                .any(|source| matches_source(source, url, &self.origin, is_redirected))
        })
    }

    /// Whether code or styles that come with the document get to run or apply: by an element
    /// with given nonce, or as attributes.
    pub fn allows_inline(&self, inline: Inline, nonce: Option<&str>, content: &str) -> bool {
        self.list.iter().all(|policy| {
            let Some(directive) = policy.effective(inline_directive_names(inline)) else {
                return true;
            };
            allows_inline(&directive.sources, inline, nonce, content)
        })
    }

    /// Whether given BASE element URL can be used.
    pub fn allows_base(&self, url: &Url) -> bool {
        self.list.iter().all(|policy| {
            policy.get("base-uri").is_none_or(|directive| {
                directive
                    .sources
                    .iter()
                    .any(|source| matches_source(source, url, &self.origin, false))
            })
        })
    }

    /// Whether what gets loaded for given destination is up to policies at all.
    pub fn restricts(&self, destination: Destination) -> bool {
        self.list
            .iter()
            .any(|policy| policy.effective(directive_names(destination)).is_some())
    }

    /// The policies, rewritten to work for the saved document (see the top of this file).
    pub fn rewritten(&self, saved: &Saved) -> Vec<Policy> {
        self.rewritten_from(0, saved)
    }

    /// Same as rewritten(), for policies after the first n (the rest is someone else's).
    pub fn rewritten_from(&self, n: usize, saved: &Saved) -> Vec<Policy> {
        self.list
            .iter()
            .skip(n)
            .map(|policy| rewrite(policy, &self.origin, saved))
            .collect()
    }
}

/// What a saved document (along with the documents of its frames, they get its policies too)
/// is made of, that policies have to allow.
#[derive(Clone, Debug, Default)]
pub struct Saved {
    /// What its SCRIPT, STYLE, and stylesheet LINK elements are named by
    pub nonce: String,
    /// Nonces policies of saved documents have
    pub nonces: BTreeSet<String>,
    /// Whether the nonce elements get named by is set (it's the first one of the page, if it has
    /// any)
    pub has_nonce: bool,
    /// What has elements named by nonce, scripts or styles
    pub named: BTreeSet<Destination>,
    /// What has been put into data URLs (and blob URLs), by destination
    pub data: BTreeSet<Destination>,
    pub blob: BTreeSet<Destination>,
    /// Whether stylesheets import others (out of data URLs, those can't be named by nonce)
    pub has_imports: bool,
    /// Hashes of what's in SCRIPT and STYLE elements there are (named by nonce, see
    /// Restrictions::apply())
    pub script_hashes: BTreeSet<String>,
    pub style_hashes: BTreeSet<String>,
    /// Hashes of event handlers and STYLE attributes there are
    pub script_attributes: BTreeSet<String>,
    pub style_attributes: BTreeSet<String>,
}

impl Saved {
    pub fn new() -> Saved {
        // Nonces are for nobody to guess
        let mut bytes: Vec<u8> = vec![];
        for _ in 0..2 {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(0);
            bytes.extend(hasher.finish().to_le_bytes());
        }

        Saved {
            nonce: BASE64_STANDARD.encode(bytes),
            ..Default::default()
        }
    }

    /// Keeps nonces of policies of a document that's getting saved, the first one is what
    /// elements get named by (scripts of the page may be expecting it).
    pub fn add_nonces(&mut self, policies: &[Policy]) {
        for policy in policies {
            for directive in &policy.directives {
                for value in directive
                    .sources
                    .iter()
                    .filter_map(|source| nonce_of(source))
                {
                    if !self.has_nonce {
                        self.nonce = value.to_string();
                        self.has_nonce = true;
                    }
                    self.nonces.insert(value.to_string());
                }
            }
        }
        // Elements are named from the start
        self.has_nonce = true;
    }

    /// Keeps the hash of code or styles that are in the saved document.
    pub fn add_inline(&mut self, inline: Inline, content: &str) {
        let hash = format!(
            "'sha256-{}'",
            BASE64_STANDARD.encode(Sha256::digest(content.as_bytes()))
        );
        match inline {
            Inline::Script => self.script_hashes.insert(hash),
            Inline::Style => self.style_hashes.insert(hash),
            Inline::ScriptAttribute => self.script_attributes.insert(hash),
            Inline::StyleAttribute => self.style_attributes.insert(hash),
        };
    }
}

fn has_keyword(sources: &[String], keyword: &str) -> bool {
    sources
        .iter()
        .any(|source| source.eq_ignore_ascii_case(keyword))
}

fn nonce_of(source: &str) -> Option<&str> {
    let lowercase = source.to_ascii_lowercase();
    (lowercase.starts_with("'nonce-") && source.ends_with('\'') && source.len() > 8)
        .then(|| &source[7..source.len() - 1])
}

/// Algorithm and value of hash sources ('sha256-…').
fn hash_of(source: &str) -> Option<(&str, String)> {
    let inner = source.strip_prefix('\'')?.strip_suffix('\'')?;
    let (algorithm, value) = inner.split_once('-')?;
    let algorithm = match algorithm.to_ascii_lowercase().as_str() {
        "sha256" => "sha256",
        "sha384" => "sha384",
        "sha512" => "sha512",
        _ => return None,
    };
    // Both base64 and base64url are fine
    Some((algorithm, value.replace('-', "+").replace('_', "/")))
}

fn digest(algorithm: &str, data: &[u8]) -> String {
    match algorithm {
        "sha256" => BASE64_STANDARD.encode(Sha256::digest(data)),
        "sha384" => BASE64_STANDARD.encode(Sha384::digest(data)),
        _ => BASE64_STANDARD.encode(Sha512::digest(data)),
    }
}

/// Whether sources allow every bit of inline code there is: they don't name any.
fn allows_all_inline(sources: &[String], inline: Inline) -> bool {
    has_keyword(sources, "'unsafe-inline'")
        && !sources
            .iter()
            .any(|source| nonce_of(source).is_some() || hash_of(source).is_some())
        && !(matches!(inline, Inline::Script | Inline::ScriptAttribute)
            && has_keyword(sources, "'strict-dynamic'"))
}

/// https://www.w3.org/TR/CSP3/#match-element-to-source-list
fn allows_inline(sources: &[String], inline: Inline, nonce: Option<&str>, content: &str) -> bool {
    if allows_all_inline(sources, inline) {
        return true;
    }
    if matches!(inline, Inline::Script | Inline::Style)
        && let Some(nonce) = nonce.filter(|nonce| !nonce.is_empty())
        && sources.iter().any(|source| nonce_of(source) == Some(nonce))
    {
        return true;
    }
    if matches!(inline, Inline::Script | Inline::Style) || has_keyword(sources, "'unsafe-hashes'") {
        return sources
            .iter()
            .filter_map(|source| hash_of(source))
            .any(|(algorithm, value)| digest(algorithm, content.as_bytes()) == value);
    }

    false
}

/// Whether scripts get to be loaded because of what their integrity attributes say.
fn integrity_matches(sources: &[String], integrity: Option<&str>) -> bool {
    let hashes: Vec<(&str, String)> = sources
        .iter()
        .filter_map(|source| hash_of(source))
        .collect();
    let Some(integrity) = integrity.filter(|_| !hashes.is_empty()) else {
        return false;
    };
    let mut metadata = integrity.split_ascii_whitespace().peekable();
    metadata.peek().is_some()
        && metadata.all(|item| {
            let item = item.split('?').next().unwrap_or_default();
            item.split_once('-').is_some_and(|(algorithm, value)| {
                hashes.iter().any(|(hash_algorithm, hash_value)| {
                    hash_algorithm.eq_ignore_ascii_case(algorithm) && *hash_value == value
                })
            })
        })
}

/// https://www.w3.org/TR/CSP3/#match-schemes
fn scheme_matches(expression: &str, scheme: &str) -> bool {
    let expression = expression.to_ascii_lowercase();
    let scheme = scheme.to_ascii_lowercase();
    expression == scheme
        || (expression == "http" && scheme == "https")
        || (expression == "ws" && matches!(scheme.as_str(), "wss" | "http" | "https"))
        || (expression == "wss" && scheme == "https")
}

fn is_scheme_source(source: &str) -> bool {
    source.ends_with(':')
        && source.len() > 1
        && source.starts_with(|character: char| character.is_ascii_alphabetic())
        && source[..source.len() - 1].chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

/// Parts of host sources ([scheme://]host[:port][/path]).
struct HostSource<'a> {
    scheme: Option<&'a str>,
    host: &'a str,
    port: Option<&'a str>,
    path: &'a str,
}

fn parse_host_source(source: &str) -> Option<HostSource<'_>> {
    if source.starts_with('\'') {
        return None;
    }
    let (scheme, rest) = match source.split_once("://") {
        Some((scheme, rest)) => (Some(scheme), rest),
        None => (None, source),
    };
    let host_end = rest.find([':', '/']).unwrap_or(rest.len());
    let (host, rest) = rest.split_at(host_end);
    if host.is_empty() {
        return None;
    }
    let (port, path) = match rest.strip_prefix(':') {
        Some(rest) => {
            let port_end = rest.find('/').unwrap_or(rest.len());
            (Some(&rest[..port_end]), &rest[port_end..])
        }
        None => (None, rest),
    };

    Some(HostSource {
        scheme,
        host,
        port,
        path,
    })
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}

/// https://www.w3.org/TR/CSP3/#match-url-to-source-expression
fn matches_source(source: &str, url: &Url, origin: &Origin, is_redirected: bool) -> bool {
    let scheme = url.scheme().to_ascii_lowercase();

    if source == "*" {
        return matches!(scheme.as_str(), "http" | "https") || scheme == origin.scheme;
    }

    if source.eq_ignore_ascii_case("'self'") {
        // Files are of the same origin as each other, as far as this goes
        if origin.scheme == "file" {
            return scheme == "file";
        }
        let host = url.host_str().map(str::to_ascii_lowercase);
        if origin.host.is_none() || origin.host != host {
            return false;
        }
        let port = url.port_or_known_default();
        if origin.scheme == scheme && origin.port == port {
            return true;
        }
        let ports_are_fine = origin.port == port
            || (origin.port == default_port(&origin.scheme) && port == default_port(&scheme));
        return ports_are_fine
            && (matches!(scheme.as_str(), "https" | "wss")
                || (origin.scheme == "http" && matches!(scheme.as_str(), "http" | "ws")));
    }

    if is_scheme_source(source) {
        return scheme_matches(&source[..source.len() - 1], &scheme);
    }

    let Some(expression) = parse_host_source(source) else {
        return false;
    };
    let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
        return false;
    };

    match expression.scheme {
        Some(expression_scheme) if !scheme_matches(expression_scheme, &scheme) => return false,
        None if !scheme_matches(&origin.scheme, &scheme) => return false,
        _ => {}
    }

    let pattern = expression.host.to_ascii_lowercase();
    let host_matches = match pattern.strip_prefix('*') {
        Some(suffix) => suffix.starts_with('.') && host.ends_with(suffix),
        None => pattern == host,
    };
    if !host_matches {
        return false;
    }

    let port = url.port_or_known_default();
    let port_matches = match expression.port {
        None => port == default_port(&scheme),
        Some("*") => true,
        Some(expression_port) => expression_port.parse::<u16>().ok().is_some_and(|expected| {
            Some(expected) == port || (expected == 80 && port == Some(443))
        }),
    };
    if !port_matches {
        return false;
    }

    // Where requests got redirected to doesn't have to be what's in the path
    if is_redirected || expression.path.is_empty() {
        return true;
    }
    let path = decode_path(url.path());
    let expected = decode_path(expression.path);
    if expected.ends_with('/') {
        path.starts_with(&expected)
    } else {
        path == expected
    }
}

fn decode_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let hex = |byte: u8| (byte as char).to_digit(16);
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2]))
        {
            decoded.push((high * 16 + low) as u8);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Whether a source can let anything reach out to the network.
fn reaches_network(source: &str) -> bool {
    if source == "*" || source.eq_ignore_ascii_case("'self'") {
        return true;
    }
    if is_scheme_source(source) {
        return !matches!(
            source.to_ascii_lowercase().as_str(),
            "data:" | "blob:" | "filesystem:"
        );
    }
    parse_host_source(source).is_some()
}

fn rewrite(policy: &Policy, origin: &Origin, saved: &Saved) -> Policy {
    let mut policy = policy.clone();
    policy
        .directives
        .retain(|directive| !HEADER_ONLY_DIRECTIVES.contains(&directive.name.as_str()));

    // The saved document isn't where the page was, what was relative to it has to be spelled out
    let nonce = format!("'nonce-{}'", saved.nonce);
    for directive in policy.directives.iter_mut() {
        let mut sources: Vec<String> = vec![];
        for source in &directive.sources {
            let source = if source.eq_ignore_ascii_case("'self'") {
                origin.serialized().unwrap_or_else(|| source.clone())
            } else if origin.is_http()
                && source != "*"
                && parse_host_source(source).is_some_and(|host| host.scheme.is_none())
            {
                format!("{}://{}", origin.scheme, source)
            } else {
                source.clone()
            };
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        // Scripts of the page may have nonces of their own written in them, those stay; elements
        // are named by the one nonce that everything allows (frames have their own nonces, and
        // nonces of what they're in, see the top of this file)
        if sources.iter().any(|source| nonce_of(source).is_some()) {
            for value in saved.nonces.iter().chain(std::iter::once(&saved.nonce)) {
                let source = format!("'nonce-{}'", value);
                if !sources.contains(&source) {
                    sources.push(source);
                }
            }
        }
        directive.sources = sources;
    }

    // Elements get to be what they were, named by nonce
    for (inline, destination) in [
        (Inline::Script, Destination::Script),
        (Inline::Style, Destination::Style),
    ] {
        let names = inline_directive_names(inline);
        let Some(effective) = policy.effective(names) else {
            continue;
        };
        // What allows all inline code would stop doing it if it named some
        let allows_all = allows_all_inline(&effective.sources, inline);
        let needs_nonce = !allows_all
            && saved.named.contains(&destination)
            && !effective.sources.contains(&nonce);
        // Elements in data URLs that aren't named, and what isn't an element (@import)
        let has_imports = destination == Destination::Style && saved.has_imports;
        let needs_data = (has_imports || (allows_all && saved.data.contains(&destination)))
            && !allows_scheme(&effective.sources, "data:");
        let needs_blob =
            saved.blob.contains(&destination) && !allows_scheme(&effective.sources, "blob:");
        if !(needs_nonce || needs_data || needs_blob) {
            continue;
        }

        let directive = policy.specific(names).unwrap();
        directive.sources.retain(|source| source != "'none'");
        if needs_nonce {
            directive.sources.push(nonce.clone());
        }
        if needs_data {
            directive.sources.push("data:".to_string());
        }
        if needs_blob {
            directive.sources.push("blob:".to_string());
        }
    }

    // Attributes get to be what they were, named by hash
    for (inline, hashes) in [
        (Inline::ScriptAttribute, &saved.script_attributes),
        (Inline::StyleAttribute, &saved.style_attributes),
    ] {
        let names = inline_directive_names(inline);
        let Some(effective) = policy.effective(names) else {
            continue;
        };
        if hashes.is_empty() || allows_all_inline(&effective.sources, inline) {
            continue;
        }
        let directive = policy.specific(names).unwrap();
        directive.sources.retain(|source| source != "'none'");
        for source in std::iter::once(&"'unsafe-hashes'".to_string()).chain(hashes.iter()) {
            if !directive.sources.contains(source) {
                directive.sources.push(source.clone());
            }
        }
    }

    // Everything else is in data URLs
    for destination in [
        Destination::Image,
        Destination::Font,
        Destination::Media,
        Destination::Frame,
        Destination::Object,
        Destination::Manifest,
        Destination::Worker,
    ] {
        let names = directive_names(destination);
        let Some(effective) = policy.effective(names) else {
            continue;
        };
        let needs_data =
            saved.data.contains(&destination) && !allows_scheme(&effective.sources, "data:");
        let needs_blob =
            saved.blob.contains(&destination) && !allows_scheme(&effective.sources, "blob:");
        if !(needs_data || needs_blob) {
            continue;
        }
        let directive = policy.specific(names).unwrap();
        directive.sources.retain(|source| source != "'none'");
        if needs_data {
            directive.sources.push("data:".to_string());
        }
        if needs_blob {
            directive.sources.push("blob:".to_string());
        }
    }

    policy
}

/// Whether sources allow URLs of given scheme, whatever they are.
fn allows_scheme(sources: &[String], scheme: &str) -> bool {
    let url = Url::parse(&format!("{},", scheme)).unwrap();
    sources
        .iter()
        .any(|source| matches_source(source, &url, &Origin::default(), false))
}

/// What Monolith keeps saved documents from doing, on top of what their policies do.
#[derive(Clone, Debug, Default)]
pub struct Restrictions {
    /// Nothing gets to reach out to the network
    pub isolate: bool,
    /// Blob URLs are fine (documents that replay responses make modules out of them)
    pub allows_blobs: bool,
    pub no_scripts: bool,
    pub no_styles: bool,
    pub no_fonts: bool,
    pub no_images: bool,
    pub no_frames: bool,
    pub no_objects: bool,
    /// The document gets a base URL of its own (-b)
    pub custom_base: bool,
}

impl Restrictions {
    /// The restrictions, as a policy of their own (for documents that have none).
    pub fn as_policy(&self) -> Option<Policy> {
        let mut policy = Policy { directives: vec![] };
        self.apply(&mut policy, &Saved::default());
        (!policy.directives.is_empty()).then_some(policy)
    }

    /// Makes given policy keep given saved document from doing what it isn't to be doing.
    pub fn apply(&self, policy: &mut Policy, saved: &Saved) {
        if self.isolate {
            for directive in policy.directives.iter_mut() {
                if !FETCH_DIRECTIVES.contains(&directive.name.as_str()) {
                    continue;
                }
                // Elements named by nonce get to load from anywhere, so do scripts that come
                // from those ('strict-dynamic'), elements get named by hash instead
                let is_named = directive.sources.iter().any(|source| {
                    nonce_of(source).is_some() || source.eq_ignore_ascii_case("'strict-dynamic'")
                });
                directive.sources.retain(|source| {
                    !reaches_network(source)
                        && nonce_of(source).is_none()
                        && !source.eq_ignore_ascii_case("'strict-dynamic'")
                });
                if is_named {
                    let name = directive.name.as_str();
                    let is_script = name.starts_with("script-src") || name == "default-src";
                    let is_style = name.starts_with("style-src") || name == "default-src";
                    let mut names: Vec<&String> = vec![];
                    if is_script && !name.ends_with("-attr") {
                        names.extend(&saved.script_hashes);
                    }
                    if is_style && !name.ends_with("-attr") {
                        names.extend(&saved.style_hashes);
                    }
                    for source in names {
                        if !directive.sources.contains(source) {
                            directive.sources.push(source.clone());
                        }
                    }
                    // What used to be named shouldn't get to be just anything now
                    let allows_all = directive
                        .sources
                        .iter()
                        .all(|source| hash_of(source).is_none());
                    if allows_all {
                        directive
                            .sources
                            .retain(|source| !source.eq_ignore_ascii_case("'unsafe-inline'"));
                    }
                    // Elements in data URLs can't be named by hash
                    let needs_data = (is_script && saved.data.contains(&Destination::Script))
                        || (is_style && saved.data.contains(&Destination::Style));
                    if needs_data && !allows_scheme(&directive.sources, "data:") {
                        directive.sources.push("data:".to_string());
                    }
                }
                if directive.sources.is_empty() {
                    directive.sources.push("'none'".to_string());
                }
            }
            if !policy.has("default-src") {
                let mut sources = vec!["'unsafe-eval'", "'unsafe-inline'", "data:"];
                if self.allows_blobs {
                    sources.push("blob:");
                }
                policy.set("default-src", &sources);
            }
        }
        if self.no_styles {
            policy.remove("style-src-elem");
            policy.remove("style-src-attr");
            policy.set("style-src", &["'none'"]);
        }
        if self.no_fonts {
            policy.set("font-src", &["'none'"]);
        }
        if self.no_frames {
            policy.set("frame-src", &["'none'"]);
            policy.set("child-src", &["'none'"]);
        }
        if self.no_scripts {
            policy.remove("script-src-elem");
            policy.remove("script-src-attr");
            policy.set("script-src", &["'none'"]);
        }
        if self.no_scripts || self.no_objects {
            policy.set("object-src", &["'none'"]);
        }
        if self.no_images {
            // Data URLs are what images get replaced with
            policy.set("img-src", &["data:"]);
        }
        if self.custom_base {
            // What's been asked for is what it gets
            policy.remove("base-uri");
        }
    }
}
