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
    use std::process::{Command, Stdio};
    use url::Url;

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
    fn stdin_target_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut echo = Command::new("echo")
            .arg("Hello from STDIN")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let echo_out = echo.stdout.take().unwrap();
        echo.wait().unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        cmd.stdin(echo_out);
        let out = cmd.arg("-M").arg("-").output().unwrap();

        // STDOUT should contain HTML from STDIN
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head></head><body>Hello from STDIN\n</body></html>\n"
        );

        Ok(())
    }

    #[test]
    fn css_import_string() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let path_html: &Path = Path::new("src/tests/data/css/index.html");
        let path_css: &Path = Path::new("src/tests/data/css/style.css");

        assert!(path_html.is_file());
        assert!(path_css.is_file());

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDOUT should contain embedded CSS url()'s
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head><style>\n\n    @charset \"UTF-8\";\n\n    @import \"data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K\";\n\n    @import url(\"data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K\");\n\n    @import url(\"data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K\");\n\n</style>\n</head><body></body></html>\n"
        );

        // STDERR should list files that got retrieved
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file_url_html}\n \
                {file_url_css}\n \
                {file_url_css}\n \
                {file_url_css}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(&path_html).unwrap())
                    .unwrap()
                    .into_string(),
                file_url_css = Url::from_file_path(fs::canonicalize(&path_css).unwrap())
                    .unwrap()
                    .into_string(),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use assert_cmd::prelude::*;
    use std::env;
    use std::process::Command;

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
}
