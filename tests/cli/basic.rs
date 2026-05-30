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

    #[test]
    fn css_image_props() {
        // for a predictable resolved url hash
        use monolith::css::hash_url;

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let path_html: &Path = Path::new("tests/_data_/css/css_prop_assets.html");
        let path_img: &Path = Path::new("tests/_data_/css/colors.png");

        assert!(path_html.is_file());
        assert!(path_img.is_file());

        let file_url_img = Url::from_file_path(fs::canonicalize(path_img).unwrap()).unwrap();
        let url_hash = hash_url(file_url_img.to_string());

        let out = cmd.arg("-M").arg("-x").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should list files that got retrieved
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "\
                {file_url_html}\n\
                {file_url_img}\n\
                {file_url_img}\n\
                {file_url_img}\n\
                {file_url_img}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(path_html).unwrap()).unwrap(),
                file_url_img = file_url_img,
            )
        );

        // STDOUT should contain a custom property defining the background image
        // the two icons should use the custom prop instead of a data url
        // the var names is the hash of the full path: ./blue-red.png
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r##"<html><head>
    <style>
        .icon {
            display: block;
            height: 32px;
            width: 32px;
        }

        .bg {
            background-image: var(--img-644aba16ff0fb4659c6ddec15a510f7bbced77f23da4dd76267056e525627b27);
        }

        .icon-1 {
            background-image: var(--img-644aba16ff0fb4659c6ddec15a510f7bbced77f23da4dd76267056e525627b27);
        }

        .icon-2 {
            background-position: 32px 0;
            background-image: var(--img-644aba16ff0fb4659c6ddec15a510f7bbced77f23da4dd76267056e525627b27);
        }

        .icon-3 {
            background: var(--img-644aba16ff0fb4659c6ddec15a510f7bbced77f23da4dd76267056e525627b27) 0px 32px;
        }

        .icon-4 {
            background-position: 32px 32px;
        }
    @property --img-644aba16ff0fb4659c6ddec15a510f7bbced77f23da4dd76267056e525627b27 {inherits: false; syntax: "<url>"; initial-value: url("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAIAAAAlC+aJAAAAAXNSR0IB2cksfwAAAARnQU1BAACxjwv8YQUAAAAgY0hSTQAAeiYAAICEAAD6AAAAgOgAAHUwAADqYAAAOpgAABdwnLpRPAAAAAlwSFlzAAAuIwAALiMBeKU/dgAAAFpJREFUaN7t0YEJACAMA8FU3H/lOkVF4X6AlKOVdCbr1Oj+yucBAAAAAAAAAAAAAAAAAAAAAAAAAAAA3G/38IFqHwAAAAAAAAAAAAAAAAAAAAAAAAAAAAB4rAMqRwSAP0qNuQAAAABJRU5ErkJggg==");}</style>
<meta name="robots" content="none"></meta></head>

<body>
    <div class="icon icon-1"></div>
    <hr>
    <div class="icon icon-2"></div>
    <hr>
    <div class="icon icon-3"></div>
    <hr>
    <div class="icon bg icon-4"></div>

</body></html>
"##.replace("PROP_NAME", &format!("img-{}", url_hash))
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
