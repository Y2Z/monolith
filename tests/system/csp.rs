use std::path::PathBuf;
use std::process::Command;

use crate::support::{Server, Workspace, assert_success, with_each_browser};

/// A page with a policy that names its scripts and styles, and one that loads another script once
/// it runs.
fn website() -> Server {
    let server = Server::with_responder(Box::new(|request| match request.path.as_str() {
        "/" => (200, "text/html", PAGE.as_bytes().to_vec()),
        "/style.css" => (
            200,
            "text/css",
            b"#linked { color: rgb(1, 2, 3); }".to_vec(),
        ),
        "/late.js" => (200, "text/javascript", b"document.title = 'late';".to_vec()),
        _ => (200, "image/png", b"\x89PNG".to_vec()),
    }));
    server.add_header(
        "/",
        "Content-Security-Policy: script-src 'nonce-abc' 'strict-dynamic'; style-src 'self' 'nonce-abc'; img-src 'self'",
    );
    server
}

const PAGE: &str = "<!doctype html><html><head><title>t</title>\
    <link rel=stylesheet href=/style.css>\
    <style nonce=abc>#named { color: rgb(4, 5, 6); }</style>\
    <style>#unnamed { color: rgb(7, 8, 9); }</style></head><body>\
    <p id=named>n</p><p id=unnamed>u</p><p id=linked>l</p><p id=allowed>no</p><p id=blocked>no</p>\
    <p id=attribute style=\"color: rgb(10, 11, 12)\">a</p>\
    <img src=/own.png><img src=http://localhost:1/other.png>\
    <script nonce=abc>document.getElementById('allowed').textContent = 'ran';</script>\
    <script>document.getElementById('blocked').textContent = 'ran';</script>\
    </body></html>";

/// Saves without a browser.
fn save(workspace: &Workspace, url: &str, arguments: &[&str]) -> PathBuf {
    let path = workspace.directory.path().join("saved.html");
    let output = Command::new(env!("CARGO_BIN_EXE_monolith"))
        .args(arguments)
        .arg("--output")
        .arg(&path)
        .arg(url)
        .output()
        .unwrap();
    assert_success(&output);
    path
}

#[test]
fn saved_pages_do_what_their_policies_allow() {
    with_each_browser(|browser| {
        let server = website();
        let workspace = Workspace::new(browser);
        let saved = save(&workspace, &server.url("/"), &[]);
        let html = workspace.open(&saved);

        assert!(html.contains("<p id=\"allowed\">ran</p>"), "{html}");
        assert!(html.contains("<p id=\"blocked\">no</p>"), "{html}");
        // What the policy didn't allow doesn't get applied when the saved page is opened either
        assert!(html.contains("rgb(4, 5, 6)"), "{html}");
        assert!(!html.contains("rgb(7, 8, 9)"), "{html}");
        assert!(!html.contains("rgb(10, 11, 12)"), "{html}");
        assert!(html.contains("rgb(1, 2, 3)"), "{html}");
        assert!(server.was_asked_for("/own.png"));
    });
}

#[test]
fn saved_pages_stay_isolated_whatever_their_policies_say() {
    with_each_browser(|browser| {
        let server = website();
        let late = server.url("/late.js");
        let page = PAGE.replace(
            "document.getElementById('allowed').textContent = 'ran';",
            &format!(
                "document.getElementById('allowed').textContent = 'ran';\
                 const s = document.createElement('script'); s.src = '{}'; document.head.append(s);",
                late
            ),
        );
        let server = {
            drop(server);
            let page = page.clone();
            let server =
                Server::with_responder(Box::new(move |request| match request.path.as_str() {
                    "/" => (200, "text/html", page.clone().into_bytes()),
                    _ => (200, "text/javascript", b"document.title = 'late';".to_vec()),
                }));
            server.add_header(
                "/",
                "Content-Security-Policy: script-src 'nonce-abc' 'strict-dynamic'",
            );
            server
        };
        let workspace = Workspace::new(browser);
        let saved = save(&workspace, &server.url("/"), &["-I"]);
        let html = workspace.open(&saved);

        // What the page's scripts get to load from anywhere (strict-dynamic) stays out
        assert!(html.contains("<p id=\"allowed\">ran</p>"), "{html}");
        assert!(!server.was_asked_for("/late.js"));
        assert!(!html.contains("<title>late</title>"), "{html}");
    });
}
