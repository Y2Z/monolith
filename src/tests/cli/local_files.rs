//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use assert_cmd::prelude::*;
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use url::Url;

    #[test]
    fn local_file_target_input_relative_target_path() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let cwd_normalized: String =
            str!(env::current_dir().unwrap().to_str().unwrap()).replace("\\", "/");
        let out = cmd
            .arg("-M")
            .arg(if cfg!(windows) {
                "src\\tests\\data\\basic\\local-file.html"
            } else {
                "src/tests/data/basic/local-file.html"
            })
            .output()
            .unwrap();
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

        // STDOUT should contain HTML from the local file
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "\
            <!DOCTYPE html><html lang=\"en\"><head>\n  \
            <meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n  \
            <title>Local HTML file</title>\n  \
            <link href=\"data:text/css;base64,Ym9keSB7CiAgICBiYWNrZ3JvdW5kLWNvbG9yOiAjMDAwOwogICAgY29sb3I6ICNmZmY7Cn0K\" rel=\"stylesheet\" type=\"text/css\">\n  \
            <link rel=\"stylesheet\" type=\"text/css\">\n</head>\n\n<body>\n  \
            <img alt=\"\">\n  \
            <a href=\"file://local-file.html/\">Tricky href</a>\n  \
            <a href=\"https://github.com/Y2Z/monolith\">Remote URL</a>\n  \
            <script src=\"data:application/javascript;base64,ZG9jdW1lbnQuYm9keS5zdHlsZS5iYWNrZ3JvdW5kQ29sb3IgPSAiZ3JlZW4iOwpkb2N1bWVudC5ib2R5LnN0eWxlLmNvbG9yID0gInJlZCI7Cg==\"></script>\n\n\n\n\
            </body></html>\n\
            "
        );

        // STDERR should contain list of retrieved file URLs, two missing
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file}{cwd}/src/tests/data/basic/local-file.html\n \
                {file}{cwd}/src/tests/data/basic/local-style.css\n \
                {file}{cwd}/src/tests/data/basic/local-style-does-not-exist.css (not found)\n \
                {file}{cwd}/src/tests/data/basic/monolith.png (not found)\n \
                {file}{cwd}/src/tests/data/basic/local-script.js\n\
                ",
                file = file_url_protocol,
                cwd = cwd_normalized
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn local_file_target_input_absolute_target_path() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let path_html: &Path = Path::new("src/tests/data/basic/local-file.html");

        let out = cmd
            .arg("-M")
            .arg("-Ijci")
            .arg(path_html.as_os_str())
            .output()
            .unwrap();

        // STDOUT should contain HTML from the local file
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            format!(
                "\
                <!DOCTYPE html><html lang=\"en\"><head>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-inline' data:; style-src 'none'; script-src 'none'; img-src data:;\"></meta>\n  \
                <meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n  \
                <title>Local HTML file</title>\n  \
                <link rel=\"stylesheet\" type=\"text/css\">\n  \
                <link rel=\"stylesheet\" type=\"text/css\">\n</head>\n\n<body>\n  \
                <img src=\"{empty_image}\" alt=\"\">\n  \
                <a href=\"file://local-file.html/\">Tricky href</a>\n  \
                <a href=\"https://github.com/Y2Z/monolith\">Remote URL</a>\n  \
                <script></script>\n\n\n\n\
                </body></html>\n\
                ",
                empty_image = empty_image!()
            )
        );

        // STDERR should contain only the target file
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "{file_url_html}\n",
                file_url_html = Url::from_file_path(fs::canonicalize(&path_html).unwrap())
                    .unwrap()
                    .into_string(),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn local_file_url_target_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let cwd_normalized: String =
            str!(env::current_dir().unwrap().to_str().unwrap()).replace("\\", "/");
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };
        let out = cmd
            .arg("-M")
            .arg("-cji")
            .arg(if cfg!(windows) {
                format!(
                    "{file}{cwd}/src/tests/data/basic/local-file.html",
                    file = file_url_protocol,
                    cwd = cwd_normalized,
                )
            } else {
                format!(
                    "{file}{cwd}/src/tests/data/basic/local-file.html",
                    file = file_url_protocol,
                    cwd = cwd_normalized,
                )
            })
            .output()
            .unwrap();

        // STDOUT should contain HTML from the local file
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            format!(
                "\
                <!DOCTYPE html><html lang=\"en\"><head>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none'; script-src 'none'; img-src data:;\"></meta>\n  \
                <meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n  \
                <title>Local HTML file</title>\n  \
                <link rel=\"stylesheet\" type=\"text/css\">\n  \
                <link rel=\"stylesheet\" type=\"text/css\">\n</head>\n\n<body>\n  \
                <img src=\"{empty_image}\" alt=\"\">\n  \
                <a href=\"file://local-file.html/\">Tricky href</a>\n  \
                <a href=\"https://github.com/Y2Z/monolith\">Remote URL</a>\n  \
                <script></script>\n\n\n\n\
                </body></html>\n\
                ",
                empty_image = empty_image!()
            )
        );

        // STDERR should contain list of retrieved file URLs
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "{file}{cwd}/src/tests/data/basic/local-file.html\n",
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn embed_file_url_local_asset_within_style_attribute() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let path_html: &Path = Path::new("src/tests/data/svg/index.html");
        let path_svg: &Path = Path::new("src/tests/data/svg/image.svg");

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDOUT should contain HTML with date URL for background-image in it
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head></head><body><div style=\"background-image: url('data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KICAgIDxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9InJlZCIgLz4KICAgIDxjaXJjbGUgY3g9IjE1MCIgY3k9IjEwMCIgcj0iODAiIGZpbGw9ImdyZWVuIiAvPgogICAgPHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+Cjwvc3ZnPgo=')\"></div>\n</body></html>\n"
        );

        // STDERR should list files that got retrieved
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file_url_html}\n \
                {file_url_css}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(&path_html).unwrap())
                    .unwrap()
                    .into_string(),
                file_url_css = Url::from_file_path(fs::canonicalize(&path_svg).unwrap())
                    .unwrap()
                    .into_string(),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn discard_integrity_for_local_files() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let cwd_normalized: String =
            str!(env::current_dir().unwrap().to_str().unwrap()).replace("\\", "/");
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };
        let out = cmd
            .arg("-M")
            .arg("-i")
            .arg(if cfg!(windows) {
                format!(
                    "{file}{cwd}/src/tests/data/integrity/index.html",
                    file = file_url_protocol,
                    cwd = cwd_normalized,
                )
            } else {
                format!(
                    "{file}{cwd}/src/tests/data/integrity/index.html",
                    file = file_url_protocol,
                    cwd = cwd_normalized,
                )
            })
            .output()
            .unwrap();

        // STDOUT should contain HTML from the local file; integrity attributes should be missing
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            format!(
                "\
                <!DOCTYPE html><html lang=\"en\"><head>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"img-src data:;\"></meta>\n  \
                <title>Local HTML file</title>\n  \
                <link href=\"data:text/css;base64,Ym9keSB7CiAgICBiYWNrZ3JvdW5kLWNvbG9yOiAjMDAwOwogICAgY29sb3I6ICNGRkY7Cn0K\" rel=\"stylesheet\" type=\"text/css\" crossorigin=\"anonymous\">\n  \
                <link href=\"style.css\" rel=\"stylesheet\" type=\"text/css\" crossorigin=\"anonymous\">\n</head>\n\n<body>\n  \
                <p>This page should have black background and white foreground, but only when served via http: (not via file:)</p>\n  \
                <script src=\"data:application/javascript;base64,ZnVuY3Rpb24gbm9vcCgpIHsKICAgIGNvbnNvbGUubG9nKCJtb25vbGl0aCIpOwp9Cg==\"></script>\n  \
                <script src=\"script.js\"></script>\n\n\n\n\
                </body></html>\n\
                "
            )
        );

        // STDERR should contain list of retrieved file URLs
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file}{cwd}/src/tests/data/integrity/index.html\n \
                {file}{cwd}/src/tests/data/integrity/style.css\n \
                {file}{cwd}/src/tests/data/integrity/style.css\n \
                {file}{cwd}/src/tests/data/integrity/script.js\n \
                {file}{cwd}/src/tests/data/integrity/script.js\n\
                ",
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }
}
