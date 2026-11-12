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
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use url::Url;

    #[test]
    fn malformed_srcset_descriptor_does_not_hang() {
        let mut cmd = crate::cli::monolith();
        cmd.arg("-")
            .write_stdin("<img srcset=\"data:,image invalid\">")
            // Busy machines take their time, hanging takes forever
            .timeout(std::time::Duration::from_secs(60))
            .assert()
            .success();
    }

    #[test]
    fn print_help_information() {
        let mut cmd = crate::cli::monolith();
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
        let mut cmd = crate::cli::monolith();
        let out = cmd.arg("-V").output().unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain program name and version
        assert_eq!(
            crate::cli::without_metadata(&out.stdout),
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

        let mut cmd = crate::cli::monolith();
        cmd.write_stdin(echo.stdout);
        let out = cmd.arg("-").output().unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML created out of STDIN
        assert_eq!(
            crate::cli::without_metadata(&out.stdout),
            r#"<html><head><meta name="robots" content="none"></head><body>Hello from STDIN
</body></html>
"#
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn css_import_string() {
        let mut cmd = crate::cli::monolith();
        let path_html: &Path = Path::new("tests/_data_/css/index.html");
        let path_css: &Path = Path::new("tests/_data_/css/style.css");

        assert!(path_html.is_file());
        assert!(path_css.is_file());

        let out = cmd.arg(path_html.as_os_str()).output().unwrap();

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
            crate::cli::without_metadata(&out.stdout),
            r##"<html><head><style>

    @charset "UTF-8";

    @import "data:text/css;charset=utf-8,body{background-color:%23000;color:%23fff}%0A";

    @import url("data:text/css;charset=utf-8,body{background-color:%23000;color:%23fff}%0A");

    @import url("data:text/css;charset=utf-8,body{background-color:%23000;color:%23fff}%0A");

</style>
<meta name="robots" content="none"></head><body></body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn output_file_into_missing_directories() {
        let mut cmd = crate::cli::monolith_writing_files();
        let path_html: &Path = Path::new("tests/_data_/basic/local-file.html");
        let dir_tmp: PathBuf =
            env::temp_dir().join(format!("monolith-test-{}", std::process::id()));
        let path_output: PathBuf = dir_tmp.join("subdir").join("output.html");

        let out = cmd
            .arg("-o")
            .arg(path_output.as_os_str())
            .arg(path_html.as_os_str())
            .output()
            .unwrap();

        // The output file should get created along with missing directories in its path
        assert!(path_output.is_file());

        // STDOUT should be empty
        assert_eq!(crate::cli::without_metadata(&out.stdout), "");

        // Exit code should be 0
        out.assert().code(0);

        // Clean up
        fs::remove_dir_all(&dir_tmp).unwrap();
    }

    #[test]
    fn create_dirs_option_is_gone() {
        let mut cmd = crate::cli::monolith_writing_files();

        let out = cmd
            .args(["--create-dirs", "tests/_data_/basic/local-file.html"])
            .output()
            .unwrap();

        // Directories get created anyway, the option isn't needed anymore
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("unexpected argument '--create-dirs'"),
            "{stderr}"
        );
        out.assert().code(2);
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

    #[test]
    fn bad_input_empty_target() {
        let mut cmd = crate::cli::monolith();
        let out = cmd.arg("").output().unwrap();

        // STDERR should contain error description
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            "Error: no target specified\n"
        );

        // STDOUT should be empty
        assert_eq!(crate::cli::without_metadata(&out.stdout), "");

        // Exit code should be 1
        out.assert().code(1);
    }

    #[test]
    fn unsupported_scheme() {
        let mut cmd = crate::cli::monolith();
        let out = cmd.arg("mailto:snshn@tutanota.com").output().unwrap();

        // STDERR should contain error description
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            "Error: unsupported target URL scheme \"mailto\"\n"
        );

        // STDOUT should be empty
        assert_eq!(crate::cli::without_metadata(&out.stdout), "");

        // Exit code should be 1
        out.assert().code(1);
    }
}

#[cfg(test)]
mod passing_saving {
    use std::fs;

    #[test]
    fn saving_again_keeps_what_has_been_saved_before() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("page.html");
        fs::write(&output, "keep me").unwrap();

        crate::cli::monolith_writing_files()
            .args(["-q", "-o"])
            .arg(&output)
            .arg("data:text/html,<p>new")
            .assert()
            .success();

        assert_eq!(fs::read_to_string(&output).unwrap(), "keep me");
        assert!(
            fs::read_to_string(directory.path().join("page (2).html"))
                .unwrap()
                .contains("<p>new")
        );
    }
}
