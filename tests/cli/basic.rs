//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use assert_cmd::cargo_bin_cmd;
    use assert_cmd::prelude::*;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use url::Url;

    #[test]
    fn print_help_information() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
        let echo = Command::new("echo")
            .arg("Hello from STDIN")
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        cmd.write_stdin(echo.stdout);
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
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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

    @import "data:text/css;charset=utf-8;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K";

    @import url("data:text/css;charset=utf-8;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K");

    @import url("data:text/css;charset=utf-8;base64,Ym9keXtiYWNrZ3JvdW5kLWNvbG9yOiMwMDA7Y29sb3I6I2ZmZn0K");

</style>
<meta name="robots" content="none"></meta></head><body></body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn output_file_into_missing_directories() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let path_html: &Path = Path::new("tests/_data_/basic/local-file.html");
        let dir_tmp: PathBuf =
            env::temp_dir().join(format!("monolith-test-{}", std::process::id()));
        let path_output: PathBuf = dir_tmp.join("subdir").join("output.html");

        let out = cmd
            .arg("-M")
            .arg("--create-dirs")
            .arg("-o")
            .arg(path_output.as_os_str())
            .arg(path_html.as_os_str())
            .output()
            .unwrap();

        // The output file should get created along with missing directories in its path
        assert!(path_output.is_file());

        // STDOUT should be empty
        assert_eq!(String::from_utf8_lossy(&out.stdout), "");

        // Exit code should be 0
        out.assert().code(0);

        // Clean up
        fs::remove_dir_all(&dir_tmp).unwrap();
    }

    #[test]
    fn output_file_into_missing_directories_without_create_dirs_flag() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let path_html: &Path = Path::new("tests/_data_/basic/local-file.html");
        let dir_tmp: PathBuf =
            env::temp_dir().join(format!("monolith-test-noflag-{}", std::process::id()));
        let path_output: PathBuf = dir_tmp.join("subdir").join("output.html");

        let out = cmd
            .arg("-M")
            .arg("-o")
            .arg(path_output.as_os_str())
            .arg(path_html.as_os_str())
            .output()
            .unwrap();

        // Without --create-dirs nothing gets created
        assert!(!path_output.is_file());
        assert!(!dir_tmp.exists());

        // The error points the user at the flag instead of panicking
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("--create-dirs"), "stderr: {stderr}");

        // Exit code is 1, not a panic
        out.assert().code(1);
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
    use assert_cmd::cargo_bin_cmd;
    use assert_cmd::prelude::*;
    use std::env;

    #[test]
    fn bad_input_empty_target() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
