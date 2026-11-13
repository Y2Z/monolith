use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;

use monolith::browser::{executable_name, find_installed_browsers, uses_devtools};
use monolith::url::Url;
use tempfile::TempDir;

const FIREFOX_NAMES: [&str; 4] = ["firefox", "firefox-esr", "librewolf", "icecat"];
const CHROMIUM_NAMES: [&str; 7] = [
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
    "brave-browser",
    "brave",
    "microsoft-edge",
];

// Browsers aren't small, and running too many of them at once only leads to timeouts
const MAX_BROWSERS_AT_ONCE: usize = 3;
static BROWSERS_RUNNING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

/// Browsers of given names found in PATH, in the order of the names.
fn find_in_path(names: &[&str]) -> Vec<PathBuf> {
    let Some(path) = env::var_os("PATH") else {
        return vec![];
    };
    let directories: Vec<PathBuf> = env::split_paths(&path).collect();

    names
        .iter()
        .flat_map(|name| {
            directories
                .iter()
                .map(|directory| directory.join(executable_name(name)))
                .filter(|path| path.is_file())
                .collect::<Vec<PathBuf>>()
        })
        .collect()
}

/// Whether a Chromium-based browser works for Monolith at all: some are installed in ways that
/// keep them from starting (snaps missing what they depend on, for one), those don't get tested.
fn starts(browser: &Path) -> bool {
    let server = Server::new(&[("/", "text/html", b"<title>t</title>works")]);
    let Ok(temporary_directory) = TempDir::new() else {
        return false;
    };
    Command::new(env!("CARGO_BIN_EXE_monolith"))
        .env("MONOLITH_BROWSER", browser)
        .env("MONOLITH_BROWSER_SCROLL_LIMIT", "0")
        .env("TMPDIR", temporary_directory.path())
        .env("TEMP", temporary_directory.path())
        .env("TMP", temporary_directory.path())
        .args(["--browser", "--quiet", "--timeout", "60", "--output", "-"])
        .arg(server.url("/"))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains("works")
        })
}

/// Browsers to test: Firefox (or anything based on it) first, then the first Chromium-based
/// browser that can be started.
pub fn browsers() -> Vec<PathBuf> {
    static BROWSERS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    BROWSERS
        .get_or_init(|| {
            let browsers = find_browsers();
            match browsers.is_empty() {
                true => say("no browsers found, nothing is going to be tested in browsers"),
                false => say(&format!(
                    "testing in {}",
                    browsers
                        .iter()
                        .map(|browser| browser.display().to_string())
                        .collect::<Vec<String>>()
                        .join(" and ")
                )),
            }
            browsers
        })
        .clone()
}

/// Says something no matter what: tests only get their output shown if they fail, this goes
/// around that (only print!() and the like get captured).
fn say(text: &str) {
    let _ = writeln!(std::io::stderr(), "{}", text);
}

fn find_browsers() -> Vec<PathBuf> {
    if let Some(paths) = env::var_os("MONOLITH_TEST_BROWSERS").filter(|paths| !paths.is_empty()) {
        return env::split_paths(&paths).collect();
    }

    let installed = find_installed_browsers();
    let candidates = |names: &[&str], is_devtools: bool| {
        let mut candidates = find_in_path(names);
        candidates.extend(
            installed
                .iter()
                .filter(|path| uses_devtools(path) == is_devtools)
                .cloned(),
        );
        // PATH can have the same directory more than once (or links to it); what's in it can't
        // be told apart by where it leads, snaps all lead to the same program
        let mut seen: Vec<PathBuf> = vec![];
        candidates.retain(|path| {
            let path = match (path.parent().map(fs::canonicalize), path.file_name()) {
                (Some(Ok(directory)), Some(name)) => directory.join(name),
                _ => path.clone(),
            };
            let is_new = !seen.contains(&path);
            seen.push(path);
            is_new
        });
        candidates
    };

    let firefox = candidates(&FIREFOX_NAMES, false).into_iter().next();
    let chromium = candidates(&CHROMIUM_NAMES, true)
        .into_iter()
        .find(|browser| match starts(browser) {
            true => true,
            false => {
                say(&format!(
                    "{} doesn't start, it won't be tested",
                    browser.display()
                ));
                false
            }
        });

    [firefox, chromium].into_iter().flatten().collect()
}

/// Runs given test for every browser there is.
pub fn with_each_browser(test: impl Fn(&Path)) {
    for browser in browsers() {
        let (running, finished) = &BROWSERS_RUNNING;
        let mut count = running.lock().unwrap_or_else(|error| error.into_inner());
        while *count >= MAX_BROWSERS_AT_ONCE {
            count = finished
                .wait(count)
                .unwrap_or_else(|error| error.into_inner());
        }
        *count += 1;
        drop(count);

        println!("testing {}", browser.display());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| test(&browser)));

        *running.lock().unwrap_or_else(|error| error.into_inner()) -= 1;
        finished.notify_one();

        if let Err(panic) = result {
            println!("failed using {}", browser.display());
            std::panic::resume_unwind(panic);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub authorization: String,
    pub cookie: String,
    pub host: String,
    pub path: String,
    pub user_agent: String,
}

type Responder = Box<dyn Fn(&Request) -> (u16, &'static str, Vec<u8>) + Send + Sync>;

/// Website for the browser to visit, it's gone along with the test.
pub struct Server {
    pub address: String,
    requests: Arc<Mutex<Vec<Request>>>,
    /// Header lines to send along with what's at given paths
    headers: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl Server {
    /// Takes paths along with media types and what's to be served there.
    pub fn new(routes: &[(&str, &'static str, &[u8])]) -> Server {
        let routes: HashMap<String, (&'static str, Vec<u8>)> = routes
            .iter()
            .map(|(path, media_type, body)| (path.to_string(), (*media_type, body.to_vec())))
            .collect();

        Server::with_responder(Box::new(move |request| {
            let path = request.path.split('?').next().unwrap_or("");
            match routes.get(path) {
                Some((media_type, body)) => (200, media_type, body.clone()),
                None => (404, "text/plain", b"Not Found".to_vec()),
            }
        }))
    }

    pub fn with_responder(responder: Responder) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(vec![]));
        let headers: Arc<Mutex<HashMap<String, Vec<String>>>> = Arc::default();
        let responder = Arc::new(responder);
        let recorded = Arc::clone(&requests);
        let extra = Arc::clone(&headers);

        thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                let responder = Arc::clone(&responder);
                let recorded = Arc::clone(&recorded);
                let extra = Arc::clone(&extra);
                thread::spawn(move || respond(stream, &responder, &recorded, &extra));
            }
        });

        Server {
            address,
            requests,
            headers,
        }
    }

    /// Makes what's at given path come with given header line.
    pub fn add_header(&self, path: &str, header: &str) {
        self.headers
            .lock()
            .unwrap()
            .entry(path.to_string())
            .or_default()
            .push(header.to_string());
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.address, path)
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    pub fn was_asked_for(&self, path: &str) -> bool {
        self.requests()
            .iter()
            .any(|request| request.path.starts_with(path))
    }
}

fn respond(
    mut stream: TcpStream,
    responder: &Responder,
    requests: &Mutex<Vec<Request>>,
    headers: &Mutex<HashMap<String, Vec<String>>>,
) {
    let mut received = vec![];
    let mut buffer = [0; 4096];

    let header_end = loop {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(size) => received.extend_from_slice(&buffer[..size]),
        }
        if let Some(end) = received.windows(4).position(|part| part == b"\r\n\r\n") {
            break end + 4;
        }
    };

    let head = String::from_utf8_lossy(&received[..header_end]).to_string();
    let mut lines = head.lines();
    let mut request_line = lines.next().unwrap_or("").split(' ').skip(1);
    let mut request = Request {
        authorization: String::new(),
        cookie: String::new(),
        host: String::new(),
        path: request_line.next().unwrap_or("").to_string(),
        user_agent: String::new(),
    };
    let mut content_length = 0;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let value = value.trim();
            match name.to_ascii_lowercase().as_str() {
                "content-length" => content_length = value.parse().unwrap_or(0),
                "authorization" => request.authorization = value.to_string(),
                "cookie" => request.cookie = value.to_string(),
                "host" => request.host = value.to_string(),
                "user-agent" => request.user_agent = value.to_string(),
                _ => {}
            }
        }
    }
    while received.len() < header_end + content_length {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(size) => received.extend_from_slice(&buffer[..size]),
        }
    }

    requests.lock().unwrap().push(request.clone());

    let (status, media_type, mut body) = responder(&request);
    // Redirects have where they lead to as their body
    let location = match (300..400).contains(&status) {
        true => format!(
            "Location: {}\r\n",
            String::from_utf8_lossy(&std::mem::take(&mut body))
        ),
        false => String::new(),
    };
    let extra: String = headers
        .lock()
        .unwrap()
        .get(request.path.split('?').next().unwrap_or(""))
        .map(|lines| lines.iter().map(|line| format!("{}\r\n", line)).collect())
        .unwrap_or_default();
    let head = format!(
        "HTTP/1.1 {} Status\r\nContent-Type: {}\r\n{}{}Content-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        media_type,
        location,
        extra,
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
}

/// A place for one test to keep its files, including what browsers use as their profiles.
pub struct Workspace {
    pub browser: PathBuf,
    pub directory: TempDir,
    pub temporary_directory: TempDir,
}

impl Workspace {
    pub fn new(browser: &Path) -> Workspace {
        Workspace {
            browser: browser.to_path_buf(),
            // Browsers that come as snaps have a /tmp of their own, they'd see nothing in ours
            directory: TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap(),
            temporary_directory: TempDir::new().unwrap(),
        }
    }

    /// Monolith, writing to STDOUT.
    pub fn monolith(&self) -> Command {
        let mut command = self.monolith_writing_files();
        command.args(["--output", "-"]);
        command
    }

    /// Monolith, writing wherever it's told to.
    pub fn monolith_writing_files(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_monolith"));
        command
            .env("MONOLITH_BROWSER", &self.browser)
            .env("MONOLITH_BROWSER_SCROLL_LIMIT", "0")
            .env("MONOLITH_BROWSER_COLOR_SCHEME", "light")
            .env("TMPDIR", self.temporary_directory.path())
            .env("TEMP", self.temporary_directory.path())
            .env("TMP", self.temporary_directory.path())
            .args(["--browser", "--timeout", "300"]);
        command
    }

    /// Saves given URL into a file, and provides its path.
    pub fn save(&self, url: &str, arguments: &[&str]) -> PathBuf {
        let path = self.directory.path().join("saved.html");
        let output = self
            .monolith_writing_files()
            .args(arguments)
            .args(["--output"])
            .arg(&path)
            .arg(url)
            .output()
            .unwrap();
        assert_success(&output);

        path
    }

    /// Turns an archive into a document, no browser is needed for that.
    pub fn unpack(&self, name: &str, archive: &[u8]) -> PathBuf {
        let source = self.directory.path().join(name);
        let path = self.directory.path().join("unpacked.html");
        fs::write(&source, archive).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_monolith"))
            .arg("--output")
            .arg(&path)
            .arg(&source)
            .output()
            .unwrap();
        assert_success(&output);

        path
    }

    /// Provides what comes out for given URL.
    pub fn render(&self, url: &str, arguments: &[&str]) -> String {
        let output = self.monolith().args(arguments).arg(url).output().unwrap();
        assert_success(&output);

        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// Opens a saved document in the browser, and provides what it has turned into.
    pub fn open(&self, path: &Path) -> String {
        self.render(
            Url::from_file_path(path).unwrap().as_str(),
            &["--remove-js"],
        )
    }

    pub fn has_leftovers(&self) -> bool {
        fs::read_dir(self.temporary_directory.path())
            .unwrap()
            .next()
            .is_some()
    }
}

pub fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "monolith has failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
