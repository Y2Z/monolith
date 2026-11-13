#[cfg(unix)]
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::thread::sleep;
#[cfg(unix)]
use std::time::{Duration, Instant};

use crate::support::{Server, Workspace, assert_success, with_each_browser};

fn website(host: &str) -> Server {
    let page = format!(
        "<!doctype html><title>t</title><img src=/own.png>\
         <img src=http://{}/tracker.png><script>fetch('http://{}/api')</script>",
        host, host
    );

    Server::with_responder(Box::new(move |request| match request.path.as_str() {
        "/" => (200, "text/html", page.clone().into_bytes()),
        _ => (200, "image/png", b"\x89PNG".to_vec()),
    }))
}

#[test]
fn does_not_contact_blacklisted_domains() {
    with_each_browser(|browser| {
        let server = website("localhost:1");
        let host = "ads.invalid";
        let html = Workspace::new(browser).render(&server.url("/"), &["--remove-js", "-d", host]);

        assert!(html.contains("data:image/png"));

        // Requests of the page itself are what's being kept away from everything else
        let blocked_server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><script>const i=new Image();\
                  i.src=location.href.replace('127.0.0.1','localhost')+'tracker.png'</script>",
            ),
            ("/tracker.png", "image/png", b"\x89PNG"),
        ]);
        Workspace::new(browser).render(
            &blocked_server.url("/"),
            &["--remove-js", "-d", "localhost"],
        );

        assert!(!blocked_server.was_asked_for("/tracker.png"));
    });
}

#[test]
fn only_contacts_whitelisted_domains() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><script>const i=new Image();\
                  i.src=location.href.replace('127.0.0.1','localhost')+'tracker.png'</script>",
            ),
            ("/tracker.png", "image/png", b"\x89PNG"),
        ]);

        Workspace::new(browser).render(&server.url("/"), &["--remove-js", "-W", "-d", "127.0.0.1"]);

        assert!(server.was_asked_for("/"));
        assert!(!server.was_asked_for("/tracker.png"));
    });
}

#[test]
fn does_not_request_what_got_excluded() {
    with_each_browser(|browser| {
        let server = Server::new(&[
            (
                "/",
                "text/html",
                b"<!doctype html><title>t</title><script>new Image().src='/a.png'</script>",
            ),
            ("/a.png", "image/png", b"\x89PNG"),
        ]);

        Workspace::new(browser).render(&server.url("/"), &["--remove-js", "--remove-images"]);

        assert!(!server.was_asked_for("/a.png"));
    });
}

#[test]
fn sends_no_cookies_of_its_own() {
    with_each_browser(|browser| {
        let server = Server::new(&[("/", "text/html", b"<title>t</title>")]);

        Workspace::new(browser).render(&server.url("/"), &[]);

        assert!(
            server
                .requests()
                .iter()
                .all(|request| request.cookie.is_empty())
        );
    });
}

#[test]
fn sends_specified_cookies_and_user_agent() {
    with_each_browser(|browser| {
        let server = Server::new(&[("/", "text/html", b"<title>t</title>")]);
        let workspace = Workspace::new(browser);
        let cookies = workspace.directory.path().join("cookies.txt");
        std::fs::write(
            &cookies,
            "# Netscape HTTP Cookie File\n\
             127.0.0.1\tFALSE\t/\tFALSE\t0\tfresh\tyes\n\
             127.0.0.1\tFALSE\t/\tFALSE\t1000\texpired\tno\n",
        )
        .unwrap();

        workspace.render(
            &server.url("/"),
            &["-u", "Agent/1.0", "-C", cookies.to_str().unwrap()],
        );

        let requests = server.requests();
        let request = requests.iter().find(|r| r.path == "/").unwrap();
        assert_eq!(request.cookie, "fresh=yes");
        assert_eq!(request.user_agent, "Agent/1.0");
    });
}

#[test]
fn leaves_typed_passwords_out() {
    with_each_browser(|browser| {
        let server = Server::new(&[(
            "/",
            "text/html",
            b"<!doctype html><title>t</title><input id=p type=password><input id=n>\
              <script>p.value='hunter2';n.value='Typed name'</script>",
        )]);

        let html = Workspace::new(browser).render(&server.url("/"), &["--remove-js"]);

        assert!(html.contains("Typed name"));
        assert!(!html.contains("value=\"hunter2\""));
    });
}

#[test]
fn leaves_nothing_behind() {
    with_each_browser(|browser| {
        let server = Server::new(&[("/", "text/html", b"<title>t</title>")]);
        let workspace = Workspace::new(browser);

        workspace.render(&server.url("/"), &[]);

        assert!(!workspace.has_leftovers());
    });
}

#[test]
fn does_not_let_browsers_contact_anything_else() {
    with_each_browser(|browser| {
        // Everything browsers do behind the scenes goes through this proxy
        let proxy = Server::new(&[]);
        let server = Server::new(&[("/", "text/html", b"<title>t</title>")]);
        let proxy_url = format!("http://{}", proxy.address);

        let output = Workspace::new(browser)
            .monolith()
            .env("http_proxy", &proxy_url)
            .env("https_proxy", &proxy_url)
            .env("HTTP_PROXY", &proxy_url)
            .env("HTTPS_PROXY", &proxy_url)
            .env("no_proxy", "127.0.0.1")
            .env("NO_PROXY", "127.0.0.1")
            .arg(server.url("/"))
            .output()
            .unwrap();

        assert_success(&output);
        assert_eq!(
            proxy
                .requests()
                .iter()
                .map(|r| r.path.as_str())
                .collect::<Vec<_>>(),
            Vec::<&str>::new()
        );
    });
}

#[cfg(unix)]
#[test]
fn cleans_up_after_being_interrupted() {
    with_each_browser(|browser| {
        let server = Server::with_responder(Box::new(|request| {
            if request.path == "/slow" {
                sleep(Duration::from_secs(30));
            }
            (
                200,
                "text/html",
                b"<!doctype html><title>t</title><script>fetch('/slow')</script>".to_vec(),
            )
        }));
        let workspace = Workspace::new(browser);
        let mut monolith = workspace
            .monolith()
            .arg(server.url("/"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(30);
        while !server.was_asked_for("/slow") && Instant::now() < deadline {
            sleep(Duration::from_millis(100));
        }
        Command::new("kill")
            .args(["-INT", &monolith.id().to_string()])
            .status()
            .unwrap();

        assert_eq!(monolith.wait().unwrap().code(), Some(130));
        sleep(Duration::from_millis(300));
        assert!(
            !workspace.has_leftovers(),
            "{:?}",
            std::fs::read_dir(workspace.temporary_directory.path())
                .unwrap()
                .flatten()
                .map(|entry| entry.file_name())
                .collect::<Vec<_>>()
        );
    });
}
