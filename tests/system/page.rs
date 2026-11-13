use std::sync::{Arc, Mutex};

use crate::support::{Server, Workspace, assert_success, with_each_browser};

fn page_that_describes_itself(script: &str) -> Vec<u8> {
    format!(
        "<!doctype html><title>t</title><p id=o></p><script>o.textContent={}</script>",
        script
    )
    .into_bytes()
}

#[test]
fn uses_large_viewport_by_default() {
    with_each_browser(|browser| {
        let body = page_that_describes_itself("innerWidth+'x'+innerHeight");
        let server = Server::new(&[("/", "text/html", &body)]);

        let html = Workspace::new(browser).render(&server.url("/"), &["--remove-js"]);

        assert!(html.contains(">1920x1080<"));
    });
}

#[test]
fn uses_specified_viewport() {
    with_each_browser(|browser| {
        let body = page_that_describes_itself("innerWidth+'x'+innerHeight");
        let server = Server::new(&[("/", "text/html", &body)]);

        let output = Workspace::new(browser)
            .monolith()
            .env("MONOLITH_BROWSER_VIEWPORT", "800x600")
            .args(["--remove-js", &server.url("/")])
            .output()
            .unwrap();

        assert_success(&output);
        assert!(String::from_utf8_lossy(&output.stdout).contains(">800x600<"));
    });
}

#[test]
fn uses_specified_color_scheme() {
    with_each_browser(|browser| {
        let body = page_that_describes_itself(
            "matchMedia('(prefers-color-scheme: dark)').matches?'Dark':'Light'",
        );
        let server = Server::new(&[("/", "text/html", &body)]);

        for (value, expected) in [("dark", ">Dark<"), ("light", ">Light<")] {
            let output = Workspace::new(browser)
                .monolith()
                .env("MONOLITH_BROWSER_COLOR_SCHEME", value)
                .args(["--remove-js", &server.url("/")])
                .output()
                .unwrap();

            assert_success(&output);
            assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
        }
    });
}

const LAZY_PAGE: &[u8] = b"<!doctype html><title>t</title><style>div{height:900px}</style>\
    <div><img id=a data-pic=/a.png width=9 height=9></div>\
    <div><img id=b data-pic=/b.png width=9 height=9></div>\
    <div><img id=c data-pic=/c.png width=9 height=9></div>\
    <script>const o=new IntersectionObserver(es=>es.forEach(e=>{if(e.isIntersecting){\
    e.target.src=e.target.dataset.pic;o.unobserve(e.target)}}));\
    document.querySelectorAll('img').forEach(i=>o.observe(i))</script>";

#[test]
fn scrolls_through_pages() {
    with_each_browser(|browser| {
        let server = Server::new(&[("/", "text/html", LAZY_PAGE), ("/c.png", "image/png", b"c")]);

        let output = Workspace::new(browser)
            .monolith()
            .env("MONOLITH_BROWSER_SCROLL_LIMIT", "10")
            .args(["--remove-js", &server.url("/")])
            .output()
            .unwrap();

        assert_success(&output);
        assert!(server.was_asked_for("/c.png"));
    });
}

#[test]
fn does_not_scroll_if_told_not_to() {
    with_each_browser(|browser| {
        let server = Server::new(&[("/", "text/html", LAZY_PAGE), ("/c.png", "image/png", b"c")]);

        Workspace::new(browser).render(&server.url("/"), &["--remove-js"]);

        assert!(!server.was_asked_for("/c.png"));
    });
}

#[test]
fn stops_scrolling_pages_that_never_end() {
    with_each_browser(|browser| {
        let server = Server::new(&[(
            "/",
            "text/html",
            b"<!doctype html><title>t</title><div id=f></div>\
              <script>const add=()=>{f.insertAdjacentHTML('beforeend',\
              '<p style=height:400px>item</p>')};for(let i=0;i<9;i++)add();\
              addEventListener('scroll',()=>{for(let i=0;i<9;i++)add()})</script>",
        )]);

        let output = Workspace::new(browser)
            .monolith()
            .env("MONOLITH_BROWSER_SCROLL_LIMIT", "3")
            .args(["--remove-js", &server.url("/")])
            .output()
            .unwrap();

        assert_success(&output);
    });
}

#[test]
fn freezes_web_components() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><x-outer></x-outer><script>\
                  customElements.define('x-inner',class extends HTMLElement{connectedCallback(){\
                  this.attachShadow({mode:'open'}).innerHTML='<b>Nested text</b>'}});\
                  customElements.define('x-outer',class extends HTMLElement{connectedCallback(){\
                  const r=this.attachShadow({mode:'open'});\
                  const s=new CSSStyleSheet();s.replaceSync('p{color:rgb(1,2,3)}');\
                  r.adoptedStyleSheets=[s];\
                  r.innerHTML='<p>Footer text</p><img src=/a.png><x-inner></x-inner>'}});\
                  </script>",
            ),
            ("/a.png", "image/png", b"\x89PNG"),
        ]);

        let html = Workspace::new(browser).render(&server.url("/"), &["--remove-js"]);

        assert!(html.contains("shadowrootmode=\"open\""));
        assert!(html.contains("Footer text"));
        assert!(html.contains("Nested text"));
        assert!(html.contains("color: rgb(1, 2, 3)"));
        assert!(html.contains("data:image/png"));
    });
}

#[test]
fn runs_no_javascript_when_told_not_to() {
    with_each_browser(|browser| {
        let page = b"<!doctype html><title>t</title><p id=o>static</p>\
            <iframe src=/frame></iframe><script>o.textContent='dynamic';fetch('/page')</script>";
        let server = Server::new(&[
            ("/", "text/html", page),
            (
                "/frame",
                "text/html",
                b"<script>fetch('/from-frame')</script>",
            ),
        ]);

        let output = Workspace::new(browser)
            .monolith()
            .env("MONOLITH_BROWSER_DISABLE_JS", "1")
            .args(["--remove-js", &server.url("/")])
            .output()
            .unwrap();

        assert_success(&output);
        let html = String::from_utf8_lossy(&output.stdout);
        assert!(html.contains(">static<"), "{html}");
        assert!(!server.was_asked_for("/page"));
        assert!(!server.was_asked_for("/from-frame"));
    });
}

#[test]
fn leaves_svgs_of_other_origins_out() {
    with_each_browser(|browser| {
        // The page points at the server under another name, which makes it another origin
        let page: Arc<Mutex<String>> = Arc::default();
        let server = {
            let page = Arc::clone(&page);
            Server::with_responder(Box::new(move |request| match request.path.as_str() {
                "/" => (200, "text/html", page.lock().unwrap().clone().into_bytes()),
                _ => (
                    200,
                    "image/svg+xml",
                    b"<svg xmlns=\"http://www.w3.org/2000/svg\">\
                      <symbol id=\"x\"><rect width=\"1\" height=\"1\"/></symbol></svg>"
                        .to_vec(),
                ),
            }))
        };
        *page.lock().unwrap() = format!(
            "<!doctype html><title>t</title><svg id=own><use href=/own.svg#x></use></svg>\
             <svg id=other><use href={}#x></use></svg>",
            server.url("/other.svg").replace("127.0.0.1", "localhost")
        );

        for arguments in [&[][..], &["--remove-js"][..]] {
            let html = Workspace::new(browser).render(&server.url("/"), arguments);
            assert!(html.contains("<symbol id=\"x\">"), "{html}");
            assert!(
                html.contains("<svg id=\"other\"><use></use></svg>"),
                "{html}"
            );
        }
        // Browsers don't ask for them, neither does Monolith
        assert!(server.was_asked_for("/own.svg"));
        assert!(!server.was_asked_for("/other.svg"));
    });
}
