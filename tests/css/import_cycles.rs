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
    use std::thread;

    use url::Url;

    use monolith::core::MonolithOptions;
    use monolith::css::embed_css;
    use monolith::session::Session;

    /// Stylesheets to import, as files: names and contents.
    fn stylesheets(files: &[(&str, &str)]) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        for (name, css) in files {
            fs::write(directory.path().join(name), css).unwrap();
        }
        directory
    }

    fn embed(directory: &Path, css: &str) -> String {
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session = Session::new(None, None, options);
        let url = Url::from_file_path(directory.join("index.html")).unwrap();

        embed_css(&mut session, &url, css)
    }

    #[test]
    fn stylesheet_importing_itself() {
        let directory = stylesheets(&[(
            "site.css",
            "@import url(\"site.css\");\nbody{color:#123456}",
        )]);

        let result = embed(directory.path(), "@import \"site.css\";");

        // What it has in it stays, importing itself is what gets ignored
        assert!(result.contains("body{color:%23123456}"));
        assert!(result.contains("@import url(%22data:text/css,%22);"));
        assert_eq!(result.matches("color:%23123456").count(), 1);
    }

    #[test]
    fn stylesheets_importing_each_other() {
        let directory = stylesheets(&[
            ("a.css", "@import \"b.css\";\na{color:red}"),
            ("b.css", "@import url(c.css);\nb{color:blue}"),
            ("c.css", "@import \"a.css\";\nc{color:green}"),
        ]);

        let result = embed(directory.path(), "@import \"a.css\";");

        for rule in ["a{color:red}", "b{color:blue}", "c{color:green}"] {
            assert_eq!(result.matches(rule).count(), 1, "{}", rule);
        }
    }

    #[test]
    fn same_stylesheet_imported_more_than_once() {
        let directory = stylesheets(&[
            ("a.css", "@import \"d.css\";"),
            ("b.css", "@import \"d.css\";"),
            ("d.css", "d{color:green}"),
        ]);

        let result = embed(directory.path(), "@import \"a.css\";\n@import url(b.css);");

        // Not a cycle, browsers embed it each time too
        assert_eq!(result.matches("d{color:green}").count(), 2);
    }

    #[test]
    fn fragments_make_no_difference() {
        let directory = stylesheets(&[("site.css", "@import \"site.css#again\";\ns{color:red}")]);

        let result = embed(directory.path(), "@import \"site.css#first\";");

        assert_eq!(result.matches("s{color:red}").count(), 1);
    }

    #[test]
    fn imports_go_only_so_deep() {
        let files: Vec<(String, String)> = (0..100)
            .map(|level| {
                (
                    format!("{}.css", level),
                    format!("@import \"{}.css\";\nl{}{{color:red}}", level + 1, level),
                )
            })
            .collect();
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(name, css)| (name.as_str(), css.as_str()))
            .collect();
        let directory = stylesheets(&files);

        let result = embed(directory.path(), "@import \"0.css\";");

        assert!(result.contains("l31{color:red}"));
        assert!(!result.contains("l32{color:red}"));
    }

    #[test]
    fn redirects_into_cycles() {
        // /a.css redirects to /b.css, which imports /a.css
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
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
                let response = match String::from_utf8_lossy(&received).split(' ').nth(1) {
                    Some("/a.css") => "HTTP/1.1 302 Found\r\nLocation: /b.css\r\n\
                                       Content-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_string(),
                    _ => {
                        let body = "@import \"a.css\";\nb{color:blue}";
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/css\r\n\
                             Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                    }
                };
                let _ = stream.write_all(response.as_bytes());
            }
        });
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session = Session::new(None, None, options);
        let url = Url::parse(&format!("{}/", origin)).unwrap();

        let result = embed_css(&mut session, &url, "@import \"a.css\";");

        assert_eq!(result.matches("b{color:blue}").count(), 1);
        assert!(result.contains("data:text/css,"));
    }
}
