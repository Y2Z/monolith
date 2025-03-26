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
                r#"{file}{cwd}/tests/_data_/basic/local-file.html
{file}{cwd}/tests/_data_/basic/local-style.css
{file}{cwd}/tests/_data_/basic/local-style-does-not-exist.css (file not found)
{file}{cwd}/tests/_data_/basic/monolith.png (file not found)
{file}{cwd}/tests/_data_/basic/local-script.js
"#,
                file = file_url_protocol,
                cwd = cwd_normalized
            )
        );

        // STDOUT should contain HTML from the local file
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r##"<!DOCTYPE html><html lang="en"><head>
  <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
  <title>Local HTML file</title>
  <link href="data:text/css;base64,Ym9keSB7CiAgICBiYWNrZ3JvdW5kLWNvbG9yOiAjMDAwOwogICAgY29sb3I6ICNmZmY7Cn0K" rel="stylesheet" type="text/css">
  <link rel="stylesheet" type="text/css">
<meta name="robots" content="none"></meta></head>

<body>
  <img alt="">
  <a href="file://local-file.html/">Tricky href</a>
  <a href="https://github.com/Y2Z/monolith">Remote URL</a>
  <script>document.body.style.backgroundColor = "green";
document.body.style.color = "red";
</script>



</body></html>
"##
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
                r##"<!DOCTYPE html><html lang="en"><head><meta http-equiv="Content-Security-Policy" content="default-src 'unsafe-eval' 'unsafe-inline' data:; style-src 'none'; script-src 'none'; img-src data:;"></meta>
  <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
  <title>Local HTML file</title>
  <link rel="stylesheet" type="text/css">
  <link rel="stylesheet" type="text/css">
<meta name="robots" content="none"></meta></head>

<body>
  <img src="{empty_image}" alt="">
  <a href="file://local-file.html/">Tricky href</a>
  <a href="https://github.com/Y2Z/monolith">Remote URL</a>
  <script></script>



</body></html>
"##,
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
                r##"<!DOCTYPE html><html lang="en"><head><meta http-equiv="Content-Security-Policy" content="style-src 'none'; script-src 'none'; img-src data:;"></meta>
  <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
  <title>Local HTML file</title>
  <link rel="stylesheet" type="text/css">
  <link rel="stylesheet" type="text/css">
<meta name="robots" content="none"></meta></head>

<body>
  <img src="{empty_image}" alt="">
  <a href="file://local-file.html/">Tricky href</a>
  <a href="https://github.com/Y2Z/monolith">Remote URL</a>
  <script></script>



</body></html>
"##,
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
                r#"{file_url_html}
{file_url_svg}
"#,
                file_url_html = Url::from_file_path(fs::canonicalize(path_html).unwrap()).unwrap(),
                file_url_svg = Url::from_file_path(fs::canonicalize(path_svg).unwrap()).unwrap(),
            )
        );

        // STDOUT should contain HTML with date URL for background-image in it
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r##"<html><head><meta name="robots" content="none"></meta></head><body><div style="background-image: url(&quot;data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KICAgIDxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9InJlZCIgLz4KICAgIDxjaXJjbGUgY3g9IjE1MCIgY3k9IjEwMCIgcj0iODAiIGZpbGw9ImdyZWVuIiAvPgogICAgPHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+Cjwvc3ZnPgo=&quot;)"></div>
</body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn embed_svg_local_asset_via_use() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let path_html: &Path = Path::new("tests/_data_/svg/svg.html");
        let path_svg: &Path = Path::new("tests/_data_/svg/icons.svg");

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should list files that got retrieved
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                r#"{file_url_html}
{file_url_svg}
"#,
                file_url_html = Url::from_file_path(fs::canonicalize(path_html).unwrap()).unwrap(),
                file_url_svg = Url::from_file_path(fs::canonicalize(path_svg).unwrap()).unwrap(),
            )
        );

        // STDOUT should contain HTML with one symbol extracted from SVG file
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r##"<html><head><meta name="robots" content="none"></meta></head><body>
<button class="tm-votes-lever__button" data-test-id="votes-lever-upvote-button" title="Like" type="button">
  <svg class="tm-svg-img tm-votes-lever__icon" height="24" width="24">
    <title>Like</title>
    <use xlink:href="#icon-1"><symbol id="icon-1">
      <path fill-rule="evenodd" clip-rule="evenodd" d="M10 20h4V10h3l-5-6.5L7 10h3v10Z"></path>
    </symbol></use>
  </svg>
</button>


</body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn embed_svg_local_asset_via_image() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let path_html: &Path = Path::new("tests/_data_/svg/image.html");
        let path_svg: &Path = Path::new("tests/_data_/svg/image.svg");

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should list files that got retrieved
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                r#"{file_url_html}
{file_url_svg}
"#,
                file_url_html = Url::from_file_path(fs::canonicalize(path_html).unwrap()).unwrap(),
                file_url_svg = Url::from_file_path(fs::canonicalize(path_svg).unwrap()).unwrap(),
            )
        );

        // STDOUT should contain HTML with data URL of SVG file
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r##"<html><head><meta name="robots" content="none"></meta></head><body>
        <svg height="24" width="24">
            <image href="data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIGJhc2VQcm9maWxlPSJmdWxsIiB3aWR0aD0iMzAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KICAgIDxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9InJlZCIgLz4KICAgIDxjaXJjbGUgY3g9IjE1MCIgY3k9IjEwMCIgcj0iODAiIGZpbGw9ImdyZWVuIiAvPgogICAgPHRleHQgeD0iMTUwIiB5PSIxMjUiIGZvbnQtc2l6ZT0iNjAiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGZpbGw9IndoaXRlIj5TVkc8L3RleHQ+Cjwvc3ZnPgo=" width="24" height="24">
        </image></svg>
    

</body></html>
"##
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
                r#"{file}{cwd}/tests/_data_/integrity/index.html
{file}{cwd}/tests/_data_/integrity/style.css
{file}{cwd}/tests/_data_/integrity/style.css
{file}{cwd}/tests/_data_/integrity/script.js
{file}{cwd}/tests/_data_/integrity/script.js
"#,
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // STDOUT should contain HTML from the local file; integrity attributes should be missing
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            format!(
                r##"<!DOCTYPE html><html lang="en"><head><meta http-equiv="Content-Security-Policy" content="img-src data:;"></meta>
  <title>Local HTML file</title>
  <link href="data:text/css;base64,Ym9keSB7CiAgICBiYWNrZ3JvdW5kLWNvbG9yOiAjMDAwOwogICAgY29sb3I6ICNGRkY7Cn0K" rel="stylesheet" type="text/css" crossorigin="anonymous">
  <link href="style.css" rel="stylesheet" type="text/css" crossorigin="anonymous">
<meta name="robots" content="none"></meta></head>

<body>
  <p>This page should have black background and white foreground, but only when served via http: (not via file:)</p>
  <script>function noop() {{
    console.log("monolith");
}}
</script>
  <script src="script.js"></script>



</body></html>
"##
            )
        );

        // Exit code should be 0
        out.assert().code(0);
    }
}
