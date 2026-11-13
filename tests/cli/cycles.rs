//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    use assert_cmd::prelude::*;

    /// Serves pages with assets that redirect into cycles; returns the URL of the given page.
    fn serve_cycles(page_path: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("http://{address}");
        let url = format!("{origin}{page_path}");

        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };

                let origin = origin.clone();
                thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

                    let mut request: Vec<u8> = Vec::new();
                    let mut byte = [0];
                    while !request.ends_with(b"\r\n\r\n") && stream.read_exact(&mut byte).is_ok() {
                        request.push(byte[0]);
                    }

                    let request_line = String::from_utf8_lossy(&request);
                    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
                    let (status, content_type, location, body): (&str, &str, &str, String) =
                        match path {
                            "/index.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                "<link rel=\"stylesheet\" href=\"/alias.css\"><p>redirect cycle</p>"
                                    .to_string(),
                            ),
                            "/alias.css" => (
                                "302 Found",
                                "text/css",
                                "Location: /site.css\r\n",
                                String::new(),
                            ),
                            "/site.css" => (
                                "200 OK",
                                "text/css",
                                "",
                                "@import url(\"/alias.css\");body{color:#654321}".to_string(),
                            ),
                            "/frame-index.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                "<iframe src=\"/frame-alias.html\"></iframe><p>frame redirect cycle</p>"
                                    .to_string(),
                            ),
                            "/frame-alias.html" => (
                                "302 Found",
                                "text/html",
                                "Location: /frame.html\r\n",
                                String::new(),
                            ),
                            "/frame.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                "<iframe src=\"/frame-alias.html\"></iframe><p>frame marker</p>"
                                    .to_string(),
                            ),
                            "/svg-index.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                "<svg><use href=\"/svg-alias.svg\"></use></svg><p>svg redirect cycle</p>"
                                    .to_string(),
                            ),
                            "/svg-alias.svg" => (
                                "302 Found",
                                "image/svg+xml",
                                "Location: /icon.svg\r\n",
                                String::new(),
                            ),
                            "/icon.svg" => (
                                "200 OK",
                                "image/svg+xml",
                                "",
                                "<svg xmlns=\"http://www.w3.org/2000/svg\"><use href=\"/svg-alias.svg\"></use><text>icon marker</text></svg>"
                                    .to_string(),
                            ),
                            "/svg-canonical-index.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                "<svg><use href=\"/canonical-alias.svg\"></use></svg><p>svg canonical cycle</p>"
                                    .to_string(),
                            ),
                            "/svg-canonical-image-index.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                "<img src=\"/canonical-alias.svg\"><svg><use href=\"/canonical-alias.svg\"></use></svg><p>svg canonical after image</p>"
                                    .to_string(),
                            ),
                            "/canonical-alias.svg" => (
                                "302 Found",
                                "image/svg+xml",
                                "Location: /canonical.svg\r\n",
                                String::new(),
                            ),
                            "/canonical.svg" => (
                                "200 OK",
                                "image/svg+xml",
                                "",
                                "<svg xmlns=\"http://www.w3.org/2000/svg\"><use href=\"/canonical.svg\"></use><text>canonical marker</text></svg>"
                                    .to_string(),
                            ),
                            "/svg-both-index.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                "<svg><use href=\"/svg-both-alias.svg#sym\" xlink:href=\"/icon-sym.svg#sym\"></use></svg><p>svg both attributes</p>"
                                    .to_string(),
                            ),
                            "/svg-both-alias.svg" => (
                                "302 Found",
                                "image/svg+xml",
                                "Location: /icon-sym.svg\r\n",
                                String::new(),
                            ),
                            "/icon-sym.svg" => (
                                "200 OK",
                                "image/svg+xml",
                                "",
                                "<svg xmlns=\"http://www.w3.org/2000/svg\"><symbol id=\"sym\"><text>symbol marker</text></symbol></svg>"
                                    .to_string(),
                            ),
                            "/base-index.html" => (
                                "302 Found",
                                "text/html",
                                "Location: /base-final.html\r\n",
                                String::new(),
                            ),
                            "/base-final.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                "<base href=\"/base-index.html\"><link rel=\"stylesheet\" href=\"base-style.css\"><iframe src=\"/base-index.html\"></iframe><p>base redirect marker</p>"
                                    .to_string(),
                            ),
                            "/base-style.css" => (
                                "200 OK",
                                "text/css",
                                "",
                                "body{color:#abcdef}".to_string(),
                            ),
                            "/cred-index.html" => (
                                "200 OK",
                                "text/html",
                                "",
                                format!(
                                    "<p>credentials marker</p><img src=\"http://user:pass@{}/pixel.svg\"><iframe src=\"/cred-index.html\"></iframe>",
                                    origin.trim_start_matches("http://")
                                ),
                            ),
                            "/pixel.svg" => (
                                "200 OK",
                                "image/svg+xml",
                                "",
                                "<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_string(),
                            ),
                            _ => ("404 Not Found", "text/plain", "", String::new()),
                        };

                    let response = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{location}Connection: close\r\n\r\n",
                        body.len(),
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(body.as_bytes());
                });
            }
        });

        url
    }

    #[test]
    fn css_import_cycle_does_not_hang() {
        let out = crate::cli::monolith()
            .arg("tests/_data_/cycles/css/index.html")
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        assert!(output.contains("color: #123456"), "{output}");
        assert!(output.contains("data:text/css"), "{output}");
    }

    #[test]
    fn redirected_stylesheet_import_cycle_does_not_hang() {
        let url = serve_cycles("/index.html");

        let out = crate::cli::monolith()
            .arg("-q")
            .arg(&url)
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        assert!(output.contains("color:#654321"), "{output}");
    }

    #[test]
    fn redirected_frame_cycle_does_not_hang() {
        let url = serve_cycles("/frame-index.html");

        let out = crate::cli::monolith()
            .arg("-q")
            .arg(&url)
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        assert!(output.contains("frame redirect cycle"), "{output}");
        assert!(output.contains("data:text/html"), "{output}");
    }

    #[test]
    fn redirected_svg_use_cycle_does_not_hang() {
        let url = serve_cycles("/svg-index.html");

        let out = crate::cli::monolith()
            .arg("-q")
            .arg(&url)
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        assert!(output.contains("svg redirect cycle"), "{output}");
        assert!(output.contains("icon marker"), "{output}");
        assert_eq!(
            output.matches("href=\"/svg-alias.svg\"").count(),
            1,
            "{output}"
        );
    }

    #[test]
    fn redirected_svg_use_canonical_cycle_does_not_hang() {
        let url = serve_cycles("/svg-canonical-index.html");

        let out = crate::cli::monolith()
            .arg("-q")
            .arg(&url)
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        // The SVG refers to itself by its canonical URL, which prefetching must not hide
        assert!(output.contains("svg canonical cycle"), "{output}");
        assert_eq!(output.matches("canonical marker").count(), 1, "{output}");
    }

    #[test]
    fn iframe_cycle_does_not_hang() {
        let out = crate::cli::monolith()
            .arg("tests/_data_/cycles/iframe/index.html")
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        assert!(output.contains("Iframe cycle"), "{output}");
        assert!(output.contains("data:text/html;charset=utf-8,"), "{output}");
    }

    #[test]
    fn svg_use_cycle_does_not_hang() {
        let out = crate::cli::monolith()
            .arg("tests/_data_/cycles/svg/index.html")
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        assert!(output.contains("SVG use cycle"), "{output}");
        assert!(output.contains("icon marker"), "{output}");
    }

    #[test]
    fn base_url_matching_redirected_target_still_walks_document() {
        let url = serve_cycles("/base-index.html");

        let out = crate::cli::monolith()
            .arg("-q")
            .arg(&url)
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        // The document got processed under the URL it was requested by, which is not a cycle
        assert!(output.contains("base redirect marker"), "{output}");
        assert!(output.contains("color:#abcdef"), "{output}");
        assert!(output.contains("<style>"), "{output}");
        // Referring to the URL it was requested by is still a cycle
        assert!(output.contains("data:text/html"), "{output}");
    }

    #[test]
    fn redirected_svg_use_with_both_hrefs_embeds_once() {
        let url = serve_cycles("/svg-both-index.html");

        let out = crate::cli::monolith()
            .arg("-q")
            .arg(&url)
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        assert!(output.contains("svg both attributes"), "{output}");
        assert_eq!(output.matches("symbol marker").count(), 1, "{output}");
        assert_eq!(output.matches("href=\"#sym\"").count(), 2, "{output}");
        assert!(!output.contains("svg-both-alias.svg"), "{output}");
        assert!(!output.contains("icon-sym.svg"), "{output}");
    }

    #[test]
    fn redirected_svg_use_canonical_cycle_after_image_does_not_hang() {
        let url = serve_cycles("/svg-canonical-image-index.html");

        let out = crate::cli::monolith()
            .arg("-q")
            .arg(&url)
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        // Prefetching the image must not hide where the alias leads
        assert!(output.contains("svg canonical after image"), "{output}");
        assert_eq!(
            output.matches("<text>canonical marker</text>").count(),
            1,
            "{output}"
        );
    }

    #[test]
    fn credentials_found_while_walking_do_not_hide_cycles() {
        let url = serve_cycles("/cred-index.html");

        let out = crate::cli::monolith()
            .arg("-q")
            .arg(&url)
            .timeout(Duration::from_secs(5))
            .output()
            .unwrap();

        let output = crate::cli::without_metadata(&out.stdout);
        out.assert().success();

        // Credentials taken from the image URL must not make the page look like a new one
        assert_eq!(output.matches("credentials marker").count(), 1, "{output}");
    }
}
