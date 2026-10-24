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
    use std::path::{MAIN_SEPARATOR, Path};
    use url::Url;

    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn local_file_target_input_relative_target_path() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
  <link href="data:text/css;charset=utf-8,body {%0A    background-color: %23000;%0A    color: %23fff;%0A}%0A" rel="stylesheet" type="text/css">
  <link rel="stylesheet" type="text/css">
<meta name="robots" content="none"></head>

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
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
                r##"<!DOCTYPE html><html lang="en"><head><meta http-equiv="Content-Security-Policy" content="default-src 'unsafe-eval' 'unsafe-inline' data:; style-src 'none'; script-src 'none'; img-src data:;">
  <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
  <title>Local HTML file</title>
  <link rel="stylesheet" type="text/css">
  <link rel="stylesheet" type="text/css">
<meta name="robots" content="none"></head>

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
    fn local_file_target_input_full_target_path() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        // On Windows this is a path that starts with a drive letter, e.g. "C:\...\local-file.html"
        let path_html = env::current_dir()
            .unwrap()
            .join("tests")
            .join("_data_")
            .join("basic")
            .join("local-file.html");

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
                file_url_html = Url::from_file_path(fs::canonicalize(&path_html).unwrap()).unwrap(),
            )
        );

        // STDOUT should contain HTML from the local file
        assert!(String::from_utf8_lossy(&out.stdout).contains("<title>Local HTML file</title>"));

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn local_file_url_target_input() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
                r##"<!DOCTYPE html><html lang="en"><head><meta http-equiv="Content-Security-Policy" content="style-src 'none'; script-src 'none'; img-src data:;">
  <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
  <title>Local HTML file</title>
  <link rel="stylesheet" type="text/css">
  <link rel="stylesheet" type="text/css">
<meta name="robots" content="none"></head>

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
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
            r##"<html><head><meta name="robots" content="none"></head><body><div style="background-image: url(&quot;data:image/svg+xml,%3Csvg version=%221.1%22 baseProfile=%22full%22 width=%22300%22 height=%22200%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%0A    %3Crect width=%22100%25%22 height=%22100%25%22 fill=%22red%22 /%3E%0A    %3Ccircle cx=%22150%22 cy=%22100%22 r=%2280%22 fill=%22green%22 /%3E%0A    %3Ctext x=%22150%22 y=%22125%22 font-size=%2260%22 text-anchor=%22middle%22 fill=%22white%22%3ESVG%3C/text%3E%0A%3C/svg%3E%0A&quot;)"></div>
</body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn embed_svg_local_asset_via_use() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
            r##"<html><head><meta name="robots" content="none"></head><body>
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
    fn embed_svg_symbol_asset_via_use() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let path_html: &Path = Path::new("tests/_data_/svg/svg_inline_symbol_use.html");

        let out = cmd.arg("-M").arg(path_html.as_os_str()).output().unwrap();

        // STDERR should list files that got retrieved (only 1; no self-embedding for the relative xlink)
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                r#"{file_url_html}
"#,
                file_url_html = Url::from_file_path(fs::canonicalize(path_html).unwrap()).unwrap(),
            )
        );

        // STDOUT should contain HTML with the use id kept the same because it
        // references an inlined SVG symbol in the same document
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r##"<html><head><meta name="robots" content="none"></head><body>
  <svg xmlns="http://www.w3.org/2000/svg">
    <defs>
      <symbol id="icon-1" viewBox="0 0 24 24">
        <path fill-rule="evenodd" clip-rule="evenodd" d="M10 20h4V10h3l-5-6.5L7 10h3v10Z"></path>
      </symbol>
      <symbol id="icon-2" viewBox="0 0 24 24">
        <path fill-rule="evenodd" clip-rule="evenodd" d="M10 20h4V10h3l-5-6.5L7 10h3v10Z"></path>
      </symbol>
    </defs>
  </svg>

  <button class="tm-votes-lever__button" data-test-id="votes-lever-upvote-button" title="Like" type="button">
    <svg class="icon">
      <use xmlns:xlink="http://www.w3.org/1999/xlink" xlink:href="#icon-1" href="#icon-1">
      </use>
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
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
            r##"<html><head><meta name="robots" content="none"></head><body>
        <svg height="24" width="24">
            <image href="data:image/svg+xml,%3Csvg version=%221.1%22 baseProfile=%22full%22 width=%22300%22 height=%22200%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%0A    %3Crect width=%22100%25%22 height=%22100%25%22 fill=%22red%22 /%3E%0A    %3Ccircle cx=%22150%22 cy=%22100%22 r=%2280%22 fill=%22green%22 /%3E%0A    %3Ctext x=%22150%22 y=%22125%22 font-size=%2260%22 text-anchor=%22middle%22 fill=%22white%22%3ESVG%3C/text%3E%0A%3C/svg%3E%0A" width="24" height="24">
        </image></svg>
    "##.to_owned() + r##"

</body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn discard_integrity_for_local_files() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
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
            r##"<!DOCTYPE html><html lang="en"><head><meta http-equiv="Content-Security-Policy" content="img-src data:;">
        <title>Local HTML file</title>
        <link href="data:text/css;charset=utf-8,body {%0A    background-color: %23000;%0A    color: %23FFF;%0A}%0A" rel="stylesheet" type="text/css" crossorigin="anonymous">
        <link href="style.css" rel="stylesheet" type="text/css" crossorigin="anonymous">
    <meta name="robots" content="none"></head>

    <body>
        <p>
            This page should have black background and white foreground, but
            only when served via http: (not via file:)
        </p>
        <script>function noop() {
  console.log("<\/script>");
}
</script>
        <script src="script.js"></script>
    "##.to_owned() + r##"

</body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn escape_script_end_tag_variants_in_local_asset() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let cwd_normalized: String = env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("\\", "/");
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };
        let out = cmd
            .arg("-M")
            .arg("tests/_data_/script-escape/index.html")
            .output()
            .unwrap();

        // STDERR should contain list of retrieved file URLs
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                r#"{file}{cwd}/tests/_data_/script-escape/index.html
{file}{cwd}/tests/_data_/script-escape/script.js
"#,
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // STDOUT should contain every closing SCRIPT tag variant escaped
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r##"<!DOCTYPE html><html><head>
    <title>Script end tag escaping</title>
    <script>var a = "<\/script>";
var b = "<\/SCRIPT>";
var c = "<\/script >";
var d = "<\/script/>";
var e = "<\/scripts>";
</script>
<meta name="robots" content="none"></head>
<body>


</body></html>
"##
        );

        // Exit code should be 0
        out.assert().code(0);
    }
}
