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
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// "user:secret", the way it goes into Authorization headers
    const AUTHORIZATION: &str = "Basic dXNlcjpzZWNyZXQ=";

    /// Site that shows something else to whoever logs in, along with what it's been asked for
    /// (and whether by someone who's logged in). Pages link to the site on given other origin.
    struct Site {
        origin: String,
        requests: Arc<Mutex<Vec<(String, bool)>>>,
    }

    impl Site {
        fn new(other_origin: &str) -> Site {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let origin = format!("http://{}", listener.local_addr().unwrap());
            let requests = Arc::new(Mutex::new(vec![]));
            let recorded = Arc::clone(&requests);
            let other_origin = other_origin.to_string();

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
                    let is_logged_in = head.lines().any(|line| {
                        line.to_ascii_lowercase().starts_with("authorization:")
                            && line.ends_with(AUTHORIZATION)
                    });
                    recorded.lock().unwrap().push((path.clone(), is_logged_in));

                    let who = if is_logged_in {
                        "LOGGED-IN"
                    } else {
                        "ANONYMOUS"
                    };
                    let (media_type, body) = match path.as_str() {
                        "/style.css" => ("text/css", format!("p::after{{content:\"{who}\"}}")),
                        "/settings" => (
                            "text/html",
                            format!(
                                "<title>Settings</title><link rel=\"stylesheet\" href=\"/style.css\">\
                                 <img src=\"{other_origin}/image.png\"><a href=\"/about\">about</a><p>{who}</p>"
                            ),
                        ),
                        "/about" => (
                            "text/html",
                            "<title>About</title><link rel=\"stylesheet\" href=\"/style.css\"><p>About</p>"
                                .to_string(),
                        ),
                        _ => ("image/png", who.to_string()),
                    };
                    let _ = stream.write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            media_type,
                            body.len(),
                            body
                        )
                        .as_bytes(),
                    );
                }
            });

            Site { origin, requests }
        }

        /// URL as someone who's logged in.
        fn logged_in(&self, path: &str) -> String {
            format!(
                "{}{}",
                self.origin.replacen("http://", "http://user:secret@", 1),
                path
            )
        }

        fn url(&self, path: &str) -> String {
            format!("{}{}", self.origin, path)
        }

        fn requests(&self) -> Vec<(String, bool)> {
            self.requests.lock().unwrap().clone()
        }
    }

    fn save(directory: &Path, args: &[&str]) {
        crate::cli::monolith_writing_files()
            .current_dir(directory)
            .arg("-q")
            .args(args)
            .assert()
            .success();
    }

    fn read(directory: &Path, name: &str) -> String {
        fs::read_to_string(directory.join(name)).unwrap()
    }

    #[test]
    fn assets_get_retrieved_as_whoever_the_page_is_retrieved_as() {
        let (other, saved) = (Site::new(""), tempfile::tempdir().unwrap());
        let site = Site::new(&other.origin);

        save(saved.path(), &[&site.logged_in("/settings")]);

        let settings = read(saved.path(), "Settings.html");
        assert!(settings.contains("LOGGED-IN\"}"));
        assert!(
            site.requests()
                .iter()
                .all(|(_, is_logged_in)| *is_logged_in)
        );
        // Credentials are for their own site only
        assert_eq!(other.requests(), vec![("/image.png".to_string(), false)]);
        assert!(!settings.contains("secret"));
        assert!(!settings.contains("user@"));
    }

    #[test]
    fn logging_in_makes_for_other_assets() {
        for order in [[0, 1], [1, 0]] {
            let (site, saved) = (Site::new(""), tempfile::tempdir().unwrap());
            let targets = [site.logged_in("/settings"), site.url("/about")];

            save(saved.path(), &[&targets[order[0]], &targets[order[1]]]);

            assert!(read(saved.path(), "Settings.html").contains("LOGGED-IN\"}"));
            assert!(read(saved.path(), "About.html").contains("ANONYMOUS\"}"));
            // Each one got retrieved once, as the one it was for
            let mut requests = site.requests();
            requests.retain(|(path, _)| path != "/favicon.ico" && path != "/image.png");
            requests.sort();
            assert_eq!(
                requests,
                vec![
                    ("/about".to_string(), false),
                    ("/settings".to_string(), true),
                    ("/style.css".to_string(), false),
                    ("/style.css".to_string(), true),
                ]
            );
            // Links lead to files saved as whoever's linking to them
            assert!(read(saved.path(), "Settings.html").contains("<a href=\"About.html\">"));
        }
    }

    #[test]
    fn same_page_as_someone_else_is_some_other_page() {
        let (site, saved) = (Site::new(""), tempfile::tempdir().unwrap());

        save(
            saved.path(),
            &[&site.logged_in("/settings"), &site.url("/settings")],
        );

        assert!(read(saved.path(), "Settings.html").contains("<p>LOGGED-IN</p>"));
        assert!(read(saved.path(), "Settings (2).html").contains("<p>ANONYMOUS</p>"));
    }

    #[test]
    fn archives_keep_no_credentials() {
        for format in ["MHTML", "WARC"] {
            let (site, saved) = (Site::new(""), tempfile::tempdir().unwrap());

            save(saved.path(), &["-F", format, &site.logged_in("/settings")]);

            let name = format!("Settings.{}", format.to_lowercase());
            let archive = fs::read(saved.path().join(&name)).unwrap();
            let archive = String::from_utf8_lossy(&archive);
            assert!(archive.contains("LOGGED-IN"), "{}", format);
            assert!(!archive.contains("secret"), "{}", format);
            assert!(!archive.contains(AUTHORIZATION), "{}", format);
            assert!(!archive.contains("user@"), "{}", format);
        }
    }
}
