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
use serde_json::{Value, json};
use tempfile::{Builder, TempDir};
use tungstenite::client::{IntoClientRequest, client_with_config};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Error as WebSocketError, Message, WebSocket};
use url::Url;

use crate::cookies::Cookie;
use crate::core::{MonolithOptions, print_error_message, print_info_message};
use crate::net::is_domain_allowed;
use crate::session::Session;

const BROWSER_COLOR_SCHEME_VARIABLE: &str = "MONOLITH_BROWSER_COLOR_SCHEME";
const BROWSER_PATH_VARIABLE: &str = "MONOLITH_BROWSER";
// Firefox and browsers made out of it, the ones that come with WebDriver BiDi
const BROWSER_NAMES: [&str; 4] = ["firefox", "firefox-esr", "librewolf", "icecat"];
// These don't speak WebDriver BiDi on their own, what's needed for that doesn't come with them
const UNSUPPORTED_BROWSERS: [&str; 7] = [
    "brave", "chrome", "chromium", "edge", "opera", "safari", "vivaldi",
];
const MAX_LAUNCHER_SCRIPT_SIZE: u64 = 64 * 1024;
const BROWSER_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const BROWSER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
// How long the network has to stay silent for the page to be considered done
const NETWORK_QUIET_PERIOD: Duration = Duration::from_millis(1000);
// Pages that never stop talking to their servers get this much time after they've loaded
const NETWORK_QUIET_TIMEOUT: Duration = Duration::from_secs(15);
const NETWORK_POLL_INTERVAL: Duration = Duration::from_millis(100);
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
user_pref("extensions.update.enabled", false);
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

// Makes the document's current state a part of its HTML, then serializes it.
// Passwords stay out, that's the only thing here which could've been typed in by a script
// and yet shouldn't end up in a file.
const SNAPSHOT_SCRIPT: &str = r#"(() => {
    const rules = (sheet) => {
        try {
            return Array.from(sheet.cssRules, (rule) => rule.cssText).join("\n");
        } catch (error) {
            return "";
        }
    };
    // CSS-in-JS libraries insert their rules directly, those never appear in the markup
    for (const style of document.querySelectorAll("style")) {
        if (style.sheet && !style.textContent.trim()) {
            style.textContent = rules(style.sheet);
        }
    }
    for (const sheet of document.adoptedStyleSheets || []) {
        const style = document.createElement("style");
        style.textContent = rules(sheet);
        document.head.appendChild(style);
    }
    for (const input of document.querySelectorAll("input")) {
        if (input.type === "checkbox" || input.type === "radio") {
            input.toggleAttribute("checked", input.checked);
        } else if (input.type !== "password" && input.type !== "file") {
            input.setAttribute("value", input.value);
        }
    }
    for (const textarea of document.querySelectorAll("textarea")) {
        textarea.textContent = textarea.value;
    }
    for (const option of document.querySelectorAll("option")) {
        option.toggleAttribute("selected", option.selected);
    }
    const doctype = document.doctype
        ? new XMLSerializer().serializeToString(document.doctype)
        : "";
    return JSON.stringify({
        contentType: document.contentType,
        html: doctype + document.documentElement.outerHTML,
        title: document.title,
        url: location.href,
    });
})()"#;

// Anything larger than this has to be retrieved again by Monolith
const MAX_CAPTURED_RESPONSE_SIZE: u64 = 200_000_000;

/// Something the page has received, along with all URLs that led to it.
#[derive(Debug)]
pub struct CapturedAsset {
    pub content_type: String,
    pub data: Vec<u8>,
    pub destination: String,
    /// Whether it's something that can stand in for retrieving the URL
    pub is_reusable: bool,
    pub method: String,
    pub status: u64,
    pub urls: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorScheme {
    Dark,
    Light,
}

#[derive(Debug)]
pub struct RenderedPage {
    pub assets: Vec<CapturedAsset>,
    pub content_type: String,
    pub html: String,
    pub title: String,
    pub url: String,
}

/// Opens given URL in a browser that's about to cease to exist, and returns what the page became.
/// Everything the session is set up to refuse gets refused by the browser as well.
///
/// The browser stays out of sight, unless the page turns out to be a challenge that someone has to
/// deal with, in which case it gets opened again in a window.
pub fn render_page(url: &str, session: &Session) -> Result<RenderedPage, String> {
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
) -> Result<Option<RenderedPage>, String> {
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
pub fn profile_preferences(options: &MonolithOptions, color_scheme: Option<ColorScheme>) -> String {
    let mut preferences = PROFILE_PREFERENCES.to_string();

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
        "font" => !options.no_fonts,
        "image" => !options.no_images,
        "style" => !options.no_styles,
        _ => true,
    };

    is_wanted && Url::parse(url).is_ok_and(|url| is_domain_allowed(&url, options))
}

fn find_browser() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os(BROWSER_PATH_VARIABLE).filter(|path| !path.is_empty()) {
        let path = PathBuf::from(path);
        if is_unsupported_browser(&path) {
            return Err(format!(
                "\"{}\" is not supported yet, {} has to be Firefox or a browser based on it",
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
        .or_else(find_installed_browser)
        .ok_or(format!(
            "could not find Firefox, install it or set {} to its path",
            BROWSER_PATH_VARIABLE
        ))
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
fn find_installed_browser() -> Option<PathBuf> {
    ["Firefox", "LibreWolf"].iter().find_map(|name| {
        let path = PathBuf::from(format!(
            "/Applications/{}.app/Contents/MacOS/{}",
            name,
            name.to_lowercase()
        ));
        path.is_file().then_some(path)
    })
}

#[cfg(target_os = "windows")]
fn find_installed_browser() -> Option<PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .find_map(|variable| {
            let path = PathBuf::from(env::var_os(variable)?)
                .join("Mozilla Firefox")
                .join("firefox.exe");
            path.is_file().then_some(path)
        })
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn find_installed_browser() -> Option<PathBuf> {
    None
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

/// Snaps have a /tmp of their own, and can't see the real one.
fn create_profile_directory(is_snap: bool) -> io::Result<TempDir> {
    let mut builder = Builder::new();
    builder.prefix("monolith-browser-");

    match env::var_os("HOME").filter(|_| is_snap) {
        Some(home) => {
            let directory = PathBuf::from(home).join("snap/firefox/common");
            fs::create_dir_all(&directory)?;
            builder.tempdir_in(directory)
        }
        None => builder.tempdir(),
    }
}

#[doc(hidden)]
/// Looks for the address the browser prints into its error stream upon starting.
pub fn parse_endpoint(line: &str) -> Option<String> {
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
        let is_snap = is_snap(executable);
        let profile = create_profile_directory(is_snap)
            .map_err(|error| format!("could not create browser profile: {}", error))?;
        let preferences = profile_preferences(options, choose_color_scheme(options)?);
        fs::write(profile.path().join("user.js"), preferences)
            .map_err(|error| format!("could not create browser profile: {}", error))?;

        let mut command = Command::new(executable);
        if !is_visible {
            command.arg("--headless");
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
        // Keeps helper processes together, and away from what's typed into the terminal
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
            .args(["--no-remote", "--new-instance", "--profile"])
            .arg(profile.path())
            .args(["--remote-debugging-port", "0", "--wait-for-browser"])
            .arg("about:blank")
            .env("MOZ_CRASHREPORTER_DISABLE", "1")
            // Without this one the browser pays no attention to services.settings.server
            .env("MOZ_REMOTE_SETTINGS_DEVTOOLS", "1")
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
        browser.connection = Some(Connection::open(&endpoint, options)?);

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
    last_command_id: u64,
    logged_urls: HashSet<String>,
    options: &'a MonolithOptions,
    refused_requests: HashSet<String>,
    requests_in_flight: HashSet<String>,
    socket: WebSocket<TcpStream>,
}

impl<'a> Connection<'a> {
    fn open(endpoint: &str, options: &'a MonolithOptions) -> Result<Connection<'a>, String> {
        let stream = TcpStream::connect(endpoint)
            .map_err(|error| format!("could not connect to the browser: {}", error))?;
        let request = format!("ws://{}/session", endpoint)
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
            last_command_id: 0,
            logged_urls: HashSet::new(),
            options,
            refused_requests: HashSet::new(),
            requests_in_flight: HashSet::new(),
            socket,
        })
    }

    fn start_session(
        &mut self,
        cookies: Option<&[Cookie]>,
        deadline: Instant,
    ) -> Result<(), String> {
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
        if self.options.domains.is_some()
            || self.options.no_fonts
            || self.options.no_images
            || self.options.no_styles
        {
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

    /// Returns the browsing context the page ended up in.
    fn open_page(&mut self, url: &str, deadline: Instant) -> Result<String, String> {
        let tree = self.command("browsingContext.getTree", json!({}), deadline)?;
        let context = tree["contexts"][0]["context"]
            .as_str()
            .ok_or("browser has no window to work with")?
            .to_string();

        match self.command(
            "browsingContext.navigate",
            json!({"context": context, "url": url, "wait": "complete"}),
            deadline,
        ) {
            Ok(_) => Ok(context),
            // Something on the page never finished, what did finish can still be saved
            Err(CommandError::TimedOut) => Ok(context),
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
        let result = self.command(
            "script.evaluate",
            json!({
                "awaitPromise": false,
                "expression": CHALLENGE_SCRIPT,
                "target": {"context": context},
            }),
            Instant::now() + NETWORK_QUIET_TIMEOUT,
        )?;

        Ok(result["result"]["value"] == true)
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
        let deadline = deadline.min(Instant::now() + NETWORK_QUIET_TIMEOUT);
        let mut quiet_since = Instant::now();

        while Instant::now() < deadline {
            if !self.requests_in_flight.is_empty() {
                quiet_since = Instant::now();
            } else if quiet_since.elapsed() >= NETWORK_QUIET_PERIOD {
                return;
            }

            match self.read(Instant::now() + NETWORK_POLL_INTERVAL) {
                Ok(Some(message)) => self.observe(&message),
                Ok(None) => {}
                Err(_) => return,
            }
        }
    }

    fn take_snapshot(&mut self, context: &str, deadline: Instant) -> Result<RenderedPage, String> {
        // A page that's taking long to load is better saved than not
        let deadline = deadline.max(Instant::now() + NETWORK_QUIET_TIMEOUT);
        let result = self.command(
            "script.evaluate",
            json!({
                "awaitPromise": false,
                "expression": SNAPSHOT_SCRIPT,
                "target": {"context": context},
            }),
            deadline,
        )?;
        if result["type"] != "success" {
            return Err("could not read the rendered page".to_string());
        }

        let snapshot = result["result"]["value"]
            .as_str()
            .and_then(|value| serde_json::from_str::<Value>(value).ok())
            .ok_or("could not read the rendered page")?;

        Ok(RenderedPage {
            assets: vec![],
            content_type: snapshot["contentType"].as_str().unwrap_or("").to_string(),
            html: snapshot["html"].as_str().unwrap_or("").to_string(),
            title: snapshot["title"].as_str().unwrap_or("").to_string(),
            url: snapshot["url"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Reports what the browser has received, the same way Monolith does for what it retrieves.
    fn log_response(&mut self, request: &str, details: &Value) {
        let status = details["status"].as_u64().unwrap_or(0);
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
            let status_text = details["statusText"].as_str().unwrap_or("");
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

            let deadline = Instant::now() + NETWORK_QUIET_TIMEOUT;
            let params = json!({"dataType": "response", "request": request});
            // Whatever is gone from the browser's memory by now gets retrieved by Monolith instead
            if let Ok(result) = self.command("network.getData", params, deadline)
                && let Some(data) = decode_bytes(&result["bytes"])
            {
                let is_reusable = response.is_usable();
                assets.push(CapturedAsset {
                    content_type: response.content_type,
                    data,
                    destination: response.destination,
                    is_reusable,
                    method: response.method,
                    status: response.status,
                    urls: response.urls,
                });
            }
        }

        assets
    }

    fn close(&mut self) {
        let deadline = Instant::now() + BROWSER_SHUTDOWN_TIMEOUT;
        let _ = self.command("browser.close", json!({}), deadline);
    }

    fn command(
        &mut self,
        method: &str,
        params: Value,
        deadline: Instant,
    ) -> Result<Value, CommandError> {
        let id = self
            .send(method, params)
            .map_err(|_| CommandError::Disconnected)?;

        loop {
            let message = match self.read(deadline) {
                Ok(Some(message)) => message,
                Ok(None) => return Err(CommandError::TimedOut),
                Err(_) => return Err(CommandError::Disconnected),
            };

            if message["id"].as_u64() != Some(id) {
                self.observe(&message);
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
        self.last_command_id += 1;
        let id = self.last_command_id;
        let command = json!({"id": id, "method": method, "params": params}).to_string();
        self.socket
            .send(Message::text(command))
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
                response.sequence = sequence;
                if !response.urls.iter().any(|existing| existing == url) {
                    response.urls.push(url.to_string());
                }
            }
            Some("network.responseCompleted") => {
                self.requests_in_flight.remove(request);

                let details = &message["params"]["response"];
                if let Some(response) = self.captured_responses.get_mut(request) {
                    response.status = details["status"].as_u64().unwrap_or(0);
                    response.content_type = details["mimeType"].as_str().unwrap_or("").to_string();
                }
                self.log_response(request, details);
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

#[doc(hidden)]
#[derive(Default)]
pub struct CapturedResponse {
    pub content_type: String,
    pub destination: String,
    pub method: String,
    pub sequence: usize,
    pub status: u64,
    pub urls: Vec<String>,
}

impl CapturedResponse {
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
    TimedOut,
}

impl From<CommandError> for String {
    fn from(error: CommandError) -> String {
        match error {
            CommandError::Disconnected => "lost connection to the browser".to_string(),
            CommandError::Failed(reason) => reason,
            CommandError::TimedOut => "timed out waiting for the browser".to_string(),
        }
    }
}
