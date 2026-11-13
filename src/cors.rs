//! Cross-Origin Resource Sharing, the way browsers do it (https://fetch.spec.whatwg.org/#http-cors-protocol):
//! some requests (fonts, module scripts, anything with a `crossorigin` attribute) get made in CORS
//! mode, and what other origins send in response can only be used if they allow it. Browsers
//! drop what fails, so does Monolith.
//!
//! Pages that come from files get treated the way Firefox treats them
//! (security.fileuri.strict_origin_policy): files in the same directory as the page, or in
//! directories within it, are the page's own, anything else from other files isn't allowed, and
//! what comes from websites has to allow pages with no origin of their own ("null").

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::url::Url;

/// Whose page it is, when it comes to what other origins allow.
#[derive(Clone, Debug, PartialEq)]
pub enum PageOrigin {
    /// Scheme, host, and port of a website (https://example.com)
    Tuple(String),
    /// A file: what's in its directory (or within it) is the page's own
    File(Url),
    /// Documents with no origin of their own (data URLs), they allow nothing but "*"
    Opaque,
}

impl PageOrigin {
    /// Origin of the document at given URL, none for documents CORS doesn't apply to.
    pub fn of(url: &Url) -> Option<PageOrigin> {
        match url.scheme() {
            "http" | "https" => Some(PageOrigin::Tuple(url.origin().ascii_serialization())),
            "file" => url.join("./").ok().map(PageOrigin::File),
            "data" => Some(PageOrigin::Opaque),
            _ => None,
        }
    }

    /// What goes into Origin headers.
    pub fn serialized(&self) -> &str {
        match self {
            PageOrigin::Tuple(origin) => origin,
            PageOrigin::File(_) | PageOrigin::Opaque => "null",
        }
    }

    /// Whether given URL is of the same origin as the page.
    pub fn is_same_origin(&self, url: &Url) -> bool {
        match self {
            PageOrigin::Tuple(origin) => {
                matches!(url.scheme(), "http" | "https")
                    && url.origin().ascii_serialization() == *origin
            }
            // Files in the page's directory, or within it
            PageOrigin::File(directory) => {
                url.scheme() == "file" && url.path().starts_with(directory.path())
            }
            PageOrigin::Opaque => false,
        }
    }
}

/// How a request gets made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RequestMode {
    /// What isn't up to other origins to allow (images, stylesheets, classic scripts…)
    NoCors,
    /// Has to be allowed by other origins; with credentials (cookies, HTTP authentication)
    /// only for "use-credentials", otherwise just for the page's own origin
    Cors { with_credentials: bool },
}

impl RequestMode {
    /// Mode of elements with given `crossorigin` attribute value: missing means no CORS,
    /// anything but "use-credentials" means CORS without credentials (empty and invalid values
    /// are the same as "anonymous").
    pub fn of_attribute(crossorigin: Option<&str>) -> RequestMode {
        match crossorigin {
            None => RequestMode::NoCors,
            Some(value) => RequestMode::Cors {
                with_credentials: value.trim().eq_ignore_ascii_case("use-credentials"),
            },
        }
    }

    /// Mode of SCRIPT elements: modules are CORS without credentials, unless the attribute says
    /// otherwise.
    pub fn of_script(script_type: Option<&str>, crossorigin: Option<&str>) -> RequestMode {
        let is_module =
            script_type.is_some_and(|value| value.trim().eq_ignore_ascii_case("module"));
        match RequestMode::of_attribute(crossorigin) {
            RequestMode::NoCors if is_module => RequestMode::Cors {
                with_credentials: false,
            },
            mode => mode,
        }
    }

    /// Fonts are always CORS without credentials.
    pub const FONT: RequestMode = RequestMode::Cors {
        with_credentials: false,
    };
}

/// What a CORS request gets sent with.
#[derive(Clone, Debug)]
pub struct CorsRequest {
    pub origin: PageOrigin,
    pub with_credentials: bool,
}

impl CorsRequest {
    /// Whether cookies and HTTP authentication go along with a request to given URL: always to
    /// the page's own origin, to others only if asked for.
    pub fn sends_credentials_to(&self, url: &Url) -> bool {
        self.with_credentials || self.origin.is_same_origin(url)
    }

    /// Whether to say where the request comes from: not to the page's own origin.
    pub fn origin_header_for(&self, url: &Url) -> Option<&str> {
        (!self.origin.is_same_origin(url)).then(|| self.origin.serialized())
    }

    /// Whether what came from given URL, with given headers, may be used by the page
    /// (https://fetch.spec.whatwg.org/#cors-check).
    pub fn allows(&self, url: &Url, headers: &HeaderMap) -> bool {
        self.origin.is_same_origin(url)
            || is_allowed(self.origin.serialized(), self.with_credentials, headers)
    }

    /// Same as allows(), for headers that have been recorded (names in any case).
    pub fn allows_recorded(&self, url: &Url, headers: &[(String, String)]) -> bool {
        self.allows(url, &header_map(headers))
    }

    /// What it gets cached under, along with what's cached for the URL itself: what's allowed
    /// for some isn't for others, and what's sent with credentials isn't what's sent without.
    pub fn cache_key_suffix(&self) -> String {
        format!(
            " (CORS from {}{})",
            self.origin.serialized(),
            if self.with_credentials {
                ", with credentials"
            } else {
                ""
            }
        )
    }
}

/// Whether a response with given headers may be used by pages of given (serialized) origin
/// (https://fetch.spec.whatwg.org/#cors-check).
pub fn is_allowed(origin: &str, with_credentials: bool, headers: &HeaderMap) -> bool {
    let mut allowed = headers.get_all("access-control-allow-origin").iter();
    let (Some(allowed), None) = (allowed.next(), allowed.next()) else {
        // Missing, or more than one
        return false;
    };
    let Ok(allowed) = allowed.to_str() else {
        return false;
    };

    if !with_credentials {
        return allowed == "*" || allowed == origin;
    }

    // What comes with credentials has to name the origin, and say credentials are fine
    allowed == origin
        && headers
            .get("access-control-allow-credentials")
            .and_then(|value| value.to_str().ok())
            == Some("true")
}

/// Whether what a browser (or anything else that keeps records) has received from given URL was
/// kept from the page that asked for it: requests made in CORS mode say where they're from, and
/// what other origins send back has to allow that. Cookies going along mean credentials were asked
/// for.
pub fn was_blocked(
    url: &Url,
    request_headers: &[(String, String)],
    response_headers: &[(String, String)],
) -> bool {
    let header = |name: &str| {
        request_headers
            .iter()
            .find(|(header, _)| header.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
    };
    let Some(origin) = header("origin") else {
        return false;
    };
    // Browsers say where requests are from to their own origin too, sometimes
    if matches!(url.scheme(), "http" | "https") && url.origin().ascii_serialization() == origin {
        return false;
    }

    !is_allowed(
        origin,
        header("cookie").is_some(),
        &header_map(response_headers),
    )
}

/// Recorded headers (names in any case), the way responses have them.
fn header_map(headers: &[(String, String)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.trim().to_ascii_lowercase().as_bytes()),
            HeaderValue::from_str(value.trim()),
        ) {
            map.append(name, value);
        }
    }
    map
}
