//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// What a server has been asked for: path, Origin header, and whether cookies came along.
    type Requests = Arc<Mutex<Vec<(String, Option<String>, bool)>>>;

    /// Server that sends what given function says to, for paths it gets asked for:
    /// headers, and a body.
    fn serve(
        respond: impl Fn(&str) -> (Vec<String>, String) + Send + 'static,
    ) -> (String, Requests) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests: Requests = Arc::new(Mutex::new(vec![]));
        let recorded = Arc::clone(&requests);

        thread::spawn(move || {
            for mut stream in listener.incoming().map_while(Result::ok) {
                let mut received = vec![];
                let mut buffer = [0; 4096];
                while !received.windows(4).any(|part| part == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(size) => received.extend_from_slice(&buffer[..size]),
                    }
                }
                let head = String::from_utf8_lossy(&received).to_string();
                let path = head.split(' ').nth(1).unwrap_or("").to_string();
                let header = |name: &str| {
                    head.lines()
                        .find(|line| line.to_ascii_lowercase().starts_with(&format!("{}:", name)))
                        .map(|line| line[name.len() + 1..].trim().to_string())
                };
                recorded.lock().unwrap().push((
                    path.clone(),
                    header("origin"),
                    header("cookie").is_some(),
                ));

                let (headers, body) = respond(&path);
                let mut response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n",
                    body.len()
                );
                for header in headers {
                    response.push_str(&header);
                    response.push_str("\r\n");
                }
                response.push_str("\r\n");
                response.push_str(&body);
                let _ = stream.write_all(response.as_bytes());
            }
        });

        (origin, requests)
    }

    /// Assets, allowed to be used by pages of other origins (or not) by what their names say.
    fn assets(path: &str) -> (Vec<String>, String) {
        let mut headers = vec!["Content-Type: application/octet-stream".to_string()];
        match path {
            p if p.starts_with("/allowed") => {
                headers.push("Access-Control-Allow-Origin: *".to_string())
            }
            p if p.starts_with("/twice") => {
                headers.push("Access-Control-Allow-Origin: *".to_string());
                headers.push("Access-Control-Allow-Origin: *".to_string());
            }
            _ => {}
        }

        (headers, format!("ASSET{}", path))
    }

    /// Page with given markup, at the origin it's served from, with assets at another.
    fn page(markup: &str) -> (String, Requests, Requests) {
        let (assets_origin, asset_requests) = serve(assets);
        let markup = markup.replace("ASSETS", &assets_origin);
        let (page_origin, page_requests) = serve(move |path| match path {
            "/" => (vec!["Content-Type: text/html".to_string()], markup.clone()),
            _ => assets(path),
        });

        (page_origin, page_requests, asset_requests)
    }

    fn save(url: &str, args: &[&str]) -> (String, String) {
        let directory = tempfile::tempdir().unwrap();
        let cookies = directory.path().join("cookies.txt");
        fs::write(
            &cookies,
            "# Netscape HTTP Cookie File\n127.0.0.1\tFALSE\t/\tFALSE\t4102444800\tsid\tsecret\n",
        )
        .unwrap();

        let output = crate::cli::monolith()
            .arg("-C")
            .arg(&cookies)
            .args(args)
            .arg(url)
            .output()
            .unwrap();

        (
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    #[test]
    fn fonts_from_other_origins_have_to_be_allowed() {
        let (page_origin, page_requests, asset_requests) = page(
            "<style>@font-face{font-family:a;src:url(ASSETS/allowed.woff)}\
             @font-face{font-family:b;src:url(\"ASSETS/denied.woff\")}\
             @font-face{font-family:c;src:url(/own.woff)}\
             @font-face{font-family:d;src:url(ASSETS/twice.woff)}\
             body{background:url(ASSETS/denied.png)}</style>",
        );

        let (output, errors) = save(&format!("{}/", page_origin), &["-j"]);

        assert!(output.contains("ASSET/allowed.woff"));
        assert!(output.contains("ASSET/own.woff"));
        assert!(!output.contains("ASSET/denied.woff"));
        assert!(!output.contains("ASSET/twice.woff"));
        // Backgrounds aren't up to other origins
        assert!(output.contains("ASSET/denied.png"));
        assert!(errors.contains("/denied.woff (blocked by CORS)"));
        assert!(errors.contains("/twice.woff (blocked by CORS)"));

        // Fonts get asked for the way browsers ask for them, without cookies
        let requests = asset_requests.lock().unwrap().clone();
        let font = requests
            .iter()
            .find(|(path, _, _)| path == "/allowed.woff")
            .unwrap();
        assert_eq!(font.1.as_deref(), Some(page_origin.as_str()));
        assert!(!font.2);
        let background = requests
            .iter()
            .find(|(path, _, _)| path == "/denied.png")
            .unwrap();
        assert_eq!(background.1, None);
        assert!(background.2);
        // The page's own font is the page's own
        let own = page_requests
            .lock()
            .unwrap()
            .iter()
            .find(|(path, _, _)| path == "/own.woff")
            .cloned()
            .unwrap();
        assert_eq!(own.1, None);
        assert!(own.2);
    }

    #[test]
    fn elements_with_crossorigin_have_to_be_allowed() {
        let (page_origin, _, asset_requests) = page(
            "<img id=plain src=ASSETS/denied-plain.png>\
             <img id=denied crossorigin src=ASSETS/denied.png>\
             <img id=allowed crossorigin=anonymous src=ASSETS/allowed.png>\
             <img id=credentials crossorigin=use-credentials src=ASSETS/allowed-credentials.png>",
        );

        let (output, _) = save(&format!("{}/", page_origin), &["-j"]);

        assert!(output.contains("ASSET/denied-plain.png"));
        assert!(output.contains("ASSET/allowed.png"));
        assert!(output.contains("<img id=\"denied\" crossorigin=\"\">"));
        // "*" isn't enough for what comes with credentials
        assert!(output.contains("<img id=\"credentials\" crossorigin=\"use-credentials\">"));
        let requests = asset_requests.lock().unwrap().clone();
        let with_credentials = requests
            .iter()
            .find(|(path, _, _)| path == "/allowed-credentials.png")
            .unwrap();
        assert!(with_credentials.2);
    }

    #[test]
    fn sources_go_by_what_they_are_in() {
        let (page_origin, _, _) = page(
            "<video crossorigin><source id=a src=ASSETS/denied.mp4>\
             <source id=b src=ASSETS/allowed.mp4></video>\
             <audio><source id=c src=ASSETS/denied-plain.ogg></audio>\
             <picture><source id=d srcset=ASSETS/denied.webp>\
             <img crossorigin src=ASSETS/allowed.png></picture>\
             <picture><source id=e srcset=ASSETS/denied-plain.webp><img src=ASSETS/plain.png></picture>",
        );

        let (output, errors) = save(&format!("{}/", page_origin), &["-j"]);

        // Media elements say how their sources get requested
        assert!(output.contains("<source id=\"a\">"), "{}", output);
        assert!(output.contains("ASSET/allowed.mp4"));
        assert!(output.contains("ASSET/denied-plain.ogg"));
        // Pictures go by their images
        assert!(!output.contains("ASSET/denied.webp"));
        assert!(output.contains("ASSET/allowed.png"));
        // Sets of sources are base64-encoded
        assert!(output.contains(&base64::Engine::encode(
            &base64::prelude::BASE64_STANDARD,
            "ASSET/denied-plain.webp"
        )));
        assert!(!output.contains(&base64::Engine::encode(
            &base64::prelude::BASE64_STANDARD,
            "ASSET/denied.webp"
        )));
        assert!(errors.contains("/denied.mp4 (blocked by CORS)"));
        assert!(errors.contains("/denied.webp (blocked by CORS)"));
    }

    #[test]
    fn module_scripts_have_to_be_allowed() {
        let (page_origin, _, _) = page(
            "<script type=module src=ASSETS/allowed.js></script>\
             <script type=module src=ASSETS/denied.js></script>\
             <script src=ASSETS/classic.js></script>",
        );

        let (output, _) = save(&format!("{}/", page_origin), &[]);

        assert!(output.contains("ASSET/allowed.js"));
        assert!(!output.contains("ASSET/denied.js"));
        assert!(output.contains("ASSET/classic.js"));
    }

    #[test]
    fn integrity_needs_cors() {
        let (page_origin, _, _) =
            page("<script src=ASSETS/checked.js integrity=sha256-abc></script>");

        let (output, errors) = save(&format!("{}/", page_origin), &[]);

        assert!(!output.contains("ASSET/checked.js"));
        assert!(errors.contains("/checked.js (integrity can't be checked without CORS)"));
    }

    #[test]
    fn mhtml_has_only_what_was_allowed() {
        let (page_origin, _, _) = page(
            "<style>@font-face{font-family:a;src:url(ASSETS/allowed.woff)}\
             @font-face{font-family:b;src:url(ASSETS/denied.woff)}</style>\
             <img crossorigin src=ASSETS/denied.png><img src=ASSETS/plain.png>",
        );

        let (output, errors) = save(&format!("{}/", page_origin), &["-j", "-F", "MHTML"]);

        assert!(output.contains("/allowed.woff\r\n"), "{}", output);
        assert!(output.contains("/plain.png\r\n"));
        assert!(!output.contains("/denied.woff\r\n"), "{}", output);
        assert!(!output.contains("/denied.png\r\n"), "{}", output);
        assert!(errors.contains("/denied.woff (blocked by CORS)"));
        assert!(errors.contains("/denied.png (blocked by CORS)"));
    }

    #[test]
    fn local_pages_get_what_firefox_gives_them() {
        let (assets_origin, asset_requests) = serve(assets);
        let directory = tempfile::tempdir().unwrap();
        let site = directory.path().join("site");
        fs::create_dir_all(site.join("fonts")).unwrap();
        fs::write(directory.path().join("up.woff"), "UP").unwrap();
        fs::write(site.join("same.woff"), "SAME").unwrap();
        fs::write(site.join("fonts/within.woff"), "WITHIN").unwrap();
        fs::write(
            site.join("page.html"),
            format!(
                "<style>@font-face{{font-family:a;src:url(same.woff)}}\
                 @font-face{{font-family:b;src:url(fonts/within.woff)}}\
                 @font-face{{font-family:c;src:url(../up.woff)}}\
                 @font-face{{font-family:d;src:url({0}/allowed.woff)}}\
                 @font-face{{font-family:e;src:url({0}/denied.woff)}}</style>",
                assets_origin
            ),
        )
        .unwrap();

        let output = crate::cli::monolith()
            .arg(site.join("page.html"))
            .output()
            .unwrap();
        let output = String::from_utf8_lossy(&output.stdout);

        assert!(output.contains("SAME"));
        assert!(output.contains("WITHIN"));
        assert!(!output.contains("UP"));
        assert!(output.contains("ASSET/allowed.woff"));
        assert!(!output.contains("ASSET/denied.woff"));
        // Files have no origin to tell
        let requests = asset_requests.lock().unwrap().clone();
        assert!(
            requests
                .iter()
                .all(|(_, origin, _)| origin.as_deref() == Some("null"))
        );
    }
}
