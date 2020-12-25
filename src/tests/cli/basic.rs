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
    use std::io::Write;
    use std::process::Command;
    use tempfile::NamedTempFile;

    #[test]
    fn print_version() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd.arg("-V").output().unwrap();

        // STDOUT should contain program name and version
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            format!("{} {}\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn bad_input_empty_target() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd.arg("").output().unwrap();

        // STDOUT should be empty
        assert_eq!(std::str::from_utf8(&out.stdout).unwrap(), "");

        // STDERR should contain error description
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            "No target specified\n"
        );

        // The exit code should be 1
        out.assert().code(1);

        Ok(())
    }

    #[test]
    fn bad_input_data_url() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd.arg("data:,Hello%2C%20World!").output().unwrap();

        // STDOUT should contain HTML
        assert_eq!(std::str::from_utf8(&out.stdout).unwrap(), "");

        // STDERR should contain error description
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            "Unsupported data URL media type\n"
        );

        // The exit code should be 1
        out.assert().code(1);

        Ok(())
    }

    #[test]
    fn isolate_data_url() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-I")
            .arg("data:text/html,Hello%2C%20World!")
            .output()
            .unwrap();

        // STDOUT should contain isolated HTML
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-inline' data:;\"></meta>\
            </head><body>Hello, World!</body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn remove_css_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-c")
            .arg("data:text/html,<style>body{background-color:pink}</style>Hello")
            .output()
            .unwrap();

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none';\"></meta>\
            <style></style>\
            </head><body>Hello</body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn remove_fonts_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-F")
            .arg("data:text/html,<style>@font-face { font-family: myFont; src: url(font.woff); }</style>Hi")
            .output()
            .unwrap();

        // STDOUT should contain HTML with no web fonts
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"font-src 'none';\"></meta>\
            <style></style>\
            </head><body>Hi</body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn remove_frames_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-f")
            .arg("data:text/html,<iframe src=\"https://duckduckgo.com\"></iframe>Hi")
            .output()
            .unwrap();

        // STDOUT should contain HTML with no iframes
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"frame-src 'none'; child-src 'none';\"></meta>\
            </head><body><iframe src=\"\"></iframe>Hi</body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn remove_images_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-i")
            .arg("data:text/html,<img src=\"https://google.com\"/>Hi")
            .output()
            .unwrap();

        // STDOUT should contain HTML with no images
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            format!(
                "<html>\
                <head>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"img-src data:;\"></meta>\
                </head>\
                <body>\
                <img src=\"{empty_image}\">\
                Hi\
                </body>\
                </html>\n",
                empty_image = empty_image!()
            )
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn remove_js_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-j")
            .arg("data:text/html,<script>alert(2)</script>Hi")
            .output()
            .unwrap();

        // STDOUT should contain HTML with no JS
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html>\
            <head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'none';\"></meta>\
            <script></script></head>\
            <body>Hi</body>\
            </html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn local_file_target_input() -> Result<(), Box<dyn std::error::Error>> {
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
            <link rel=\"stylesheet\" type=\"text/css\" href=\"data:text/css;base64,Ym9keSB7CiAgICBiYWNrZ3JvdW5kLWNvbG9yOiAjMDAwOwogICAgY29sb3I6ICNmZmY7Cn0K\">\n  \
            <link rel=\"stylesheet\" type=\"text/css\">\n</head>\n\n<body>\n  \
            <img alt=\"\">\n  \
            <a href=\"file://local-file.html/\">Tricky href</a>\n  \
            <a href=\"https://github.com/Y2Z/monolith\">Remote URL</a>\n  \
            <script src=\"data:application/javascript;base64,ZG9jdW1lbnQuYm9keS5zdHlsZS5iYWNrZ3JvdW5kQ29sb3IgPSAiZ3JlZW4iOwpkb2N1bWVudC5ib2R5LnN0eWxlLmNvbG9yID0gInJlZCI7Cg==\"></script>\n\n\n\n\
            </body></html>\n\
            "
        );

        // STDERR should contain list of retrieved file URLs
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file}{cwd}/src/tests/data/basic/local-file.html\n \
                {file}{cwd}/src/tests/data/basic/local-style.css\n \
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
        let cwd = env::current_dir().unwrap();
        let cwd_normalized: String =
            str!(env::current_dir().unwrap().to_str().unwrap()).replace("\\", "/");
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-jciI")
            .arg(if cfg!(windows) {
                format!(
                    "{cwd}\\src\\tests\\data\\basic\\local-file.html",
                    cwd = cwd.to_str().unwrap()
                )
            } else {
                format!(
                    "{cwd}/src/tests/data/basic/local-file.html",
                    cwd = cwd.to_str().unwrap()
                )
            })
            .output()
            .unwrap();
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

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
            if cfg!(windows) {
                format!(
                    "{file}{cwd}/src/tests/data/basic/local-file.html\n",
                    file = file_url_protocol,
                    cwd = cwd_normalized,
                )
            } else {
                format!(
                    "{file}{cwd}/src/tests/data/basic/local-file.html\n",
                    file = file_url_protocol,
                    cwd = cwd_normalized,
                )
            }
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn security_disallow_local_assets_within_data_url_targets(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("data:text/html,%3Cscript%20src=\"src/tests/data/basic/local-script.js\"%3E%3C/script%3E")
            .output()
            .unwrap();

        // STDOUT should contain HTML with no JS in it
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head><script></script></head><body></body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn embed_file_url_local_asset_within_style_attribute() -> Result<(), Box<dyn std::error::Error>>
    {
        let file_url_prefix: &str = if cfg!(windows) { "file:///" } else { "file://" };
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let mut file_svg = NamedTempFile::new()?;
        writeln!(file_svg, "<svg version=\"1.1\" baseProfile=\"full\" width=\"300\" height=\"200\" xmlns=\"http://www.w3.org/2000/svg\">\
                            <rect width=\"100%\" height=\"100%\" fill=\"red\" />\
                            <circle cx=\"150\" cy=\"100\" r=\"80\" fill=\"green\" />\
                            <text x=\"150\" y=\"125\" font-size=\"60\" text-anchor=\"middle\" fill=\"white\">SVG</text>\
                            </svg>\n")?;
        let mut file_html = NamedTempFile::new()?;
        writeln!(
            file_html,
            "<div style='background-image: url(\"{file}{path}\")'></div>\n",
            file = file_url_prefix,
            path = str!(file_svg.path().to_str().unwrap()).replace("\\", "/"),
        )?;
        let out = cmd.arg("-M").arg(file_html.path()).output().unwrap();

        // STDOUT should contain HTML with date URL for background-image in it
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head></head><body><div style=\"background-image: url('data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwJSIgaGVpZ2h0PSIxMDAlIiBmaWxsPSJyZWQiIC8+PGNpcmNsZSBjeD0iMTUwIiBjeT0iMTAwIiByPSI4MCIgZmlsbD0iZ3JlZW4iIC8+PHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+PC9zdmc+Cgo=')\"></div>\n\n</body></html>\n"
        );

        // STDERR should list temporary files that got retrieved
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file}{html_path}\n \
                {file}{svg_path}\n\
                ",
                file = file_url_prefix,
                html_path = str!(file_html.path().to_str().unwrap()).replace("\\", "/"),
                svg_path = str!(file_svg.path().to_str().unwrap()).replace("\\", "/"),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn css_import_string() -> Result<(), Box<dyn std::error::Error>> {
        let file_url_prefix: &str = if cfg!(windows) { "file:///" } else { "file://" };
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let mut file_css = NamedTempFile::new()?;
        writeln!(file_css, "body{{background-color:#000;color:#fff}}")?;
        let mut file_html = NamedTempFile::new()?;
        writeln!(
            file_html,
            "\
            <style>\n\
            @charset 'UTF-8';\n\
            \n\
            @import '{file}{css_path}';\n\
            \n\
            @import url({file}{css_path});\n\
            \n\
            @import url('{file}{css_path}')\n\
            </style>\n\
            ",
            file = file_url_prefix,
            css_path = str!(file_css.path().to_str().unwrap()).replace("\\", "/"),
        )?;
        let out = cmd.arg("-M").arg(file_html.path()).output().unwrap();

        // STDOUT should contain embedded CSS url()'s
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head><style>\n@charset 'UTF-8';\n\n@import 'data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K';\n\n@import url('data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K');\n\n@import url('data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K')\n</style>\n\n</head><body></body></html>\n"
        );

        // STDERR should list temporary files that got retrieved
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file}{html_path}\n \
                {file}{css_path}\n \
                {file}{css_path}\n \
                {file}{css_path}\n\
                ",
                file = file_url_prefix,
                html_path = str!(file_html.path().to_str().unwrap()).replace("\\", "/"),
                css_path = str!(file_css.path().to_str().unwrap()).replace("\\", "/"),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }
}
