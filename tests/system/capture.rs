use crate::support::{Server, Workspace, with_each_browser};

#[test]
fn runs_javascript() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>Loading</title><div id=a>Loading</div>\
                  <script src=/app.js></script>",
            ),
            (
                "/app.js",
                "text/javascript",
                b"fetch('/data.json').then(r=>r.json()).then(d=>{document.title=d.title;\
                  document.getElementById('a').textContent=d.text})",
            ),
            (
                "/data.json",
                "application/json",
                br#"{"title":"Rendered","text":"Made by JavaScript"}"#,
            ),
        ]);

        let html = Workspace::new(browser).render(&server.url("/"), &["--remove-js"]);

        assert!(html.contains("Made by JavaScript"));
        assert!(html.contains("<title>Rendered</title>"));
        assert!(!html.contains("fetch("));
    });
}

#[test]
fn reuses_what_only_the_browser_was_given() {
    with_each_browser(|browser| {
        let server = Server::with_responder(Box::new(|request| {
            match (request.path.as_str(), request.cookie.contains("pass=1")) {
                ("/", _) => (
                    200,
                    "text/html",
                    b"<!doctype html><title>t</title><script>document.cookie='pass=1';\
                      const l=document.createElement('link');l.rel='stylesheet';\
                      l.href='/gated.css';document.head.append(l)</script>"
                        .to_vec(),
                ),
                ("/gated.css", true) => (200, "text/css", b"p{color:rgb(1,2,3)}".to_vec()),
                _ => (403, "text/plain", b"Forbidden".to_vec()),
            }
        }));

        let html = Workspace::new(browser).render(&server.url("/"), &["--remove-js"]);

        assert!(
            html.contains("<style>p{color:rgb(1,2,3)}</style>"),
            "{}",
            html
        );
    });
}

#[test]
fn saves_pages_that_are_not_documents() {
    with_each_browser(|browser| {
        let server = Server::new(&[("/a.txt", "text/plain", b"Plain text")]);

        let html = Workspace::new(browser).render(&server.url("/a.txt"), &[]);

        assert!(html.contains("Plain text"));
    });
}

#[test]
fn reports_pages_that_can_not_be_opened() {
    with_each_browser(|browser| {
        let output = Workspace::new(browser)
            .monolith()
            .arg("http://127.0.0.1:1/")
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(1));
    });
}
