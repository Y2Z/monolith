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
    use std::process::{Command, Stdio};
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
