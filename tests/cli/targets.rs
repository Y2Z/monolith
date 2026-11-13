//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use std::fs;
    use std::path::Path;
    use std::process::Output;

    use tempfile::TempDir;

    /// Pages that link to each other, and to something that doesn't get saved.
    fn site() -> TempDir {
        let site = tempfile::tempdir().unwrap();
        fs::write(
            site.path().join("a.html"),
            "<html><head><title>Page A</title></head><body>\
             <a href=\"b.html\">B</a><a href=\"b.html#x\">X</a><a href=\"c.html\">C</a>\
             <a href=\"#top\">top</a></body></html>",
        )
        .unwrap();
        fs::write(
            site.path().join("b.html"),
            "<html><head><title>Page B</title></head><body id=\"x\"><a href=\"a.html\">A</a></body></html>",
        )
        .unwrap();
        fs::write(site.path().join("untitled.html"), "<p>No title</p>").unwrap();

        site
    }

    /// Runs monolith the way it runs by default, saving into given directory.
    fn run(directory: &Path, args: &[&str]) -> Output {
        assert_cmd::cargo_bin_cmd!(env!("CARGO_PKG_NAME"))
            .current_dir(directory)
            .args(args)
            .output()
            .unwrap()
    }

    fn target(site: &TempDir, name: &str) -> String {
        site.path().join(name).display().to_string()
    }

    fn read(directory: &Path, name: &str) -> String {
        fs::read_to_string(directory.join(name)).unwrap()
    }

    fn names(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn documents_get_saved_into_files_named_after_them() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(saved.path(), &["-q", &target(&site, "a.html")]);

        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(names(saved.path()), vec!["Page A.html"]);
    }

    #[test]
    fn untitled_documents_get_named_after_their_urls() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(saved.path(), &["-q", &target(&site, "untitled.html")]);

        assert!(output.status.success());
        assert_eq!(names(saved.path()), vec!["untitled.html"]);
    }

    #[test]
    fn several_targets_link_to_each_other() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &["-q", &target(&site, "a.html"), &target(&site, "b.html")],
        );

        assert!(output.status.success());
        assert_eq!(names(saved.path()), vec!["Page A.html", "Page B.html"]);
        let a = read(saved.path(), "Page A.html");
        assert!(a.contains("<a href=\"Page%20B.html\">B</a>"));
        assert!(a.contains("<a href=\"Page%20B.html#x\">X</a>"));
        assert!(a.contains("<a href=\"#top\">top</a>"));
        // What doesn't get saved stays where it is
        assert!(a.contains("c.html\">C</a>"));
        assert!(a.contains("<a href=\"file://"));
        assert!(read(saved.path(), "Page B.html").contains("<a href=\"Page%20A.html\">A</a>"));
    }

    #[test]
    fn links_lead_to_numbered_files() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());
        fs::write(saved.path().join("Page A.html"), "already there").unwrap();

        let output = run(
            saved.path(),
            &["-q", &target(&site, "a.html"), &target(&site, "b.html")],
        );

        assert!(output.status.success());
        assert_eq!(read(saved.path(), "Page A.html"), "already there");
        assert!(
            read(saved.path(), "Page B.html").contains("<a href=\"Page%20A%20(2).html\">A</a>")
        );
    }

    #[test]
    fn same_target_gets_saved_once() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &["-q", &target(&site, "a.html"), &target(&site, "a.html")],
        );

        assert!(output.status.success());
        assert_eq!(names(saved.path()), vec!["Page A.html"]);
    }

    #[test]
    fn failing_target_does_not_stop_others() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &[&target(&site, "a.html"), &site.path().display().to_string()],
        );

        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(&format!("Error: {}: local target", site.path().display()))
        );
        assert_eq!(names(saved.path()), vec!["Page A.html"]);
    }

    #[test]
    fn several_documents_do_not_go_to_stdout() {
        let site = site();

        let output = run(
            site.path(),
            &[
                "-o",
                "-",
                &target(&site, "a.html"),
                &target(&site, "b.html"),
            ],
        );

        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Error: several documents can't all go to STDOUT, they can as WARC\n"
        );
    }

    #[test]
    fn warc_takes_all_targets() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &[
                "-q",
                "-F",
                "WARC",
                &target(&site, "a.html"),
                &target(&site, "b.html"),
            ],
        );

        assert!(output.status.success());
        assert_eq!(names(saved.path()), vec!["Page A.warc"]);
        let warc = read(saved.path(), "Page A.warc");
        // Each target starts a part of its own, the way concatenated archives do
        assert_eq!(warc.matches("WARC-Type: warcinfo\r\n").count(), 2);
        assert!(warc.contains("<title>Page A</title>"));
        assert!(warc.contains("<title>Page B</title>"));
    }

    #[test]
    fn files_named_after_urls() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &[
                "-q",
                "-o",
                "%url%.%extension%",
                &target(&site, "a.html"),
                &target(&site, "b.html"),
            ],
        );

        assert!(output.status.success());
        let names = names(saved.path());
        assert_eq!(names.len(), 2);
        assert!(names[0].ends_with("--a.html.html"), "{:?}", names);
        assert!(names[1].ends_with("--b.html.html"), "{:?}", names);
        // Links lead to whatever files got called
        let link = format!("<a href=\"{}\">B</a>", names[1].replace(' ', "%20"));
        assert!(read(saved.path(), &names[0]).contains(&link));
    }

    #[test]
    fn directories_get_files_named_the_default_way() {
        for (pattern, expected) in [
            ("saved/", "saved/Page A.html"),
            ("saved", "saved/Page A.html"),
            ("by-type/%extension%/", "by-type/html/Page A.html"),
            ("named.HTML", "named.HTML"),
        ] {
            let (site, saved) = (site(), tempfile::tempdir().unwrap());

            let output = run(
                saved.path(),
                &["-q", "-o", pattern, &target(&site, "a.html")],
            );

            assert!(output.status.success(), "{}", pattern);
            assert!(
                saved.path().join(expected).is_file(),
                "{} -> {}",
                pattern,
                expected
            );
        }
    }

    #[test]
    fn files_named_after_urls_in_directories() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &["-q", "-o", "deep/er/%url%.%ext%", &target(&site, "a.html")],
        );

        // Where temporary directories really are differs from what they're called (symlinks on
        // macOS, short names on Windows), only how the name ends is certain
        assert!(output.status.success());
        let names = names(&saved.path().join("deep/er"));
        assert_eq!(names.len(), 1, "{:?}", names);
        assert!(names[0].ends_with("--a.html.htm"), "{:?}", names);
    }

    /// Site where /old redirects to /b, along with how many times each path got asked for.
    fn redirecting_site() -> (String, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        let recorded = std::sync::Arc::clone(&requests);
        std::thread::spawn(move || {
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
                recorded.lock().unwrap().push(path.clone());
                let response = match path.as_str() {
                    "/old" => "HTTP/1.1 301 Moved Permanently\r\nLocation: /b\r\n\
                               Content-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_string(),
                    _ => {
                        let body = format!("<title>{}</title>", path);
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\
                             Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                    }
                };
                let _ = stream.write_all(response.as_bytes());
            }
        });

        (origin, requests)
    }

    #[test]
    fn warc_has_everything_once() {
        for order in [["/a", "/b", "/old"], ["/old", "/b", "/a"]] {
            let ((origin, _), saved) = (redirecting_site(), tempfile::tempdir().unwrap());
            let targets: Vec<String> = order
                .iter()
                .map(|path| format!("{}{}", origin, path))
                .collect();

            let output = run(
                saved.path(),
                &[
                    "-q",
                    "-F",
                    "WARC",
                    "-o",
                    "site.warc",
                    &targets[0],
                    &targets[1],
                    &targets[2],
                ],
            );

            assert!(output.status.success());
            let warc = fs::read(saved.path().join("site.warc")).unwrap();
            let warc = String::from_utf8_lossy(&warc);
            for path in ["/a", "/b", "/old"] {
                let uri = format!("WARC-Target-URI: {}{}\r\n", origin, path);
                let responses = warc
                    .split("WARC/1.1\r\n")
                    .filter(|record| record.contains("WARC-Type: response\r\n"))
                    .filter(|record| record.contains(&uri))
                    .count();
                assert_eq!(responses, 1, "{:?}: {}", order, path);
            }
        }
    }

    #[test]
    fn output_option_can_only_be_given_once() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &["-o", "first.html", "-o", "-", &target(&site, "a.html")],
        );

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("the argument '--output <result.html>' cannot be used multiple times")
        );
        assert!(names(saved.path()).is_empty());
    }
}
