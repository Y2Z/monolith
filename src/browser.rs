//! Lets a real browser render the page before Monolith saves it.
//!
//! The browser runs from a temporary profile that gets deleted afterwards, and never gets to see
//! user's own profile, cookies, or anything else that belongs to them. Nothing here tries to hide
//! the fact that the browser is being automated either.

use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, BufReader, ErrorKind};
use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::Utc;
use serde_json::{Value, json};
use tempfile::{Builder, TempDir};
use tungstenite::client::{IntoClientRequest, client_with_config};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Error as WebSocketError, Message, WebSocket};
use url::Url;

use crate::cookies::Cookie;
use crate::core::{MonolithOptions, print_error_message, print_info_message};
use crate::net::is_domain_allowed;
use crate::replay::{
    CapturedAsset, CapturedPage, HttpDetails, Redirect, set_content_type, with_utf8_charset,
};
use crate::session::Session;

const BROWSER_COLOR_SCHEME_VARIABLE: &str = "MONOLITH_BROWSER_COLOR_SCHEME";
const BROWSER_DISABLE_JS_VARIABLE: &str = "MONOLITH_BROWSER_DISABLE_JS";
const BROWSER_PATH_VARIABLE: &str = "MONOLITH_BROWSER";
const BROWSER_SCROLL_LIMIT_VARIABLE: &str = "MONOLITH_BROWSER_SCROLL_LIMIT";
const BROWSER_VIEWPORT_VARIABLE: &str = "MONOLITH_BROWSER_VIEWPORT";
// What browsers start with on their own is tiny, pages only load what they think can be seen
const DEFAULT_VIEWPORT: (u32, u32) = (1920, 1080);
// How many screens to go through, some pages have no end
const DEFAULT_SCROLL_LIMIT: u32 = 30;
// Whatever comes into view gets requested quickly, this keeps long pages from taking forever
const SCROLL_QUIET_PERIOD: Duration = Duration::from_millis(400);
const SCROLL_STEP_TIMEOUT: Duration = Duration::from_secs(3);
// Firefox and browsers made out of it, the ones that come with WebDriver BiDi
const BROWSER_NAMES: [&str; 15] = [
    "firefox",
    "firefox-esr",
    "librewolf",
    "icecat",
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
    "brave-browser",
    "brave",
    "microsoft-edge",
    "microsoft-edge-stable",
    "msedge",
    "vivaldi",
    "opera",
];
// Anything that has these in its name speaks DevTools Protocol, instead of WebDriver BiDi
const DEVTOOLS_BROWSERS: [&str; 6] = ["brave", "chrome", "chromium", "edge", "opera", "vivaldi"];
// Keeps the browser from contacting anything other than what the page asks for
const DEVTOOLS_SWITCHES: [&str; 31] = [
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-breakpad",
    "--disable-client-side-phishing-detection",
    "--disable-component-extensions-with-background-pages",
    "--disable-component-update",
    "--disable-default-apps",
    "--disable-domain-reliability",
    "--disable-extensions",
    "--disable-features=MediaRouter,NetworkTimeServiceQuerying,OptimizationHints,PreconnectToSearch,Translate",
    "--disable-renderer-backgrounding",
    "--disable-search-engine-choice-screen",
    "--disable-sync",
    "--metrics-recording-only",
    "--no-default-browser-check",
    "--no-first-run",
    "--no-pings",
    "--no-service-autorun",
    "--password-store=basic",
    // The keychain of macOS is what cookies get encrypted with, and it makes browsers wait for
    // someone to answer its prompt, forever
    "--use-mock-keychain",
    // These browsers keep talking to their makers regardless of being told not to, so what they
    // talk to gets replaced with a port that nothing ever answers on
    "--apps-gallery-update-url=http://127.0.0.1:9",
    "--component-updater=url-source=http://127.0.0.1:9",
    "--gaia-url=http://127.0.0.1:9",
    "--gcm-checkin-url=http://127.0.0.1:9",
    "--gcm-mcs-endpoint=http://127.0.0.1:9",
    "--gcm-registration-url=http://127.0.0.1:9",
    "--google-apis-url=http://127.0.0.1:9",
    "--google-base-url=http://127.0.0.1:9/",
    "--lso-url=http://127.0.0.1:9",
    "--sync-url=http://127.0.0.1:9",
];
// Otherwise these browsers go and download dictionaries
const DEVTOOLS_PREFERENCES: &str = r#"{"browser":{"enable_spellchecking":false},"spellcheck":{"dictionaries":[],"dictionary":""}}"#;
// There's no way of telling these to listen for commands
const UNSUPPORTED_BROWSERS: [&str; 1] = ["safari"];
const MAX_LAUNCHER_SCRIPT_SIZE: u64 = 64 * 1024;
// Takes a while on busy machines, when nothing of the browser is in memory yet
const BROWSER_STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const BROWSER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
// How long the network has to stay silent for the page to be considered done
const NETWORK_QUIET_PERIOD: Duration = Duration::from_millis(1000);
// Pages that never stop talking to their servers get this much time after they've loaded
const NETWORK_QUIET_TIMEOUT: Duration = Duration::from_secs(15);
const NETWORK_POLL_INTERVAL: Duration = Duration::from_millis(100);
// How long to wait for the window browsers open on their own, before opening one
const WINDOW_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
// Plenty of challenges solve themselves, these don't need anyone's attention
const CHALLENGE_SELF_SOLVE_TIMEOUT: Duration = Duration::from_secs(8);
// How long a person gets to deal with a challenge
const CHALLENGE_TIMEOUT: Duration = Duration::from_secs(300);
const CHALLENGE_POLL_INTERVAL: Duration = Duration::from_millis(500);
// Used when no timeout is specified, same as what network requests use
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

// Keeps the browser from contacting anything other than what the page asks for
#[doc(hidden)]
pub const PROFILE_PREFERENCES: &str = r#"
user_pref("app.normandy.enabled", false);
user_pref("app.update.enabled", false);
user_pref("browser.aboutwelcome.enabled", false);
user_pref("browser.chrome.site_icons", false);
user_pref("browser.newtabpage.enabled", false);
user_pref("browser.safebrowsing.downloads.enabled", false);
user_pref("browser.safebrowsing.malware.enabled", false);
user_pref("browser.safebrowsing.phishing.enabled", false);
user_pref("browser.search.update", false);
user_pref("browser.shell.checkDefaultBrowser", false);
user_pref("browser.startup.page", 0);
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);
user_pref("extensions.addonAbuseReport.url", "http://127.0.0.1:9/");
user_pref("extensions.getAddons.browserMappings.url", "http://127.0.0.1:9/");
user_pref("extensions.getAddons.cache.enabled", false);
user_pref("extensions.getAddons.discovery.api_url", "http://127.0.0.1:9/");
user_pref("extensions.getAddons.get.url", "http://127.0.0.1:9/");
user_pref("extensions.getAddons.langpacks.url", "http://127.0.0.1:9/");
user_pref("extensions.systemAddon.update.enabled", false);
user_pref("extensions.systemAddon.update.url", "http://127.0.0.1:9/");
user_pref("extensions.update.autoUpdateDefault", false);
user_pref("extensions.update.background.url", "http://127.0.0.1:9/");
user_pref("extensions.update.enabled", false);
user_pref("extensions.update.url", "http://127.0.0.1:9/");
user_pref("network.captive-portal-service.enabled", false);
user_pref("network.connectivity-service.enabled", false);
user_pref("network.dns.disablePrefetch", true);
user_pref("network.http.speculative-parallel-limit", 0);
user_pref("network.predictor.enabled", false);
user_pref("network.prefetch-next", false);
user_pref("services.settings.server", "data:,#remote-settings-dummy/v1");
user_pref("toolkit.telemetry.enabled", false);
user_pref("toolkit.telemetry.unified", false);
"#;

// Only looks for what challenges are made of, plenty of regular pages load Cloudflare's scripts
const CHALLENGE_SCRIPT: &str = r##"!!(
    window._cf_chl_opt ||
    document.title === "Just a moment..." ||
    document.querySelector(
        "#challenge-form, #challenge-running, #cf-challenge-running, #px-captcha, " +
        "iframe[src*='geo.captcha-delivery.com'], " +
        "script[src*='/cdn-cgi/challenge-platform/'][src*='orchestrate']"
    )
)"##;

// Goes one screen down, using what actually scrolls: plenty of web apps keep the page itself still
const SCROLL_SCRIPT: &str = r#"(() => {
    const isScrollable = (element) =>
        element.scrollHeight > element.clientHeight + 100 &&
        ["auto", "scroll"].includes(getComputedStyle(element).overflowY);
    const target = (() => {
        const page = document.scrollingElement || document.documentElement;
        if (page.scrollHeight > page.clientHeight + 100) {
            return page;
        }
        const elements = Array.from(document.querySelectorAll("*")).filter(isScrollable);
        elements.sort((a, b) => b.scrollHeight - a.scrollHeight);
        return elements[0] || page;
    })();
    target.scrollTop += Math.floor(target.clientHeight * 0.9);
    return target.scrollTop + target.clientHeight >= target.scrollHeight - 2;
})()"#;

// Pages have to be saved the way they first appear
const SCROLL_TO_TOP_SCRIPT: &str = r#"(() => {
    for (const element of [document.scrollingElement, ...document.querySelectorAll("*")]) {
        if (element && element.scrollTop) {
            element.scrollTop = 0;
        }
    }
})()"#;

// Makes the document's current state a part of its HTML, then serializes it.
// Passwords stay out, that's the only thing here which could've been typed in by a script
// and yet shouldn't end up in a file.
// Nonces are hidden from the markup, they get to be in it again (policies of the page name
// elements by them, see csp.rs); STYLE elements made out of constructed stylesheets aren't up to
// policies, they get marked as such.
const SNAPSHOT_SCRIPT: &str = r#"(() => {
    const adoptedStyleMarker = Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) =>
        byte.toString(16).padStart(2, "0"),
    ).join("");
    const rules = (sheet) => {
        try {
            return Array.from(sheet.cssRules, (rule) => rule.cssText).join("\n");
        } catch (error) {
            return "";
        }
    };
    const freeze = (root) => {
        // CSS-in-JS libraries insert their rules directly, those never appear in the markup
        for (const style of root.querySelectorAll("style")) {
            if (style.sheet && !style.textContent.trim()) {
                style.textContent = rules(style.sheet);
                // What's been allowed in empty can't be checked again with what's in it now
                style.setAttribute("data-monolith-adopted", adoptedStyleMarker);
            }
        }
        for (const element of root.querySelectorAll("[nonce]")) {
            if (element.nonce) {
                element.setAttribute("nonce", element.nonce);
            }
        }
        for (const sheet of root.adoptedStyleSheets || []) {
            const style = document.createElement("style");
            style.textContent = rules(sheet);
            style.setAttribute("data-monolith-adopted", adoptedStyleMarker);
            (root === document ? document.head : root).appendChild(style);
        }
        for (const input of root.querySelectorAll("input")) {
            if (input.type === "checkbox" || input.type === "radio") {
                input.toggleAttribute("checked", input.checked);
            } else if (input.type !== "password" && input.type !== "file") {
                input.setAttribute("value", input.value);
            }
        }
        for (const textarea of root.querySelectorAll("textarea")) {
            textarea.textContent = textarea.value;
        }
        for (const option of root.querySelectorAll("option")) {
            option.toggleAttribute("selected", option.selected);
        }
        // Web components keep their contents out of the markup, this is what brings them back
        for (const element of root.querySelectorAll("*")) {
            if (element.shadowRoot) {
                freeze(element.shadowRoot);
                const template = document.createElement("template");
                template.setAttribute("shadowrootmode", element.shadowRoot.mode);
                template.innerHTML = element.shadowRoot.innerHTML;
                element.prepend(template);
            }
        }
    };
    freeze(document);
    const doctype = document.doctype
        ? new XMLSerializer().serializeToString(document.doctype)
        : "";
    return JSON.stringify({
        contentType: document.contentType,
        html: doctype + document.documentElement.outerHTML,
        adoptedStyleMarker,
        title: document.title,
        url: location.href,
    });
})()"#;

// Anything larger than this has to be retrieved again by Monolith
const MAX_CAPTURED_RESPONSE_SIZE: u64 = 200_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorScheme {
    Dark,
    Light,
}

/// Opens given URL in a browser that's about to cease to exist, and returns what the page became.
/// Everything the session is set up to refuse gets refused by the browser as well.
///
/// The browser stays out of sight, unless the page turns out to be a challenge that someone has to
/// deal with, in which case it gets opened again in a window.
pub fn render_page(url: &str, session: &Session) -> Result<CapturedPage, String> {
    let executable = find_browser()?;

    if let Some(page) = visit_page(&executable, url, session, false)? {
        return Ok(page);
    }

    if !has_display() {
        return Err("page is behind a challenge, which takes a display to solve".to_string());
    }
    if !session.options.silent {
        print_error_message("Waiting for the challenge to be solved in the browser window");
    }

    visit_page(&executable, url, session, true)?.ok_or("challenge was not solved".to_string())
}

/// Provides nothing if there's a challenge in the way that nobody is around to solve.
fn visit_page(
    executable: &Path,
    url: &str,
    session: &Session,
    is_visible: bool,
) -> Result<Option<CapturedPage>, String> {
    let options = &session.options;
    let mut browser = Browser::launch(executable, options, is_visible)?;
    let timeout = match options.timeout {
        0 => DEFAULT_TIMEOUT,
        seconds => Duration::from_secs(seconds),
    };
    let mut deadline = Instant::now() + timeout;

    let connection = browser.connection()?;
    connection.start_session(session.cookies.as_deref(), deadline)?;
    let context = connection.open_page(url, deadline)?;
    connection.wait_for_network_to_settle(deadline);

    let challenge_timeout = match is_visible {
        true => CHALLENGE_TIMEOUT,
        false => CHALLENGE_SELF_SOLVE_TIMEOUT,
    };
    if connection.is_showing_challenge(&context) {
        if !connection.wait_for_challenge_to_go_away(&context, challenge_timeout)? {
            return Ok(None);
        }
        deadline = Instant::now() + timeout;
        connection.wait_for_network_to_settle(deadline);
    }

    connection.scroll_through_page(&context, choose_scroll_limit(options)?, deadline);

    let mut page = connection.take_snapshot(&context, deadline)?;
    page.assets = connection.collect_assets();

    Ok(Some(page))
}

// Without one of these on Linux and BSD a window has nowhere to appear
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn has_display() -> bool {
    env::var_os("DISPLAY").is_some() || env::var_os("WAYLAND_DISPLAY").is_some()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn has_display() -> bool {
    true
}

#[doc(hidden)]
pub fn profile_preferences(
    options: &MonolithOptions,
    color_scheme: Option<ColorScheme>,
    is_js_disabled: bool,
) -> String {
    let mut preferences = PROFILE_PREFERENCES.to_string();

    if is_js_disabled {
        preferences.push_str("user_pref(\"javascript.enabled\", false);\n");
    }

    if let Some(user_agent) = &options.browser_user_agent {
        preferences.push_str(&format!(
            "user_pref(\"general.useragent.override\", {});\n",
            json!(user_agent)
        ));
    }
    if let Some(color_scheme) = color_scheme {
        let is_dark = color_scheme == ColorScheme::Dark;
        preferences.push_str(&format!(
            "user_pref(\"layout.css.prefers-color-scheme.content-override\", {});\n",
            if is_dark { 0 } else { 1 }
        ));
        preferences.push_str(&format!(
            "user_pref(\"ui.systemUsesDarkTheme\", {});\n",
            if is_dark { 1 } else { 0 }
        ));
    }

    preferences
}

/// Reads a color scheme that was asked for, nothing means following the system.
#[doc(hidden)]
pub fn parse_color_scheme(value: &str) -> Result<Option<ColorScheme>, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dark" => Ok(Some(ColorScheme::Dark)),
        "light" => Ok(Some(ColorScheme::Light)),
        "" | "auto" | "system" => Ok(None),
        _ => Err(format!(
            "unknown color scheme \"{}\", it can be dark, light, or auto",
            value
        )),
    }
}

/// Reads the size of the viewport, which goes as WIDTHxHEIGHT.
#[doc(hidden)]
pub fn parse_viewport(value: &str) -> Result<(u32, u32), String> {
    value
        .trim()
        .split_once(['x', 'X'])
        .and_then(|(width, height)| Some((width.parse().ok()?, height.parse().ok()?)))
        .filter(|(width, height)| *width > 0 && *height > 0)
        .ok_or(format!(
            "\"{}\" is not a size, it goes like 1920x1080",
            value
        ))
}

fn choose_viewport(options: &MonolithOptions) -> Result<(u32, u32), String> {
    if let Some(viewport) = options.browser_viewport {
        return Ok(viewport);
    }

    match env::var_os(BROWSER_VIEWPORT_VARIABLE) {
        Some(value) => parse_viewport(&value.to_string_lossy())
            .map_err(|error| format!("{}: {}", BROWSER_VIEWPORT_VARIABLE, error)),
        None => Ok(DEFAULT_VIEWPORT),
    }
}

/// Reads whether the browser is to keep pages from running JavaScript.
#[doc(hidden)]
pub fn parse_disable_js(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "" | "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!(
            "\"{}\" is neither yes nor no, it can be 1 or 0, true or false",
            value
        )),
    }
}

fn choose_js_disabled(options: &MonolithOptions) -> Result<bool, String> {
    if let Some(is_js_disabled) = options.browser_disable_js {
        return Ok(is_js_disabled);
    }

    match env::var_os(BROWSER_DISABLE_JS_VARIABLE) {
        Some(value) => parse_disable_js(&value.to_string_lossy())
            .map_err(|error| format!("{}: {}", BROWSER_DISABLE_JS_VARIABLE, error)),
        None => Ok(false),
    }
}

/// Reads how many screens to scroll through, zero means none at all.
#[doc(hidden)]
pub fn parse_scroll_limit(value: &str) -> Result<u32, String> {
    value
        .trim()
        .parse()
        .map_err(|_| format!("\"{}\" is not a number of screens", value))
}

fn choose_scroll_limit(options: &MonolithOptions) -> Result<u32, String> {
    if let Some(limit) = options.browser_scroll_limit {
        return Ok(limit);
    }

    match env::var_os(BROWSER_SCROLL_LIMIT_VARIABLE) {
        Some(value) => parse_scroll_limit(&value.to_string_lossy())
            .map_err(|error| format!("{}: {}", BROWSER_SCROLL_LIMIT_VARIABLE, error)),
        None => Ok(DEFAULT_SCROLL_LIMIT),
    }
}

fn choose_color_scheme(options: &MonolithOptions) -> Result<Option<ColorScheme>, String> {
    if options.browser_color_scheme.is_some() {
        return Ok(options.browser_color_scheme);
    }

    let requested = match env::var_os(BROWSER_COLOR_SCHEME_VARIABLE) {
        Some(value) => parse_color_scheme(&value.to_string_lossy())
            .map_err(|error| format!("{}: {}", BROWSER_COLOR_SCHEME_VARIABLE, error))?,
        None => None,
    };

    Ok(requested.or_else(detect_color_scheme))
}

/// Browsers without a window have no way of knowing what the desktop looks like.
#[cfg(target_os = "macos")]
fn detect_color_scheme() -> Option<ColorScheme> {
    // The setting doesn't exist in light mode, which makes this fail
    let output =
        run_quietly("defaults", &["read", "-g", "AppleInterfaceStyle"]).unwrap_or_default();

    Some(parse_macos_color_scheme(&output))
}

#[cfg(target_os = "windows")]
fn detect_color_scheme() -> Option<ColorScheme> {
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    let output = run_quietly("reg", &["query", key, "/v", "AppsUseLightTheme"])?;

    parse_windows_color_scheme(&output)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn detect_color_scheme() -> Option<ColorScheme> {
    run_quietly(
        "gdbus",
        &[
            "call",
            "--session",
            "--timeout",
            "1",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.Settings.Read",
            "org.freedesktop.appearance",
            "color-scheme",
        ],
    )
    .and_then(|output| parse_portal_color_scheme(&output))
    .or_else(|| {
        let schema = "org.gnome.desktop.interface";
        let output = run_quietly("gsettings", &["get", schema, "color-scheme"])?;

        parse_gsettings_color_scheme(&output)
    })
}

/// Provides what the command has printed, if it worked.
fn run_quietly(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).to_string())
}

#[doc(hidden)]
pub fn parse_macos_color_scheme(output: &str) -> ColorScheme {
    match output.contains("Dark") {
        true => ColorScheme::Dark,
        false => ColorScheme::Light,
    }
}

#[doc(hidden)]
pub fn parse_windows_color_scheme(output: &str) -> Option<ColorScheme> {
    let value = output.split_whitespace().last()?;

    match value {
        "0x0" => Some(ColorScheme::Dark),
        "0x1" => Some(ColorScheme::Light),
        _ => None,
    }
}

#[doc(hidden)]
/// Reads answers of XDG Desktop Portal, which look like "(<<uint32 1>>,)".
pub fn parse_portal_color_scheme(output: &str) -> Option<ColorScheme> {
    let (_, value) = output.split_once("uint32 ")?;

    match value.chars().next()? {
        '1' => Some(ColorScheme::Dark),
        '2' => Some(ColorScheme::Light),
        // The user has no preference, which is a question for something else
        _ => None,
    }
}

#[doc(hidden)]
pub fn parse_gsettings_color_scheme(output: &str) -> Option<ColorScheme> {
    match output.trim().trim_matches('\'') {
        "prefer-dark" => Some(ColorScheme::Dark),
        "default" | "prefer-light" => Some(ColorScheme::Light),
        _ => None,
    }
}

#[doc(hidden)]
/// Describes a cookie the way browsers expect to receive it.
pub fn cookie_to_json(cookie: &Cookie) -> Value {
    let mut result = json!({
        // Browsers only apply cookies to subdomains if the domain starts with a dot
        "domain": match cookie.include_subdomains {
            true => format!(".{}", cookie.domain.trim_start_matches('.')),
            false => cookie.domain.trim_start_matches('.').to_string(),
        },
        "name": cookie.name,
        "path": cookie.path,
        "secure": cookie.https_only,
        "value": {"type": "string", "value": cookie.value},
    });
    if cookie.expires > 0 {
        result["expiry"] = json!(cookie.expires);
    }

    result
}

#[doc(hidden)]
/// Tells whether Monolith would've made this request.
pub fn is_request_allowed(url: &str, destination: &str, options: &MonolithOptions) -> bool {
    let is_wanted = match destination {
        "font" => !options.remove_fonts,
        "image" => !options.remove_images,
        "style" => !options.remove_styles,
        _ => true,
    };

    is_wanted && Url::parse(url).is_ok_and(|url| is_domain_allowed(&url, options))
}

fn find_browser() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os(BROWSER_PATH_VARIABLE).filter(|path| !path.is_empty()) {
        let path = PathBuf::from(path);
        if is_unsupported_browser(&path) {
            return Err(format!(
                "\"{}\" is not supported, {} has to be based on Firefox or Chromium",
                path.display(),
                BROWSER_PATH_VARIABLE
            ));
        }
        return match path.is_file() {
            true => Ok(path),
            false => Err(format!(
                "{} points to \"{}\", which is not a file",
                BROWSER_PATH_VARIABLE,
                path.display()
            )),
        };
    }

    let directories: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();

    find_browser_in(&directories)
        .or_else(|| find_installed_browsers().into_iter().next())
        .ok_or(format!(
            "could not find a browser, install Firefox or Chromium, or set {} to one",
            BROWSER_PATH_VARIABLE
        ))
}

/// Tells the two kinds of browsers apart, they speak different languages.
#[doc(hidden)]
pub fn uses_devtools(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    DEVTOOLS_BROWSERS
        .iter()
        .any(|browser| file_name.contains(browser))
}

/// What browsers of the Chromium kind have to be started with.
#[doc(hidden)]
pub fn devtools_arguments(
    profile: &Path,
    options: &MonolithOptions,
    is_visible: bool,
) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = DEVTOOLS_SWITCHES.iter().map(OsString::from).collect();
    let mut profile_argument = OsString::from("--user-data-dir=");
    profile_argument.push(profile);
    arguments.push(profile_argument);
    arguments.push("--remote-debugging-port=0".into());

    if !is_visible {
        arguments.push("--headless=new".into());
    }
    if options.insecure {
        arguments.push("--ignore-certificate-errors".into());
    }
    if let Some(user_agent) = &options.browser_user_agent {
        arguments.push(format!("--user-agent={}", user_agent).into());
    }
    // Sandboxes of these browsers refuse to work for the superuser
    #[cfg(unix)]
    // SAFETY: it only reads the identifier of the current user
    if unsafe { libc::geteuid() } == 0 {
        arguments.push("--no-sandbox".into());
    }
    arguments.push("about:blank".into());

    arguments
}

/// Tells what a request is for, using names from the Fetch standard that BiDi goes by.
#[doc(hidden)]
pub fn destination_of(resource_type: &str, is_main_frame: bool) -> &'static str {
    match resource_type {
        "Document" if is_main_frame => "document",
        "Document" => "iframe",
        "Font" => "font",
        "Image" => "image",
        "Media" => "video",
        "Script" => "script",
        "Stylesheet" => "style",
        _ => "",
    }
}

#[doc(hidden)]
pub fn is_unsupported_browser(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    UNSUPPORTED_BROWSERS
        .iter()
        .any(|browser| file_name.contains(browser))
}

#[doc(hidden)]
pub fn find_browser_in(directories: &[PathBuf]) -> Option<PathBuf> {
    BROWSER_NAMES.iter().find_map(|name| {
        directories.iter().find_map(|directory| {
            let path = directory.join(executable_name(name));
            path.is_file().then_some(path)
        })
    })
}

#[doc(hidden)]
pub fn executable_name(name: &str) -> OsString {
    let mut file_name = OsString::from(name);
    file_name.push(env::consts::EXE_SUFFIX);
    file_name
}

// Browsers don't always make it into PATH on these two
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub fn find_installed_browsers() -> Vec<PathBuf> {
    [
        ("Firefox", "firefox"),
        ("LibreWolf", "librewolf"),
        ("Google Chrome", "Google Chrome"),
        ("Chromium", "Chromium"),
        ("Brave Browser", "Brave Browser"),
        ("Microsoft Edge", "Microsoft Edge"),
    ]
    .iter()
    .map(|(application, executable)| {
        PathBuf::from(format!(
            "/Applications/{}.app/Contents/MacOS/{}",
            application, executable
        ))
    })
    .filter(|path| path.is_file())
    .collect()
}

#[cfg(target_os = "windows")]
#[doc(hidden)]
pub fn find_installed_browsers() -> Vec<PathBuf> {
    [
        "Mozilla Firefox\\firefox.exe",
        "Google\\Chrome\\Application\\chrome.exe",
        "BraveSoftware\\Brave-Browser\\Application\\brave.exe",
        "Microsoft\\Edge\\Application\\msedge.exe",
    ]
    .iter()
    .flat_map(|executable| {
        ["ProgramFiles", "ProgramFiles(x86)"]
            .into_iter()
            .filter_map(move |variable| {
                Some(PathBuf::from(env::var_os(variable)?).join(executable))
            })
    })
    .filter(|path| path.is_file())
    .collect()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[doc(hidden)]
pub fn find_installed_browsers() -> Vec<PathBuf> {
    vec![]
}

#[doc(hidden)]
/// Firefox that comes with Ubuntu is a snap, even if it's started by what's in /usr/bin:
/// a small script that sends everything to where snaps reside.
pub fn is_snap(executable: &Path) -> bool {
    let resolved = fs::canonicalize(executable).unwrap_or(executable.to_path_buf());
    if resolved.starts_with("/snap") || resolved.file_name().is_some_and(|name| name == "snap") {
        return true;
    }

    fs::metadata(executable).is_ok_and(|metadata| metadata.len() < MAX_LAUNCHER_SCRIPT_SIZE)
        && fs::read(executable).is_ok_and(|script| script.windows(6).any(|part| part == b"/snap/"))
}

/// Snaps go by the name of what they run, Ubuntu calls its launcher chromium-browser.
#[doc(hidden)]
pub fn snap_name(executable: &Path) -> String {
    executable
        .file_stem()
        .map(|name| {
            name.to_string_lossy()
                .trim_end_matches("-browser")
                .to_string()
        })
        .unwrap_or_default()
}

/// Snaps have a /tmp of their own, and can't see the real one.
fn create_profile_directory(executable: &Path, is_snap: bool) -> io::Result<TempDir> {
    let mut builder = Builder::new();
    builder.prefix("monolith-browser-");

    match env::var_os("HOME").filter(|_| is_snap) {
        Some(home) => {
            let directory = PathBuf::from(home)
                .join("snap")
                .join(snap_name(executable))
                .join("common");
            fs::create_dir_all(&directory)?;
            builder.tempdir_in(directory)
        }
        None => builder.tempdir(),
    }
}

#[doc(hidden)]
/// Looks for the address the browser prints into its error stream upon starting.
pub fn parse_endpoint(line: &str) -> Option<String> {
    if let Some((_, endpoint)) = line.split_once("DevTools listening on ") {
        return endpoint.trim().strip_prefix("ws://").map(str::to_string);
    }

    let (_, endpoint) = line.split_once("WebDriver BiDi listening on ")?;
    let address = endpoint.trim().strip_prefix("ws://")?;

    Some(address.trim_end_matches('/').to_string())
}

/// Browser process and everything that has to go away along with it.
struct RunningBrowser {
    child: Child,
    id: u64,
    // Goes away after the process does, once it's done using it
    _profile: TempDir,
}

// A place to find browsers at for when there's no time to wait for them to be done
impl RunningBrowser {
    /// Ends the browser, along with the helper processes that would otherwise outlive it, and keep
    /// writing into the profile while it's being deleted.
    #[cfg(unix)]
    fn terminate(&mut self) {
        let group = self.child.id() as libc::pid_t;

        // SAFETY: only sends signals, the browser has been made the leader of this group
        unsafe { libc::killpg(group, libc::SIGKILL) };
        let _ = self.child.kill();
        let _ = self.child.wait();

        let deadline = Instant::now() + BROWSER_SHUTDOWN_TIMEOUT;
        // SAFETY: signal 0 merely checks if there's anyone left in the group
        while Instant::now() < deadline && unsafe { libc::killpg(group, 0) } == 0 {
            thread::sleep(NETWORK_POLL_INTERVAL);
        }
    }

    #[cfg(not(unix))]
    fn terminate(&mut self) {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &self.child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

static IS_SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
static LAST_BROWSER_ID: AtomicU64 = AtomicU64::new(0);
static RUNNING_BROWSERS: Mutex<Vec<RunningBrowser>> = Mutex::new(Vec::new());

fn running_browsers() -> std::sync::MutexGuard<'static, Vec<RunningBrowser>> {
    RUNNING_BROWSERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Ends all browsers started by this process and deletes their profiles, nothing can be rendered
/// after that. Programs that can get interrupted should call it before they exit, destructors
/// don't get to run in those situations.
pub fn shut_down() {
    IS_SHUTTING_DOWN.store(true, Ordering::SeqCst);

    for mut browser in running_browsers().drain(..) {
        browser.terminate();
    }
}

struct Browser<'a> {
    connection: Option<Connection<'a>>,
    id: u64,
}

impl<'a> Browser<'a> {
    fn launch(
        executable: &Path,
        options: &'a MonolithOptions,
        is_visible: bool,
    ) -> Result<Browser<'a>, String> {
        let is_devtools = uses_devtools(executable);
        let is_snap = is_snap(executable);
        let color_scheme = choose_color_scheme(options)?;
        let viewport = choose_viewport(options)?;
        let is_js_disabled = choose_js_disabled(options)?;
        let profile = create_profile_directory(executable, is_snap)
            .map_err(|error| format!("could not create browser profile: {}", error))?;

        let mut command = Command::new(executable);
        if is_devtools {
            // Not something that switches are there for
            let directory = profile.path().join("Default");
            fs::create_dir(&directory)
                .and_then(|_| fs::write(directory.join("Preferences"), DEVTOOLS_PREFERENCES))
                .map_err(|error| format!("could not create browser profile: {}", error))?;

            command.args(devtools_arguments(profile.path(), options, is_visible));
        } else {
            let preferences = profile_preferences(options, color_scheme, is_js_disabled);
            fs::write(profile.path().join("user.js"), preferences)
                .map_err(|error| format!("could not create browser profile: {}", error))?;

            if !is_visible {
                command.arg("--headless");
            }
            command
                .args(["--no-remote", "--new-instance", "--profile"])
                .arg(profile.path())
                .args(["--remote-debugging-port", "0", "--wait-for-browser"])
                .arg("about:blank")
                .env("MOZ_CRASHREPORTER_DISABLE", "1")
                // Without this one the browser pays no attention to services.settings.server
                .env("MOZ_REMOTE_SETTINGS_DEVTOOLS", "1")
                // Size of the screen, for pages that go by that instead
                .env("MOZ_HEADLESS_WIDTH", viewport.0.to_string())
                .env("MOZ_HEADLESS_HEIGHT", viewport.1.to_string());
        }
        // Being moved to another home directory shouldn't cut it off from the display
        if let Some(home) = env::var_os("HOME").filter(|_| env::var_os("XAUTHORITY").is_none()) {
            command.env("XAUTHORITY", PathBuf::from(home).join(".Xauthority"));
        }
        // Otherwise it leaves empty directories all over user's home directory, snaps have their own
        if !is_snap {
            command
                .env("HOME", profile.path())
                .env_remove("XDG_CACHE_HOME")
                .env_remove("XDG_CONFIG_HOME")
                .env_remove("XDG_DATA_HOME")
                .env_remove("XDG_STATE_HOME");
        }
        // Whatever browsers create for themselves has to go away along with the profile
        command.env("TMPDIR", profile.path());
        // Keeps helper processes together, and away from what's typed into the terminal
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("could not start \"{}\": {}", executable.display(), error))?;

        // Keeps reading until the browser is gone, it would've stalled with its output left unread
        let stderr = child.stderr.take().expect("stderr has been piped");
        let (sender, receiver) = mpsc::channel();
        let last_output = Arc::new(Mutex::new(String::new()));
        let last_output_written = Arc::clone(&last_output);
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Some(endpoint) = parse_endpoint(&line) {
                    let _ = sender.send(endpoint);
                } else if let (false, Ok(mut last_output)) =
                    (line.trim().is_empty(), last_output_written.lock())
                {
                    *last_output = line;
                }
            }
        });

        let id = LAST_BROWSER_ID.fetch_add(1, Ordering::SeqCst);
        running_browsers().push(RunningBrowser {
            child,
            id,
            _profile: profile,
        });
        let mut browser = Browser {
            connection: None,
            id,
        };
        if IS_SHUTTING_DOWN.load(Ordering::SeqCst) {
            return Err("browser is not available anymore".to_string());
        }
        let endpoint = receiver
            .recv_timeout(BROWSER_STARTUP_TIMEOUT)
            .map_err(|_| {
                let output = last_output
                    .lock()
                    .map(|line| line.clone())
                    .unwrap_or_default();
                match output.is_empty() {
                    true => format!("\"{}\" did not start listening", executable.display()),
                    false => format!(
                        "\"{}\" did not start listening: {}",
                        executable.display(),
                        output.trim()
                    ),
                }
            })?;
        browser.connection = Some(Connection::open(
            &endpoint,
            options,
            is_devtools,
            color_scheme,
            viewport,
            is_js_disabled,
        )?);

        Ok(browser)
    }

    fn connection(&mut self) -> Result<&mut Connection<'a>, String> {
        self.connection
            .as_mut()
            .ok_or("no connection to the browser".to_string())
    }
}

impl Drop for Browser<'_> {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.as_mut() {
            connection.close();
        }

        let deadline = Instant::now() + BROWSER_SHUTDOWN_TIMEOUT;
        while Instant::now() < deadline && self.is_running() {
            thread::sleep(NETWORK_POLL_INTERVAL);
        }

        let mut browsers = running_browsers();
        if let Some(index) = browsers.iter().position(|browser| browser.id == self.id) {
            browsers.remove(index).terminate();
        }
    }
}

impl Browser<'_> {
    fn is_running(&self) -> bool {
        running_browsers()
            .iter_mut()
            .find(|browser| browser.id == self.id)
            .is_some_and(|browser| matches!(browser.child.try_wait(), Ok(None)))
    }
}

/// WebDriver BiDi session.
struct Connection<'a> {
    captured_responses: HashMap<String, CapturedResponse>,
    color_scheme: Option<ColorScheme>,
    is_devtools: bool,
    /// Pages get to run nothing
    is_js_disabled: bool,
    is_page_loaded: bool,
    last_command_id: u64,
    logged_urls: HashSet<String>,
    options: &'a MonolithOptions,
    // What DevTools Protocol uses to tell targets apart, none of that applies to BiDi
    page_session: Option<String>,
    page_target: String,
    refused_requests: HashSet<String>,
    requests_in_flight: HashSet<String>,
    socket: WebSocket<TcpStream>,
    viewport: (u32, u32),
    worker_urls: Vec<String>,
}

impl<'a> Connection<'a> {
    fn open(
        endpoint: &str,
        options: &'a MonolithOptions,
        is_devtools: bool,
        color_scheme: Option<ColorScheme>,
        viewport: (u32, u32),
        is_js_disabled: bool,
    ) -> Result<Connection<'a>, String> {
        let (address, path) = endpoint.split_once('/').unwrap_or((endpoint, "session"));
        let stream = TcpStream::connect(address)
            .map_err(|error| format!("could not connect to the browser: {}", error))?;
        let request = format!("ws://{}/{}", address, path)
            .into_client_request()
            .map_err(|error| format!("could not connect to the browser: {}", error))?;
        // Documents can get bigger than what WebSocket libraries consider reasonable
        let config = WebSocketConfig::default()
            .max_message_size(None)
            .max_frame_size(None);
        let (socket, _) = client_with_config(request, stream, Some(config))
            .map_err(|_| "could not connect to the browser".to_string())?;

        Ok(Connection {
            captured_responses: HashMap::new(),
            color_scheme,
            is_devtools,
            is_js_disabled,
            is_page_loaded: false,
            last_command_id: 0,
            logged_urls: HashSet::new(),
            options,
            page_session: None,
            page_target: String::new(),
            refused_requests: HashSet::new(),
            requests_in_flight: HashSet::new(),
            socket,
            viewport,
            worker_urls: vec![],
        })
    }

    fn start_session(
        &mut self,
        cookies: Option<&[Cookie]>,
        deadline: Instant,
    ) -> Result<(), String> {
        if self.is_devtools {
            return self.start_devtools_session(cookies, deadline);
        }

        self.command(
            "session.new",
            json!({"capabilities": {"alwaysMatch": {"acceptInsecureCerts": self.options.insecure}}}),
            deadline,
        )?;
        self.command(
            "session.subscribe",
            json!({"events": [
                "network.beforeRequestSent",
                "network.fetchError",
                "network.responseCompleted",
            ]}),
            deadline,
        )?;
        for cookie in cookies
            .unwrap_or_default()
            .iter()
            .filter(|c| !c.is_expired())
        {
            self.command(
                "storage.setCookie",
                json!({"cookie": cookie_to_json(cookie)}),
                deadline,
            )
            .map_err(|error| {
                format!(
                    "could not set cookie \"{}\": {}",
                    cookie.name,
                    String::from(error)
                )
            })?;
        }
        // Has to work, the alternative is contacting what the user has asked not to be contacted
        if self.is_filtering_requests() {
            self.command(
                "network.addIntercept",
                json!({"phases": ["beforeRequestSent"]}),
                deadline,
            )?;
        }
        // Older browsers can't do it, assets simply get retrieved by Monolith in that case
        let _ = self.command(
            "network.addDataCollector",
            json!({"dataTypes": ["response"], "maxEncodedDataSize": MAX_CAPTURED_RESPONSE_SIZE}),
            deadline,
        );

        Ok(())
    }

    fn is_filtering_requests(&self) -> bool {
        self.options.domains.is_some()
            || self.options.remove_fonts
            || self.options.remove_images
            || self.options.remove_styles
    }

    fn start_devtools_session(
        &mut self,
        cookies: Option<&[Cookie]>,
        deadline: Instant,
    ) -> Result<(), String> {
        // Browsers can be ready to talk before they've got anything to show
        let target = loop {
            let targets = self.command("Target.getTargets", json!({}), deadline)?;
            let page = targets["targetInfos"]
                .as_array()
                .and_then(|targets| targets.iter().find(|target| target["type"] == "page"))
                .and_then(|target| target["targetId"].as_str());

            match page {
                Some(target) => break target.to_string(),
                None if Instant::now() >= deadline => {
                    return Err("browser has no window to work with".to_string());
                }
                None => thread::sleep(NETWORK_POLL_INTERVAL),
            }
        };
        let attached = self.command(
            "Target.attachToTarget",
            json!({"flatten": true, "targetId": target}),
            deadline,
        )?;
        self.page_session = attached["sessionId"].as_str().map(str::to_string);
        self.page_target = target;

        self.command("Page.enable", json!({}), deadline)?;
        self.command("Network.enable", network_buffers(), deadline)?;
        let (width, height) = self.viewport;
        self.command(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "deviceScaleFactor": 0,
                "height": height,
                "mobile": false,
                "screenHeight": height,
                "screenWidth": width,
                "width": width,
            }),
            deadline,
        )?;
        // Has to work, the alternative is contacting what the user has asked not to be contacted
        if self.is_filtering_requests() {
            self.command("Fetch.enable", json!({}), deadline)?;
        }
        // Same here, the alternative is running what the user has asked not to be run
        if self.is_js_disabled {
            self.command(
                "Emulation.setScriptExecutionDisabled",
                json!({"value": true}),
                deadline,
            )?;
        }
        if let Some(color_scheme) = self.color_scheme {
            let value = match color_scheme {
                ColorScheme::Dark => "dark",
                ColorScheme::Light => "light",
            };
            self.command(
                "Emulation.setEmulatedMedia",
                json!({"features": [{"name": "prefers-color-scheme", "value": value}]}),
                deadline,
            )?;
        }
        for cookie in cookies
            .unwrap_or_default()
            .iter()
            .filter(|c| !c.is_expired())
        {
            self.command("Network.setCookie", devtools_cookie(cookie), deadline)
                .map_err(|error| {
                    format!(
                        "could not set cookie \"{}\": {}",
                        cookie.name,
                        String::from(error)
                    )
                })?;
        }
        // Workers and frames have their own traffic, and have to be stopped until it's being watched
        self.command(
            "Target.setAutoAttach",
            json!({"autoAttach": true, "flatten": true, "waitForDebuggerOnStart": true}),
            deadline,
        )?;

        Ok(())
    }

    fn open_devtools_page(&mut self, url: &str, deadline: Instant) -> Result<String, String> {
        let result = self.command("Page.navigate", json!({"url": url}), deadline)?;
        if let Some(reason) = result["errorText"].as_str() {
            return Err(format!("could not open page: {}", reason));
        }

        // Something on the page never finishing is no reason not to save the rest of it
        while !self.is_page_loaded {
            match self.read(deadline) {
                Ok(Some(message)) => self.observe(&message),
                Ok(None) => break,
                Err(_) => return Err(CommandError::Disconnected.into()),
            }
        }

        Ok(self.page_target.clone())
    }

    /// Browsers can be ready to talk before they've got anything to show (slow machines),
    /// the window they open on their own gets waited for, and if it doesn't show up, opened.
    fn find_or_open_window(&mut self, deadline: Instant) -> Result<String, String> {
        let waiting_until = (Instant::now() + WINDOW_WAIT_TIMEOUT).min(deadline);

        loop {
            let tree = self.command("browsingContext.getTree", json!({}), deadline)?;
            if let Some(context) = tree["contexts"][0]["context"].as_str() {
                return Ok(context.to_string());
            }
            if Instant::now() >= waiting_until {
                break;
            }
            thread::sleep(NETWORK_POLL_INTERVAL);
        }

        let created = self.command(
            "browsingContext.create",
            json!({"type": "window"}),
            deadline,
        )?;
        created["context"]
            .as_str()
            .map(str::to_string)
            .ok_or("browser has no window to work with".to_string())
    }

    /// Returns the browsing context the page ended up in.
    fn open_page(&mut self, url: &str, deadline: Instant) -> Result<String, String> {
        if self.is_devtools {
            return self.open_devtools_page(url, deadline);
        }

        let context = self.find_or_open_window(deadline)?;

        let (width, height) = self.viewport;
        self.command(
            "browsingContext.setViewport",
            json!({"context": context, "viewport": {"height": height, "width": width}}),
            deadline,
        )?;

        match self.command(
            "browsingContext.navigate",
            json!({"context": context, "url": url, "wait": "complete"}),
            deadline,
        ) {
            Ok(_) => Ok(context),
            // Something on the page never finished, what did finish can still be saved
            Err(CommandError::TimedOut(_)) => Ok(context),
            Err(CommandError::Disconnected) => Err(CommandError::Disconnected.into()),
            Err(CommandError::Failed(reason)) => Err(format!(
                "could not open page: {}",
                reason.trim_start_matches("Error: ")
            )),
        }
    }

    fn is_showing_challenge(&mut self, context: &str) -> bool {
        matches!(self.check_for_challenge(context), Ok(true))
    }

    fn check_for_challenge(&mut self, context: &str) -> Result<bool, CommandError> {
        let deadline = Instant::now() + NETWORK_QUIET_TIMEOUT;

        Ok(self.evaluate(CHALLENGE_SCRIPT, context, deadline)? == true)
    }

    /// Runs given script inside of the page, and provides what comes out of it.
    fn evaluate(
        &mut self,
        script: &str,
        context: &str,
        deadline: Instant,
    ) -> Result<Value, CommandError> {
        if self.is_devtools {
            let result = self.command(
                "Runtime.evaluate",
                json!({"expression": script, "returnByValue": true}),
                deadline,
            )?;
            return match result.get("exceptionDetails") {
                Some(_) => Err(CommandError::Failed("script has failed".to_string())),
                None => Ok(result["result"]["value"].clone()),
            };
        }

        let result = self.command(
            "script.evaluate",
            json!({
                "awaitPromise": false,
                "expression": script,
                "target": {"context": context},
            }),
            deadline,
        )?;

        match result["type"] == "success" {
            true => Ok(result["result"]["value"].clone()),
            false => Err(CommandError::Failed("script has failed".to_string())),
        }
    }

    /// Tells whether the page got past the challenge in time.
    fn wait_for_challenge_to_go_away(
        &mut self,
        context: &str,
        timeout: Duration,
    ) -> Result<bool, String> {
        let deadline = Instant::now() + timeout;

        while Instant::now() < deadline {
            match self.check_for_challenge(context) {
                Ok(false) => return Ok(true),
                Err(CommandError::Disconnected) => {
                    return Err("browser got closed before the challenge was solved".to_string());
                }
                // It can't be asked anything for as long as it's navigating somewhere
                _ => {}
            }

            let next_check = Instant::now() + CHALLENGE_POLL_INTERVAL;
            while let Ok(Some(message)) = self.read(next_check) {
                self.observe(&message);
            }
        }

        Ok(false)
    }

    fn wait_for_network_to_settle(&mut self, deadline: Instant) {
        self.wait_for_network(NETWORK_QUIET_PERIOD, NETWORK_QUIET_TIMEOUT, deadline);
    }

    fn wait_for_network(&mut self, quiet_period: Duration, timeout: Duration, deadline: Instant) {
        let deadline = deadline.min(Instant::now() + timeout);
        let mut quiet_since = Instant::now();

        while Instant::now() < deadline {
            if !self.requests_in_flight.is_empty() {
                quiet_since = Instant::now();
            } else if quiet_since.elapsed() >= quiet_period {
                return;
            }

            match self.read(Instant::now() + NETWORK_POLL_INTERVAL) {
                Ok(Some(message)) => self.observe(&message),
                Ok(None) => {}
                Err(_) => return,
            }
        }
    }

    /// Lets the page see all of itself, a lot of what's on pages only loads once it's in view.
    fn scroll_through_page(&mut self, context: &str, limit: u32, deadline: Instant) {
        let mut was_at_end = false;

        for screen in 0..limit {
            if Instant::now() >= deadline {
                break;
            }
            let Ok(is_at_end) = self.evaluate(SCROLL_SCRIPT, context, deadline) else {
                break;
            };
            let is_at_end = is_at_end == true;
            self.wait_for_network(SCROLL_QUIET_PERIOD, SCROLL_STEP_TIMEOUT, deadline);

            // Reaching the end twice means that nothing new has shown up in the meantime
            if is_at_end && was_at_end {
                break;
            }
            was_at_end = is_at_end;

            if screen + 1 == limit && !self.options.silent {
                print_info_message(&format!(
                    "Stopped scrolling after {} screens, {} changes that",
                    limit, BROWSER_SCROLL_LIMIT_VARIABLE
                ));
            }
        }

        if limit > 0 {
            let _ = self.evaluate(SCROLL_TO_TOP_SCRIPT, context, deadline);
            self.wait_for_network(SCROLL_QUIET_PERIOD, SCROLL_STEP_TIMEOUT, deadline);
        }
    }

    fn take_snapshot(&mut self, context: &str, deadline: Instant) -> Result<CapturedPage, String> {
        // A page that's taking long to load is better saved than not
        let deadline = deadline.max(Instant::now() + NETWORK_QUIET_TIMEOUT);
        let snapshot = self
            .evaluate(SNAPSHOT_SCRIPT, context, deadline)
            .map_err(|_| "could not read the rendered page".to_string())?;
        let snapshot = snapshot
            .as_str()
            .and_then(|value| serde_json::from_str::<Value>(value).ok())
            .ok_or("could not read the rendered page")?;

        Ok(CapturedPage {
            assets: vec![],
            content_type: snapshot["contentType"].as_str().unwrap_or("").to_string(),
            html: snapshot["html"].as_str().unwrap_or("").to_string(),
            adopted_style_marker: snapshot["adoptedStyleMarker"].as_str().map(str::to_string),
            title: snapshot["title"].as_str().unwrap_or("").to_string(),
            url: snapshot["url"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Body of a response, along with whether it's text that has already been decoded.
    fn response_body(&mut self, request: &str) -> Option<(Vec<u8>, bool)> {
        let deadline = Instant::now() + NETWORK_QUIET_TIMEOUT;

        if self.is_devtools {
            let (session, request) = request.split_once('|')?;
            let result = self
                .command_in(
                    Some(session),
                    "Network.getResponseBody",
                    json!({"requestId": request}),
                    deadline,
                )
                .ok()?;
            let body = result["body"].as_str()?;

            return match result["base64Encoded"] == true {
                true => Some((BASE64_STANDARD.decode(body).ok()?, false)),
                false => Some((body.as_bytes().to_vec(), true)),
            };
        }

        let params = json!({"dataType": "response", "request": request});
        let result = self.command("network.getData", params, deadline).ok()?;

        // Text comes as text, already decoded
        let is_decoded = result["bytes"]["type"] == "string";
        Some((decode_bytes(&result["bytes"])?, is_decoded))
    }

    /// Reports what the browser has received, the same way Monolith does for what it retrieves.
    fn log_response(&mut self, request: &str, status: u64, status_text: &str) {
        // Whatever is at the end of a redirect gets reported for all of it
        if (300..400).contains(&status) && status != 304 {
            return;
        }
        let Some(urls) = self
            .captured_responses
            .get(request)
            .map(|response| response.urls.clone())
        else {
            return;
        };
        let (Some(first_url), Some(last_url)) = (urls.first(), urls.last()) else {
            return;
        };

        if (status == 200 || status == 304) || self.options.ignore_errors {
            match first_url == last_url {
                true => self.log_info(first_url, first_url.clone()),
                false => self.log_info(last_url, format!("{} -> {}", first_url, last_url)),
            }
        } else {
            self.log_error(last_url, format!("{} {}", status, status_text).trim());
        }
    }

    fn log_info(&mut self, url: &str, message: String) {
        if self.is_worth_logging(url) {
            print_info_message(&message);
        }
    }

    fn log_error(&mut self, url: &str, reason: &str) {
        if self.is_worth_logging(url) {
            print_error_message(&format!("{} ({})", url, reason));
        }
    }

    // Pages ask for the same things repeatedly, and Monolith only reports those once
    fn is_worth_logging(&mut self, url: &str) -> bool {
        !self.options.silent
            && Url::parse(url).is_ok_and(|url| matches!(url.scheme(), "file" | "http" | "https"))
            && self.logged_urls.insert(url.to_string())
    }

    /// Asks the browser for bodies of the responses it received.
    fn collect_assets(&mut self) -> Vec<CapturedAsset> {
        let mut assets = vec![];
        let mut responses: Vec<(String, CapturedResponse)> =
            std::mem::take(&mut self.captured_responses)
                .into_iter()
                .collect();
        // Pages care about the order things have arrived in
        responses.sort_by_key(|(_, response)| response.sequence);

        for (request, response) in responses {
            if !response.has_body() {
                continue;
            }

            // Whatever is gone from the browser's memory by now gets retrieved by Monolith instead
            if let Some((data, is_decoded)) = self.response_body(&request) {
                let is_reusable = response.is_usable();
                let mut response = response;
                // What servers say beats what browsers make of it
                if let Some((_, content_type)) = response
                    .http
                    .response_headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
                {
                    response.content_type = content_type.clone();
                }
                // Text that's been decoded is UTF-8 now, no matter what it used to be
                if is_decoded && !response.content_type.is_empty() {
                    response.content_type = with_utf8_charset(&response.content_type);
                    set_content_type(&mut response.http.response_headers, &response.content_type);
                }
                assets.push(CapturedAsset {
                    content_type: response.content_type,
                    data,
                    destination: response.destination,
                    is_reusable,
                    method: response.method,
                    status: response.status,
                    urls: response.urls,
                    http: response.http,
                });
            }
        }

        // Scripts that workers start with never show up among the page's requests
        for url in std::mem::take(&mut self.worker_urls) {
            let is_missing = !assets.iter().any(|asset| asset.urls.contains(&url));
            if is_missing && let Some(data) = self.retrieve_from_page(&url) {
                assets.push(CapturedAsset {
                    content_type: "text/javascript".to_string(),
                    data,
                    destination: "worker".to_string(),
                    is_reusable: true,
                    method: "GET".to_string(),
                    status: 200,
                    urls: vec![url],
                    ..Default::default()
                });
            }
        }

        assets
    }

    fn retrieve_from_page(&mut self, url: &str) -> Option<Vec<u8>> {
        let expression = format!("fetch({}).then((response) => response.text())", json!(url));
        let deadline = Instant::now() + NETWORK_QUIET_TIMEOUT;
        let result = self
            .command(
                "Runtime.evaluate",
                json!({"awaitPromise": true, "expression": expression, "returnByValue": true}),
                deadline,
            )
            .ok()?;

        result["result"]["value"]
            .as_str()
            .map(|text| text.as_bytes().to_vec())
    }

    fn close(&mut self) {
        let deadline = Instant::now() + BROWSER_SHUTDOWN_TIMEOUT;
        let method = match self.is_devtools {
            true => "Browser.close",
            false => "browser.close",
        };
        let _ = self.command_in(None, method, json!({}), deadline);
    }

    fn command(
        &mut self,
        method: &str,
        params: Value,
        deadline: Instant,
    ) -> Result<Value, CommandError> {
        let session = self.page_session.clone();

        self.command_in(session.as_deref(), method, params, deadline)
    }

    fn command_in(
        &mut self,
        session: Option<&str>,
        method: &str,
        params: Value,
        deadline: Instant,
    ) -> Result<Value, CommandError> {
        let id = self
            .send_to(session, method, params)
            .map_err(|_| CommandError::Disconnected)?;

        loop {
            let message = match self.read(deadline) {
                Ok(Some(message)) => message,
                Ok(None) => return Err(CommandError::TimedOut(method.to_string())),
                Err(_) => return Err(CommandError::Disconnected),
            };

            if message["id"].as_u64() != Some(id) {
                self.observe(&message);
            } else if self.is_devtools {
                return match message.get("error") {
                    Some(error) => Err(CommandError::Failed(
                        error["message"]
                            .as_str()
                            .unwrap_or("unknown error")
                            .to_string(),
                    )),
                    None => Ok(message["result"].clone()),
                };
            } else if message["type"] == "success" {
                return Ok(message["result"].clone());
            } else {
                return Err(CommandError::Failed(
                    message["message"]
                        .as_str()
                        .or(message["error"].as_str())
                        .unwrap_or("unknown error")
                        .to_string(),
                ));
            }
        }
    }

    fn send(&mut self, method: &str, params: Value) -> Result<u64, String> {
        self.send_to(None, method, params)
    }

    fn send_to(
        &mut self,
        session: Option<&str>,
        method: &str,
        params: Value,
    ) -> Result<u64, String> {
        self.last_command_id += 1;
        let id = self.last_command_id;
        let mut command = json!({"id": id, "method": method, "params": params});
        if let Some(session) = session {
            command["sessionId"] = json!(session);
        }
        self.socket
            .send(Message::text(command.to_string()))
            .map_err(|error| error.to_string())?;

        Ok(id)
    }

    /// Provides the next message, or nothing if it didn't arrive in time.
    fn read(&mut self, deadline: Instant) -> Result<Option<Value>, String> {
        let time_left = deadline.saturating_duration_since(Instant::now());
        if time_left.is_zero() {
            return Ok(None);
        }
        self.socket
            .get_ref()
            .set_read_timeout(Some(time_left))
            .map_err(|error| error.to_string())?;

        match self.socket.read() {
            Ok(Message::Text(text)) => serde_json::from_str(text.as_str())
                .map(Some)
                .map_err(|error| error.to_string()),
            Ok(_) => Ok(Some(Value::Null)),
            Err(WebSocketError::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                Ok(None)
            }
            Err(error) => Err(error.to_string()),
        }
    }

    fn observe(&mut self, message: &Value) {
        if self.is_devtools {
            self.observe_devtools(message);
            return;
        }

        let Some(request) = message["params"]["request"]["request"].as_str() else {
            return;
        };

        match message["method"].as_str() {
            Some("network.beforeRequestSent") => {
                self.requests_in_flight.insert(request.to_string());

                // The browser sits on these until told what to do with them
                if message["params"]["isBlocked"] == true {
                    let details = &message["params"]["request"];
                    let url = details["url"].as_str().unwrap_or("");
                    let is_allowed = is_request_allowed(
                        url,
                        details["destination"].as_str().unwrap_or(""),
                        self.options,
                    );
                    let method = match is_allowed {
                        true => "network.continueRequest",
                        false => "network.failRequest",
                    };
                    let _ = self.send(method, json!({"request": request}));

                    if !is_allowed {
                        self.refused_requests.insert(request.to_string());
                        if Url::parse(url).is_ok_and(|url| !is_domain_allowed(&url, self.options)) {
                            self.log_error(url, "domain not allowed");
                        }
                    }
                }

                // Redirects keep the request's identifier, which is how they end up being one asset
                let url = message["params"]["request"]["url"].as_str().unwrap_or("");
                let sequence = self.captured_responses.len();
                let response = self
                    .captured_responses
                    .entry(request.to_string())
                    .or_default();
                let details = &message["params"]["request"];
                response.destination = details["destination"].as_str().unwrap_or("").to_string();
                response.method = details["method"].as_str().unwrap_or("GET").to_string();
                response.http.request_headers = bidi_headers(&details["headers"]);
                response.sequence = sequence;
                if !response.urls.iter().any(|existing| existing == url) {
                    response.urls.push(url.to_string());
                }
            }
            Some("network.responseCompleted") => {
                self.requests_in_flight.remove(request);

                let details = &message["params"]["response"];
                if let Some(response) = self.captured_responses.get_mut(request) {
                    let status = details["status"].as_u64().unwrap_or(0);
                    if !(300..400).contains(&status) || status == 304 {
                        response.content_type =
                            details["mimeType"].as_str().unwrap_or("").to_string();
                    }
                    response.receive(
                        status,
                        details["statusText"].as_str().unwrap_or(""),
                        details["protocol"].as_str().unwrap_or(""),
                        bidi_headers(&details["headers"]),
                    );
                }
                self.log_response(
                    request,
                    details["status"].as_u64().unwrap_or(0),
                    details["statusText"].as_str().unwrap_or(""),
                );
            }
            Some("network.fetchError") => {
                self.requests_in_flight.remove(request);
                let response = self.captured_responses.remove(request);

                // Requests that were refused have already been accounted for
                if !self.refused_requests.remove(request)
                    && let Some(url) = response.as_ref().and_then(|r| r.urls.last())
                {
                    // What browsers report here only means something to their developers
                    self.log_error(url, "request failed");
                }
            }
            _ => {}
        }
    }
}

impl Connection<'_> {
    fn observe_devtools(&mut self, message: &Value) {
        let params = &message["params"];
        let session = message["sessionId"].as_str().unwrap_or("");
        // Identifiers of requests are only unique within their sessions
        let key = format!("{}|{}", session, params["requestId"].as_str().unwrap_or(""));

        match message["method"].as_str() {
            Some("Page.loadEventFired") if self.page_session.as_deref() == Some(session) => {
                self.is_page_loaded = true;
            }
            Some("Target.attachedToTarget") => {
                if params["targetInfo"]["type"] == "worker"
                    && let Some(url) = params["targetInfo"]["url"].as_str()
                {
                    self.worker_urls.push(url.to_string());
                }

                let session = params["sessionId"].as_str().unwrap_or("").to_string();
                let session = Some(session.as_str());
                let _ = self.send_to(session, "Network.enable", network_buffers());
                if self.is_filtering_requests() {
                    let _ = self.send_to(session, "Fetch.enable", json!({}));
                }
                // Frames from other websites are pages of their own
                if self.is_js_disabled
                    && matches!(
                        params["targetInfo"]["type"].as_str(),
                        Some("iframe" | "page")
                    )
                {
                    let _ = self.send_to(
                        session,
                        "Emulation.setScriptExecutionDisabled",
                        json!({"value": true}),
                    );
                }
                let _ = self.send_to(
                    session,
                    "Target.setAutoAttach",
                    json!({"autoAttach": true, "flatten": true, "waitForDebuggerOnStart": true}),
                );
                let _ = self.send_to(session, "Runtime.runIfWaitingForDebugger", json!({}));
            }
            // The browser sits on these until told what to do with them
            Some("Fetch.requestPaused") => {
                let url = params["request"]["url"].as_str().unwrap_or("");
                let destination = destination_of(
                    params["resourceType"].as_str().unwrap_or(""),
                    params["frameId"] == self.page_target.as_str(),
                );
                let is_allowed = is_request_allowed(url, destination, self.options);
                let paused_request = json!({"requestId": params["requestId"]});

                if is_allowed {
                    let _ = self.send_to(Some(session), "Fetch.continueRequest", paused_request);
                } else {
                    let mut failure = paused_request;
                    failure["errorReason"] = json!("BlockedByClient");
                    let _ = self.send_to(Some(session), "Fetch.failRequest", failure);

                    let network_id = params["networkId"].as_str().unwrap_or("");
                    self.refused_requests
                        .insert(format!("{}|{}", session, network_id));
                    if Url::parse(url).is_ok_and(|url| !is_domain_allowed(&url, self.options)) {
                        self.log_error(url, "domain not allowed");
                    }
                }
            }
            Some("Network.requestWillBeSent") => {
                self.requests_in_flight.insert(key.clone());

                // Redirects keep the request's identifier, which is how they end up being one asset
                let details = &params["request"];
                let url = details["url"].as_str().unwrap_or("");
                let destination = destination_of(
                    params["type"].as_str().unwrap_or(""),
                    params["frameId"] == self.page_target.as_str(),
                );
                let sequence = self.captured_responses.len();
                let response = self.captured_responses.entry(key).or_default();
                // Redirects get reported along with where they lead
                let redirect = &params["redirectResponse"];
                if redirect.is_object() {
                    response.receive(
                        redirect["status"].as_u64().unwrap_or(0),
                        redirect["statusText"].as_str().unwrap_or(""),
                        redirect["protocol"].as_str().unwrap_or(""),
                        devtools_headers(&redirect["headers"]),
                    );
                }
                response.destination = destination.to_string();
                response.method = details["method"].as_str().unwrap_or("GET").to_string();
                response.http.request_headers = devtools_headers(&details["headers"]);
                response.sequence = sequence;
                if !response.urls.iter().any(|existing| existing == url) {
                    response.urls.push(url.to_string());
                }
            }
            Some("Network.responseReceived") => {
                let details = &params["response"];
                let status = details["status"].as_u64().unwrap_or(0);
                if let Some(response) = self.captured_responses.get_mut(&key) {
                    response.content_type = details["mimeType"].as_str().unwrap_or("").to_string();
                    response.receive(
                        status,
                        details["statusText"].as_str().unwrap_or(""),
                        details["protocol"].as_str().unwrap_or(""),
                        devtools_headers(&details["headers"]),
                    );
                }
                self.log_response(&key, status, details["statusText"].as_str().unwrap_or(""));
            }
            Some("Network.loadingFinished") => {
                self.requests_in_flight.remove(&key);
            }
            Some("Network.loadingFailed") => {
                self.requests_in_flight.remove(&key);
                let response = self.captured_responses.remove(&key);

                // Requests that were refused have already been accounted for
                if !self.refused_requests.remove(&key)
                    && params["canceled"] != true
                    && let Some(url) = response.as_ref().and_then(|r| r.urls.last())
                {
                    self.log_error(url, "request failed");
                }
            }
            _ => {}
        }
    }
}

// Browsers forget what they've received pretty quickly otherwise
fn network_buffers() -> Value {
    json!({
        "maxResourceBufferSize": MAX_CAPTURED_RESPONSE_SIZE,
        "maxTotalBufferSize": MAX_CAPTURED_RESPONSE_SIZE,
    })
}

/// Describes a cookie the way DevTools Protocol expects to receive it.
#[doc(hidden)]
pub fn devtools_cookie(cookie: &Cookie) -> Value {
    let description = cookie_to_json(cookie);
    let mut result = json!({
        "domain": description["domain"],
        "name": description["name"],
        "path": description["path"],
        "secure": description["secure"],
        "value": cookie.value,
    });
    if cookie.expires > 0 {
        result["expires"] = json!(cookie.expires);
    }

    result
}

#[doc(hidden)]
#[derive(Default)]
pub struct CapturedResponse {
    pub content_type: String,
    pub destination: String,
    pub method: String,
    pub sequence: usize,
    pub status: u64,
    pub urls: Vec<String>,
    pub http: HttpDetails,
}

impl CapturedResponse {
    /// Takes note of what the browser got back, a redirect becomes one of the steps taken.
    fn receive(
        &mut self,
        status: u64,
        status_text: &str,
        protocol: &str,
        headers: Vec<(String, String)>,
    ) {
        if (300..400).contains(&status) && status != 304 {
            self.http.redirects.push(Redirect {
                method: self.method.clone(),
                request_headers: self.http.request_headers.clone(),
                response_headers: headers,
                status,
                status_text: status_text.to_string(),
                url: self.urls.last().cloned().unwrap_or_default(),
            });
            return;
        }

        self.status = status;
        self.http.status_text = status_text.to_string();
        self.http.http_version = http_version_of(protocol);
        self.http.response_headers = headers;
        self.http.time = Utc::now().timestamp_millis();
    }

    // Partial responses, error pages, and answers to POST requests aren't what their URLs point to
    pub fn is_usable(&self) -> bool {
        self.method == "GET" && matches!(self.status, 200 | 203 | 304)
    }

    // Redirects have nothing to offer, and partial responses are only pieces of what's been asked for
    pub fn has_body(&self) -> bool {
        self.status >= 200 && self.status != 206 && !(300..400).contains(&self.status)
            || self.status == 304
    }
}

/// Browsers name protocols the way ALPN does.
#[doc(hidden)]
pub fn http_version_of(protocol: &str) -> String {
    match protocol.to_lowercase().as_str() {
        "http/1.0" => "HTTP/1.0",
        "h2" | "http/2" | "http/2.0" => "HTTP/2",
        "h3" | "http/3" => "HTTP/3",
        "" => "",
        _ => "HTTP/1.1",
    }
    .to_string()
}

/// WebDriver BiDi gives headers as lists of names and values.
#[doc(hidden)]
pub fn bidi_headers(headers: &Value) -> Vec<(String, String)> {
    headers
        .as_array()
        .map(|headers| {
            headers
                .iter()
                .filter_map(|header| {
                    let value = decode_bytes(&header["value"])?;
                    Some((
                        header["name"].as_str()?.to_string(),
                        String::from_utf8_lossy(&value).into_owned(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// DevTools Protocol gives headers as objects, with repeated ones joined by line breaks.
#[doc(hidden)]
pub fn devtools_headers(headers: &Value) -> Vec<(String, String)> {
    headers
        .as_object()
        .map(|headers| {
            headers
                .iter()
                .flat_map(|(name, value)| {
                    value
                        .as_str()
                        .unwrap_or("")
                        .split('\n')
                        .map(|value| (name.clone(), value.to_string()))
                        .collect::<Vec<_>>()
                })
                .collect()
        })
        .unwrap_or_default()
}

#[doc(hidden)]
pub fn decode_bytes(bytes: &Value) -> Option<Vec<u8>> {
    let value = bytes["value"].as_str()?;

    match bytes["type"].as_str()? {
        "base64" => BASE64_STANDARD.decode(value).ok(),
        "string" => Some(value.as_bytes().to_vec()),
        _ => None,
    }
}

enum CommandError {
    Disconnected,
    Failed(String),
    TimedOut(String),
}

impl From<CommandError> for String {
    fn from(error: CommandError) -> String {
        match error {
            CommandError::Disconnected => "lost connection to the browser".to_string(),
            CommandError::Failed(reason) => reason,
            CommandError::TimedOut(method) => {
                format!("timed out waiting for the browser to do {}", method)
            }
        }
    }
}
