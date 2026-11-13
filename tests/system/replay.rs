use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use monolith::url::Url;
use sha2::{Digest, Sha384};

use crate::support::{Server, Workspace, assert_success, with_each_browser};

const APP: &[u8] = br#"<!doctype html><title>Loading</title><div id=a></div><script>
(async () => {
  const results = [];
  results.push((await (await fetch('/api/items?t=' + Math.random())).json()).name);
  results.push(await new Promise((done) => {
    const x = new XMLHttpRequest();
    x.open('GET', '/api/xhr');
    x.responseType = 'json';
    x.onload = () => done(x.response.name);
    x.send();
  }));
  results.push((await (await fetch('/api/post', {method: 'POST', body: '{}'})).json()).name);
  results.push((await fetch('/api/missing')).status);
  results.push((await fetch('/api/items')).headers.get('date') ? 'dated' : 'undated');
  await new Promise((done) => {
    const s = document.createElement('script');
    s.src = '/late.js';
    s.onload = done;
    document.head.append(s);
  });
  results.push(window.LATE);
  document.cookie = 'a=1';
  results.push(document.cookie);
  document.body.insertAdjacentHTML('beforeend', '<img src="/never/' + Math.random() + '.png">');
  results.push(await fetch('/never/' + Math.random()).then(() => 'reached', () => 'refused'));
  a.textContent = results.join('|');
})();
</script>"#;

fn app() -> Server {
    Server::new(&[
        ("/", "text/html", APP),
        ("/api/items", "application/json", br#"{"name":"fetched"}"#),
        ("/api/xhr", "application/json", br#"{"name":"xhr"}"#),
        ("/api/post", "application/json", br#"{"name":"posted"}"#),
        ("/late.js", "text/javascript", b"window.LATE='late'"),
    ])
}

#[test]
fn works_without_network() {
    with_each_browser(|browser| {
        let server = app();
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/"), &[]);
        let requests = server.requests().len();

        let html = workspace.open(&saved);

        assert!(
            html.contains("fetched|xhr|posted|404|dated|late|a=1|refused"),
            "{}",
            html
        );
        assert_eq!(
            server.requests().len(),
            requests,
            "saved document has reached out"
        );
    });
}

#[test]
fn works_in_every_other_browser_too() {
    with_each_browser(|saving_browser| {
        with_each_browser_for_opening(saving_browser);
    });
}

fn with_each_browser_for_opening(saving_browser: &std::path::Path) {
    let server = app();
    let workspace = Workspace::new(saving_browser);
    let saved = workspace.save(&server.url("/"), &[]);

    with_each_browser(|browser| {
        let html = Workspace::new(browser).open(&saved);

        assert!(
            html.contains("fetched|xhr|posted"),
            "saved with {}, opened with {}: {}",
            saving_browser.display(),
            browser.display(),
            html
        );
    });
}

#[test]
fn keeps_title_the_page_ended_up_with() {
    with_each_browser(|browser| {
        let server = Server::new(&[(
            "/",
            "text/html",
            b"<!doctype html><title>Before</title><script>document.title='After'</script>",
        )]);
        let workspace = Workspace::new(browser);
        let output = workspace
            .monolith_writing_files()
            .args(["--output", "%title%.html", &server.url("/")])
            .current_dir(workspace.directory.path())
            .output()
            .unwrap();

        assert_success(&output);
        assert!(workspace.directory.path().join("After.html").is_file());
    });
}

#[test]
fn freezes_if_javascript_is_not_wanted() {
    with_each_browser(|browser| {
        let server = app();

        let html = Workspace::new(browser).render(&server.url("/"), &["--remove-js"]);

        assert!(!html.contains("data-monolith-url"));
        assert!(html.contains("fetched|xhr"));
    });
}

#[test]
fn freezes_for_mhtml() {
    with_each_browser(|browser| {
        let server = app();

        let output = Workspace::new(browser).render(&server.url("/"), &["--format", "mhtml"]);

        assert!(!output.contains("data-monolith-url"));
    });
}

#[test]
fn modules_find_each_other() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/deep/",
                "text/html",
                b"<!doctype html><title>t</title><div id=a></div>\
                  <script type=module src=main.js></script>",
            ),
            (
                "/deep/main.js",
                "text/javascript",
                b"import {b} from './lib/b.js';import c from '../c.js';import './side.js';\
                  const d=await import('./lib/d.js');\
                  const e=new URL('./data.txt',import.meta.url).pathname;\
                  const text='from \"./not-an-import.js\"';\
                  a.textContent=[b,c,d.default,window.SIDE,e,text.length].join('|')",
            ),
            ("/deep/lib/b.js", "text/javascript", b"export const b='b'"),
            ("/c.js", "text/javascript", b"export default 'c'"),
            ("/deep/lib/d.js", "text/javascript", b"export default 'd'"),
            ("/deep/side.js", "text/javascript", b"window.SIDE='side'"),
        ]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/deep/"), &[]);

        let html = workspace.open(&saved);

        assert!(html.contains("b|c|d|side|/deep/data.txt|25"), "{}", html);
    });
}

#[test]
fn modules_share_a_single_instance() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><div id=a></div>\
                  <script type=module src=/main.js></script>",
            ),
            (
                "/main.js",
                "text/javascript",
                b"window.COUNT=(window.COUNT||0)+1;export const x='shared';\
                  import('/other.js').then(m=>{a.textContent='count:'+window.COUNT+m.y})",
            ),
            (
                "/other.js",
                "text/javascript",
                b"import {x} from '/main.js';export const y=':'+x",
            ),
        ]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/"), &[]);
        let html = workspace.open(&saved);

        assert!(html.contains("count:1:shared"), "{}", html);
    });
}

#[test]
fn module_workers_run() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><div id=a></div><script>\
                  const w=new Worker('/worker.js',{type:'module'});\
                  w.onmessage=e=>{a.textContent=e.data};w.postMessage('ping')</script>",
            ),
            (
                "/worker.js",
                "text/javascript",
                b"import {v} from './lib.js';\
                  onmessage=async e=>{const t=await (await fetch('/data.txt')).text();\
                  postMessage(e.data+':'+v+':'+t+':'+location.pathname)}",
            ),
            ("/lib.js", "text/javascript", b"export const v='module'"),
            ("/data.txt", "text/plain", b"replayed"),
        ]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/"), &[]);

        let html = workspace.open(&saved);

        assert!(html.contains("ping:module:replayed:/worker.js"), "{}", html);
    });
}

#[test]
fn pages_know_where_they_came_from() {
    with_each_browser(|browser| {
        let server = Server::new(&[(
            "/app/settings",
            "text/html",
            b"<!doctype html><title>t</title><div id=a></div><script>\
              const before=location.pathname+location.search;\
              history.pushState({n:1},'','/app/other');\
              const pushed=location.pathname+':'+history.state.n;\
              setTimeout(()=>{history.back();setTimeout(()=>{\
              a.textContent=[before,pushed,location.pathname,document.URL.length>0].join('|')\
              },50)},50)</script>",
        )]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/app/settings?tab=2"), &[]);

        let html = workspace.open(&saved);

        assert!(
            html.contains("/app/settings?tab=2|/app/other:1|/app/settings|true"),
            "{}",
            html
        );
    });
}

fn integrity_of(data: &[u8]) -> String {
    format!("sha384-{}", BASE64_STANDARD.encode(Sha384::digest(data)))
}

const SCRIPT: &[u8] = b"a.textContent='ran:'+location.pathname";

fn page_with_integrity(integrity: &str) -> Vec<u8> {
    format!(
        "<!doctype html><title>t</title><div id=a>idle</div>\
         <script type=module integrity=\"{}\" src=/a.js></script>",
        integrity
    )
    .into_bytes()
}

#[test]
fn scripts_keep_working_despite_being_modified() {
    with_each_browser(|browser| {
        let body = page_with_integrity(&integrity_of(SCRIPT));
        let server = Server::new(&[
            ("/", "text/html", &body),
            ("/a.js", "text/javascript", SCRIPT),
        ]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/"), &[]);

        assert!(workspace.open(&saved).contains("ran:/"));
    });
}

#[test]
fn integrity_set_by_scripts_gets_ignored() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><div id=a></div><script>\
                  const s=document.createElement('script');s.integrity=window.HASH;\
                  s.src='/chunk.js';document.head.append(s)</script>",
            ),
            (
                "/chunk.js",
                "text/javascript",
                b"a.textContent='chunk:'+location.pathname",
            ),
        ]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/"), &[]);

        assert!(workspace.open(&saved).contains("chunk:/"));
    });
}

#[test]
fn stylesheets_keep_what_they_point_at() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><i id=i class=c></i><div id=a></div><script>\
                  const l=document.createElement('link');l.rel='stylesheet';\
                  l.href='/css/a.css';l.onload=()=>{a.textContent=\
                  getComputedStyle(i).backgroundImage.slice(0,20)};\
                  document.head.append(l)</script>",
            ),
            ("/css/a.css", "text/css", b".c{background:url(icon.png)}"),
            ("/css/icon.png", "image/png", b"\x89PNG"),
        ]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/"), &[]);

        assert!(workspace.open(&saved).contains("data:image/png"));
    });
}

#[test]
fn lazy_content_loads_again_when_needed() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><style>#s{margin-top:3000px;height:9px}</style>\
                  <div id=s></div><div id=a></div><script>\
                  new IntersectionObserver(async es=>{if(es[0].isIntersecting)\
                  a.textContent=await (await fetch('/lazy.txt')).text()}).observe(s)</script>",
            ),
            ("/lazy.txt", "text/plain", b"Found while scrolling"),
        ]);
        let workspace = Workspace::new(browser);
        let output = workspace
            .monolith_writing_files()
            .env("MONOLITH_BROWSER_SCROLL_LIMIT", "10")
            .args(["--output"])
            .arg(workspace.directory.path().join("saved.html"))
            .arg(server.url("/"))
            .output()
            .unwrap();
        assert_success(&output);

        let reopened = workspace
            .monolith()
            .env("MONOLITH_BROWSER_SCROLL_LIMIT", "10")
            .arg("--remove-js")
            .arg(
                Url::from_file_path(workspace.directory.path().join("saved.html"))
                    .unwrap()
                    .as_str(),
            )
            .output()
            .unwrap();

        assert_success(&reopened);
        assert!(String::from_utf8_lossy(&reopened.stdout).contains("Found while scrolling"));
    });
}

const ARCHIVED_FILES: [(&str, &str, &str); 3] = [
    (
        "https://archived.test/page",
        "text/html",
        "<title>t</title><body><script type=module src=/app.js></script>",
    ),
    (
        "https://archived.test/app.js",
        "text/javascript",
        "import {name} from './lib.js';\
         const data = await (await fetch('/data.json')).json();\
         document.body.textContent = name + '|' + data.value + '|' + location.pathname;",
    ),
    (
        "https://archived.test/lib.js",
        "text/javascript",
        "export const name = 'imported';",
    ),
];
const ARCHIVED_DATA: (&str, &str, &str) = (
    "https://archived.test/data.json",
    "application/json",
    "{\"value\":\"fetched\"}",
);

#[test]
fn warc_gets_its_javascript_back() {
    with_each_browser(|browser| {
        let mut warc = String::new();
        for (url, content_type, body) in ARCHIVED_FILES.iter().chain([&ARCHIVED_DATA]) {
            let block = format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\n\r\n{body}");
            warc.push_str(&format!(
                "WARC/1.1\r\nWARC-Type: response\r\nWARC-Target-URI: {url}\r\n\
                 Content-Length: {}\r\n\r\n{block}\r\n\r\n",
                block.len()
            ));
        }

        let workspace = Workspace::new(browser);
        let saved = workspace.unpack("page.warc", warc.as_bytes());
        let html = workspace.open(&saved);

        assert!(html.contains("imported|fetched|/page"), "{}", html);
    });
}

#[test]
fn har_gets_its_javascript_back() {
    with_each_browser(|browser| {
        let entries: Vec<serde_json::Value> = ARCHIVED_FILES
            .iter()
            .chain([&ARCHIVED_DATA])
            .map(|(url, content_type, body)| {
                serde_json::json!({
                    "request": {"method": "GET", "url": url},
                    "response": {
                        "status": 200,
                        "headers": [],
                        "content": {"mimeType": content_type, "text": body},
                    },
                })
            })
            .collect();
        let har = serde_json::json!({"log": {"entries": entries}}).to_string();

        let workspace = Workspace::new(browser);
        let saved = workspace.unpack("page.har", har.as_bytes());
        let html = workspace.open(&saved);

        assert!(html.contains("imported|fetched|/page"), "{}", html);
    });
}

#[test]
fn saves_what_it_takes_to_replay_as_warc() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<title>t</title><body><script type=module src=/app.js></script>",
            ),
            (
                "/app.js",
                "text/javascript",
                b"import {name} from './lib.js';\
                  const data = await (await fetch('/data.json')).json();\
                  document.body.textContent = name + '|' + data.value;",
            ),
            (
                "/lib.js",
                "text/javascript",
                b"export const name = 'imported';",
            ),
            ("/data.json", "application/json", b"{\"value\":\"fetched\"}"),
        ]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/"), &["--format", "warc"]);
        let warc = std::fs::read(&saved).unwrap();
        let requests = server.requests().len();

        assert!(warc.starts_with(b"WARC/1.1\r\n"));
        let unpacked = workspace.unpack("page.warc", &warc);
        let html = workspace.open(&unpacked);

        assert!(html.contains("imported|fetched"), "{}", html);
        assert_eq!(server.requests().len(), requests, "archive has reached out");
    });
}

#[test]
fn text_in_legacy_encodings_stays_readable() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html; charset=windows-1252",
                b"<title>t</title><body><p id=p>caf\xe9</p><script src=/app.js></script>",
            ),
            (
                "/app.js",
                "text/javascript; charset=windows-1252",
                b"document.body.dataset.done = document.getElementById('p').textContent \
                  === 'caf\xe9' ? 'same' : 'different';",
            ),
        ]);
        let workspace = Workspace::new(browser);
        let saved = workspace.save(&server.url("/"), &[]);
        let html = workspace.open(&saved);

        assert!(html.contains("caf\u{e9}"), "{}", html);
        assert!(html.contains("data-done=\"same\""), "{}", html);
    });
}

#[test]
fn what_got_redirected_can_be_found_where_it_was_asked_for() {
    with_each_browser(|browser| {
        let server = Server::with_responder(Box::new(|request| match request.path.as_str() {
            "/" => (
                200,
                "text/html",
                b"<title>t</title><body><script>\
                  fetch('/old.json').then((r) => r.json()).then((d) => {\
                  document.body.dataset.value = d.value; });</script>"
                    .to_vec(),
            ),
            "/old.json" => (302, "text/plain", b"/new.json".to_vec()),
            "/new.json" => (200, "application/json", b"{\"value\":\"moved\"}".to_vec()),
            _ => (404, "text/plain", vec![]),
        }));
        let workspace = Workspace::new(browser);

        for format in ["html", "warc"] {
            let saved = workspace.save(&server.url("/"), &["--format", format]);
            let saved = match format {
                "warc" => workspace.unpack("page.warc", &std::fs::read(&saved).unwrap()),
                _ => saved,
            };
            let html = workspace.open(&saved);

            assert!(
                html.contains("data-value=\"moved\""),
                "{}: {}",
                format,
                html
            );
        }
    });
}

/// Asks another origin for things it does and doesn't allow pages to have.
const CORS_APP: &str = r#"<!doctype html><title>CORS</title><div id=a></div><script>
(async () => {
  const other = 'OTHER';
  const results = [];
  const read = (url, init) => fetch(other + url, init).then((r) => r.json()).then((j) => j.name, () => 'refused');
  results.push(await read('/allowed.json'));
  results.push(await read('/denied.json'));
  results.push(await fetch(other + '/denied.json', {mode: 'no-cors'}).then((r) => r.type + r.status, () => 'refused'));
  results.push(await read('/allowed.json', {credentials: 'include'}));
  for (const path of ['/allowed-xhr.json', '/denied-xhr.json']) {
    results.push(await new Promise((done) => {
      const x = new XMLHttpRequest();
      x.open('GET', other + path);
      x.responseType = 'json';
      x.onload = () => done(x.response.name);
      x.onerror = () => done('refused');
      x.send();
    }));
  }
  a.textContent = results.join('|');
})();
</script>"#;

#[test]
fn other_origins_allow_saved_pages_what_they_allowed() {
    with_each_browser(|browser| {
        let other = Server::new(&[
            (
                "/allowed.json",
                "application/json",
                br#"{"name":"allowed"}"#,
            ),
            ("/denied.json", "application/json", br#"{"name":"denied"}"#),
            (
                "/allowed-xhr.json",
                "application/json",
                br#"{"name":"xhr"}"#,
            ),
            (
                "/denied-xhr.json",
                "application/json",
                br#"{"name":"denied"}"#,
            ),
        ]);
        other.add_header("/allowed.json", "Access-Control-Allow-Origin: *");
        other.add_header("/allowed-xhr.json", "Access-Control-Allow-Origin: *");
        let page: &'static [u8] = Box::leak(
            CORS_APP
                .replace("OTHER", &other.url(""))
                .into_bytes()
                .into_boxed_slice(),
        );
        let server = Server::new(&[("/", "text/html", page)]);
        let workspace = Workspace::new(browser);
        // "*" isn't enough for what comes with credentials
        let expected = "allowed|refused|opaque0|refused|xhr|refused";

        let live = workspace.render(&server.url("/"), &["--remove-js"]);
        assert!(live.contains(expected), "live: {}", live);

        let saved = workspace.save(&server.url("/"), &[]);
        let html = workspace.open(&saved);
        assert!(html.contains(expected), "saved: {}", html);
    });
}
