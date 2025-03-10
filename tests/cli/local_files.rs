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
    use std::path::{Path, MAIN_SEPARATOR};
    use std::process::Command;
    use url::Url;

    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn local_file_target_input_relative_target_path() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let cwd_normalized: String = env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("\\", "/");
        let out = cmd
            .arg("-M")
            .arg(format!(
                "tests{s}_data_{s}basic{s}local-file.html",
                s = MAIN_SEPARATOR
            ))
            .output()
            .unwrap();
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

        // STDERR should contain list of retrieved file URLs, two missing
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "\
                {file}{cwd}/tests/_data_/basic/local-file.html\n\
                {file}{cwd}/tests/_data_/basic/local-style.css\n\
                {file}{cwd}/tests/_data_/basic/local-style-does-not-exist.css (file not found)\n\
                {file}{cwd}/tests/_data_/basic/monolith.png (file not found)\n\
                {file}{cwd}/tests/_data_/basic/local-script.js\n\
                ",
                file = file_url_protocol,
                cwd = cwd_normalized
            )
        );

        // STDOUT should contain HTML from the local file
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
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

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn local_file_target_input_absolute_target_path() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let path_html: &Path = Path::new("tests/_data_/basic/local-file.html");

        let out = cmd
            .arg("-M")
            .arg("-Ijci")
            .arg(path_html.as_os_str())
            .output()
            .unwrap();

        // STDERR should contain only the target file
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "{file_url_html}\n",
                file_url_html = Url::from_file_path(fs::canonicalize(path_html).unwrap()).unwrap(),
            )
        );

        // STDOUT should contain HTML from the local file
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            format!(
                "\
                <!DOCTYPE html><html lang=\"en\"><head>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-eval' 'unsafe-inline' data:; style-src 'none'; script-src 'none'; img-src data:;\"></meta>\n  \
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
                empty_image = EMPTY_IMAGE_DATA_URL
            )
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn local_file_url_target_input() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let cwd_normalized: String = env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("\\", "/");
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };
        let out = cmd
            .arg("-M")
            .arg("-cji")
            .arg(format!(
                "{file}{cwd}/tests/_data_/basic/local-file.html",
                file = file_url_protocol,
                cwd = cwd_normalized,
            ))
            .output()
            .unwrap();

        // STDERR should contain list of retrieved file URLs
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "{file}{cwd}/tests/_data_/basic/local-file.html\n",
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // STDOUT should contain HTML from the local file
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
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
                empty_image = EMPTY_IMAGE_DATA_URL
            )
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn embed_file_url_local_asset_within_style_attribute() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let path_html: &Path = Path::new("tests/_data_/svg/index.html");
        let path_svg: &Path = Path::new("tests/_data_/svg/image.svg");

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should list files that got retrieved
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "\
                {file_url_html}\n\
                {file_url_svg}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(path_html).unwrap()).unwrap(),
                file_url_svg = Url::from_file_path(fs::canonicalize(path_svg).unwrap()).unwrap(),
            )
        );

        // STDOUT should contain HTML with date URL for background-image in it
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head></head><body><div style=\"background-image: url(&quot;data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KICAgIDxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9InJlZCIgLz4KICAgIDxjaXJjbGUgY3g9IjE1MCIgY3k9IjEwMCIgcj0iODAiIGZpbGw9ImdyZWVuIiAvPgogICAgPHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+Cjwvc3ZnPgo=&quot;)\"></div>\n</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn discard_integrity_for_local_files() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let cwd_normalized: String = env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("\\", "/");
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };
        let out = cmd
            .arg("-M")
            .arg("-i")
            .arg(if cfg!(windows) {
                format!(
                    "{file}{cwd}/tests/_data_/integrity/index.html",
                    file = file_url_protocol,
                    cwd = cwd_normalized,
                )
            } else {
                format!(
                    "{file}{cwd}/tests/_data_/integrity/index.html",
                    file = file_url_protocol,
                    cwd = cwd_normalized,
                )
            })
            .output()
            .unwrap();

        // STDERR should contain list of retrieved file URLs
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "\
                {file}{cwd}/tests/_data_/integrity/index.html\n\
                {file}{cwd}/tests/_data_/integrity/style.css\n\
                {file}{cwd}/tests/_data_/integrity/style.css\n\
                {file}{cwd}/tests/_data_/integrity/script.js\n\
                {file}{cwd}/tests/_data_/integrity/script.js\n\
                ",
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // STDOUT should contain HTML from the local file; integrity attributes should be missing
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
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

        // Exit code should be 0
        out.assert().code(0);
    }
}
