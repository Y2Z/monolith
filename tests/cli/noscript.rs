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
    use std::path::Path;
    use url::Url;

    #[test]
    fn parse_noscript_contents() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let path_html: &Path = Path::new("tests/_data_/noscript/index.html");
        let path_svg: &Path = Path::new("tests/_data_/noscript/image.svg");

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should contain target HTML and embedded SVG files
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

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head><meta name=\"robots\" content=\"none\"></head><body><noscript><img src=\"data:image/svg+xml,%3Csvg version=%221.1%22 baseProfile=%22full%22 width=%22300%22 height=%22200%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%0A    %3Crect width=%22100%25%22 height=%22100%25%22 fill=%22red%22 /%3E%0A    %3Ccircle cx=%22150%22 cy=%22100%22 r=%2280%22 fill=%22green%22 /%3E%0A    %3Ctext x=%22150%22 y=%22125%22 font-size=%2260%22 text-anchor=%22middle%22 fill=%22white%22%3ESVG%3C/text%3E%0A%3C/svg%3E%0A\"></noscript>\n</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn unwrap_noscript_contents() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let path_html: &Path = Path::new("tests/_data_/noscript/index.html");
        let path_svg: &Path = Path::new("tests/_data_/noscript/image.svg");

        let out = cmd.arg("-Mn").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should contain target HTML and embedded SVG files
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

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head><meta name=\"robots\" content=\"none\"></head><body><img src=\"data:image/svg+xml,%3Csvg version=%221.1%22 baseProfile=%22full%22 width=%22300%22 height=%22200%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%0A    %3Crect width=%22100%25%22 height=%22100%25%22 fill=%22red%22 /%3E%0A    %3Ccircle cx=%22150%22 cy=%22100%22 r=%2280%22 fill=%22green%22 /%3E%0A    %3Ctext x=%22150%22 y=%22125%22 font-size=%2260%22 text-anchor=%22middle%22 fill=%22white%22%3ESVG%3C/text%3E%0A%3C/svg%3E%0A\">\n</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn unwrap_noscript_contents_nested() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let path_html: &Path = Path::new("tests/_data_/noscript/nested.html");
        let path_svg: &Path = Path::new("tests/_data_/noscript/image.svg");

        let out = cmd.arg("-Mn").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should contain target HTML and embedded SVG files
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

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head><meta name=\"robots\" content=\"none\"></head><body><h1>JS is not active</h1><img src=\"data:image/svg+xml,%3Csvg version=%221.1%22 baseProfile=%22full%22 width=%22300%22 height=%22200%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%0A    %3Crect width=%22100%25%22 height=%22100%25%22 fill=%22red%22 /%3E%0A    %3Ccircle cx=%22150%22 cy=%22100%22 r=%2280%22 fill=%22green%22 /%3E%0A    %3Ctext x=%22150%22 y=%22125%22 font-size=%2260%22 text-anchor=%22middle%22 fill=%22white%22%3ESVG%3C/text%3E%0A%3C/svg%3E%0A\">\n</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn unwrap_noscript_contents_with_script() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let path_html: &Path = Path::new("tests/_data_/noscript/script.html");
        let path_svg: &Path = Path::new("tests/_data_/noscript/image.svg");

        let out = cmd.arg("-Mn").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should contain target HTML and embedded SVG files
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

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r#"<html><head><meta name="robots" content="none"></head><body><script>alert(1);</script><img src="data:image/svg+xml,%3Csvg version=%221.1%22 baseProfile=%22full%22 width=%22300%22 height=%22200%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%0A    %3Crect width=%22100%25%22 height=%22100%25%22 fill=%22red%22 /%3E%0A    %3Ccircle cx=%22150%22 cy=%22100%22 r=%2280%22 fill=%22green%22 /%3E%0A    %3Ctext x=%22150%22 y=%22125%22 font-size=%2260%22 text-anchor=%22middle%22 fill=%22white%22%3ESVG%3C/text%3E%0A%3C/svg%3E%0A">
</body></html>
"#
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn unwrap_noscript_contents_attr_data_url() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let out = cmd
            .arg("-M")
            .arg("-n")
            .arg("data:text/html,<noscript class=\"\">test</noscript>")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain unwrapped contents of NOSCRIPT element
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r#"<html><head>test<meta name="robots" content="none"></head><body></body></html>
"#
        );

        // Exit code should be 0
        out.assert().code(0);
    }
}
