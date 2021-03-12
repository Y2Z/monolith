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
    fn parse_noscript_contents() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let path_html: &Path = Path::new("src/tests/data/noscript/index.html");
        let path_svg: &Path = Path::new("src/tests/data/noscript/image.svg");

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head></head><body><noscript><img src=\"data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KICAgIDxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9InJlZCIgLz4KICAgIDxjaXJjbGUgY3g9IjE1MCIgY3k9IjEwMCIgcj0iODAiIGZpbGw9ImdyZWVuIiAvPgogICAgPHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+Cjwvc3ZnPgo=\"></noscript>\n</body></html>\n"
        );

        // STDERR should contain target HTML and embedded SVG files
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file_url_html}\n \
                {file_url_svg}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(&path_html).unwrap())
                    .unwrap()
                    .into_string(),
                file_url_svg = Url::from_file_path(fs::canonicalize(&path_svg).unwrap())
                    .unwrap()
                    .into_string(),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn unwrap_noscript_contents() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let path_html: &Path = Path::new("src/tests/data/noscript/index.html");
        let path_svg: &Path = Path::new("src/tests/data/noscript/image.svg");

        let out = cmd.arg("-Mn").arg(path_html.as_os_str()).output().unwrap();

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head></head><body><!--noscript--><img src=\"data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KICAgIDxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9InJlZCIgLz4KICAgIDxjaXJjbGUgY3g9IjE1MCIgY3k9IjEwMCIgcj0iODAiIGZpbGw9ImdyZWVuIiAvPgogICAgPHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+Cjwvc3ZnPgo=\"><!--/noscript-->\n</body></html>\n"
        );

        // STDERR should contain target HTML and embedded SVG files
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file_url_html}\n \
                {file_url_svg}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(&path_html).unwrap())
                    .unwrap()
                    .into_string(),
                file_url_svg = Url::from_file_path(fs::canonicalize(&path_svg).unwrap())
                    .unwrap()
                    .into_string(),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn unwrap_noscript_contents_nested() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let path_html: &Path = Path::new("src/tests/data/noscript/nested.html");
        let path_svg: &Path = Path::new("src/tests/data/noscript/image.svg");

        let out = cmd.arg("-Mn").arg(path_html.as_os_str()).output().unwrap();

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head></head><body><!--noscript--><h1>JS is not active</h1><!--noscript--><img src=\"data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KICAgIDxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9InJlZCIgLz4KICAgIDxjaXJjbGUgY3g9IjE1MCIgY3k9IjEwMCIgcj0iODAiIGZpbGw9ImdyZWVuIiAvPgogICAgPHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+Cjwvc3ZnPgo=\"><!--/noscript--><!--/noscript-->\n</body></html>\n"
        );

        // STDERR should contain target HTML and embedded SVG files
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file_url_html}\n \
                {file_url_svg}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(&path_html).unwrap())
                    .unwrap()
                    .into_string(),
                file_url_svg = Url::from_file_path(fs::canonicalize(&path_svg).unwrap())
                    .unwrap()
                    .into_string(),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn unwrap_noscript_contents_with_script() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let path_html: &Path = Path::new("src/tests/data/noscript/script.html");
        let path_svg: &Path = Path::new("src/tests/data/noscript/image.svg");

        let out = cmd.arg("-Mn").arg(path_html.as_os_str()).output().unwrap();

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head></head><body><!--noscript--><img src=\"data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KICAgIDxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9InJlZCIgLz4KICAgIDxjaXJjbGUgY3g9IjE1MCIgY3k9IjEwMCIgcj0iODAiIGZpbGw9ImdyZWVuIiAvPgogICAgPHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+Cjwvc3ZnPgo=\"><!--/noscript-->\n</body></html>\n"
        );

        // STDERR should contain target HTML and embedded SVG files
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "\
                {file_url_html}\n \
                {file_url_svg}\n\
                ",
                file_url_html = Url::from_file_path(fs::canonicalize(&path_html).unwrap())
                    .unwrap()
                    .into_string(),
                file_url_svg = Url::from_file_path(fs::canonicalize(&path_svg).unwrap())
                    .unwrap()
                    .into_string(),
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }
}
