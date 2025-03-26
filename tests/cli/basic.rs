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
    fn print_help_information() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd.arg("-h").output().unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain program name, version, and usage information
        // TODO

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn print_version() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd.arg("-V").output().unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain program name and version
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            format!("{} {}\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn stdin_target_input() {
        let mut echo = Command::new("echo")
            .arg("Hello from STDIN")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let echo_out = echo.stdout.take().unwrap();
        echo.wait().unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.stdin(echo_out);
        let out = cmd.arg("-M").arg("-").output().unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML created out of STDIN
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r#"<html><head><meta name="robots" content="none"></meta></head><body>Hello from STDIN
</body></html>
"#
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn css_import_string() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let path_html: &Path = Path::new("tests/_data_/css/index.html");
        let path_css: &Path = Path::new("tests/_data_/css/style.css");

        assert!(path_html.is_file());
        assert!(path_css.is_file());

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should list files that got retrieved
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "\
                {file_url_html}\n\
                {file_url_css}\n\
                {file_url_css}\n\
                {file_url_css}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(path_html).unwrap()).unwrap(),
                file_url_css = Url::from_file_path(fs::canonicalize(path_css).unwrap()).unwrap(),
            )
        );

        // STDOUT should contain embedded CSS url()'s
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r##"<html><head><style>

    @charset "UTF-8";

    @import "data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K";

    @import url("data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K");

    @import url("data:text/css;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K");

</style>
<meta name="robots" content="none"></meta></head><body></body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
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
    fn bad_input_empty_target() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd.arg("").output().unwrap();

        // STDERR should contain error description
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            "Error: no target specified\n"
        );

        // STDOUT should be empty
        assert_eq!(String::from_utf8_lossy(&out.stdout), "");

        // Exit code should be 1
        out.assert().code(1);
    }

    #[test]
    fn unsupported_scheme() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd.arg("mailto:snshn@tutanota.com").output().unwrap();

        // STDERR should contain error description
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            "Error: unsupported target URL scheme \"mailto\"\n"
        );

        // STDOUT should be empty
        assert_eq!(String::from_utf8_lossy(&out.stdout), "");

        // Exit code should be 1
        out.assert().code(1);
    }
}
