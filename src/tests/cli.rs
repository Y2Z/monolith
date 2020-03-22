use assert_cmd::prelude::*;
use std::env;
use std::process::Command;

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

#[test]
fn bad_input_data_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd.arg("data:,Hello%2C%20World!").output().unwrap();

    // STDOUT should contain HTML
    assert_eq!(std::str::from_utf8(&out.stdout).unwrap(), "");

    // STDERR should contain error description
    assert_eq!(
        std::str::from_utf8(&out.stderr).unwrap(),
        "Unsupported data URL input\n"
    );

    // The exit code should be 1
    out.assert().code(1);

    Ok(())
}

#[test]
fn isolate_data_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd
        .arg("-I")
        .arg("data:text/html,Hello%2C%20World!")
        .output()
        .unwrap();

    // STDOUT should contain isolated HTML
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<html><head>\
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-inline' data:;\"></meta>\
</head><body>Hello, World!</body></html>\n"
    );

    // STDERR should be empty
    assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}

#[test]
fn remove_css_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd
        .arg("-c")
        .arg("data:text/html,<style>body{background-color:pink}</style>Hello")
        .output()
        .unwrap();

    // STDOUT should contain HTML with no CSS
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<html><head>\
<meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none';\"></meta>\
<style></style>\
</head><body>Hello</body></html>\n"
    );

    // STDERR should be empty
    assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}

#[test]
fn remove_frames_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd
        .arg("-f")
        .arg("data:text/html,<iframe src=\"https://google.com\"></iframe>Hi")
        .output()
        .unwrap();

    // STDOUT should contain HTML with no iframes
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<html><head>\
<meta http-equiv=\"Content-Security-Policy\" content=\"frame-src 'none';child-src 'none';\"></meta>\
</head><body><iframe src=\"\"></iframe>Hi</body></html>\n"
    );

    // STDERR should be empty
    assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}

#[test]
fn remove_images_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd
        .arg("-i")
        .arg("data:text/html,<img src=\"https://google.com\"/>Hi")
        .output()
        .unwrap();

    // STDOUT should contain HTML with no images
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<html>\
<head>\
<meta http-equiv=\"Content-Security-Policy\" content=\"img-src data:;\"></meta>\
</head>\
<body>\
<img src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\">\
Hi\
</body>\
</html>\n"
    );

    // STDERR should be empty
    assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}

#[test]
fn remove_js_from_data_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd
        .arg("-j")
        .arg("data:text/html,<script>alert(2)</script>Hi")
        .output()
        .unwrap();

    // STDOUT should contain HTML with no JS
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<html>\
<head>\
<meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'none';\"></meta>\
<script></script></head>\
<body>Hi</body>\
</html>\n"
    );

    // STDERR should be empty
    assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}

#[test]
fn local_file_target_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let cwd_normalized: String =
        str!(env::current_dir().unwrap().to_str().unwrap()).replace("\\", "/");
    let out = cmd
        .arg(if cfg!(windows) {
            "src\\tests\\data\\local-file.html"
        } else {
            "src/tests/data/local-file.html"
        })
        .output()
        .unwrap();
    let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

    // STDOUT should contain HTML from the local file
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<!DOCTYPE html><html lang=\"en\"><head>\n  \
<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n  \
<title>Local HTML file</title>\n  \
<link href=\"data:text/css;base64,Ym9keSB7CiAgICBiYWNrZ3JvdW5kLWNvbG9yOiAjMDAwOwogICAgY29sb3I6ICNmZmY7Cn0K\" rel=\"stylesheet\" type=\"text/css\">\n  \
<link href=\"data:text/css;base64,\" rel=\"stylesheet\" type=\"text/css\">\n</head>\n\n<body>\n  \
<img alt=\"\" src=\"\">\n  \
<a href=\"file://local-file.html/\">Tricky href</a>\n  \
<a href=\"https://github.com/Y2Z/monolith\">Remote URL</a>\n  \
<script src=\"data:application/javascript;base64,ZG9jdW1lbnQuYm9keS5zdHlsZS5iYWNrZ3JvdW5kQ29sb3IgPSAiZ3JlZW4iOwpkb2N1bWVudC5ib2R5LnN0eWxlLmNvbG9yID0gInJlZCI7Cg==\"></script>\n\n\n\n\
</body></html>\n"
    );

    // STDERR should contain list of retrieved file URLs
    assert_eq!(
        std::str::from_utf8(&out.stderr).unwrap(),
        format!(
            "{file}{cwd}/src/tests/data/local-file.html\n\
{file}{cwd}/src/tests/data/local-style.css\n\
{file}{cwd}/src/tests/data/local-script.js\n",
            file = file_url_protocol,
            cwd = cwd_normalized
        )
    );

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}

#[test]
fn local_file_target_input_absolute_target_path() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir().unwrap();
    let cwd_normalized: String =
        str!(env::current_dir().unwrap().to_str().unwrap()).replace("\\", "/");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd
        .arg("-jciI")
        .arg(if cfg!(windows) {
            format!(
                "{cwd}\\src\\tests\\data\\local-file.html",
                cwd = cwd.to_str().unwrap()
            )
        } else {
            format!(
                "{cwd}/src/tests/data/local-file.html",
                cwd = cwd.to_str().unwrap()
            )
        })
        .output()
        .unwrap();
    let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

    // STDOUT should contain HTML from the local file
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<!DOCTYPE html><html lang=\"en\"><head>\
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-inline' data:; style-src 'none'; script-src 'none'; img-src data:;\"></meta>\n  \
<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n  \
<title>Local HTML file</title>\n  \
<link href=\"\" rel=\"stylesheet\" type=\"text/css\">\n  \
<link href=\"\" rel=\"stylesheet\" type=\"text/css\">\n</head>\n\n<body>\n  \
<img alt=\"\" src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\">\n  \
<a href=\"file://local-file.html/\">Tricky href</a>\n  \
<a href=\"https://github.com/Y2Z/monolith\">Remote URL</a>\n  \
<script src=\"\"></script>\n\n\n\n\
</body></html>\n"
    );

    // STDERR should contain only the target file
    assert_eq!(
        std::str::from_utf8(&out.stderr).unwrap(),
        format!(
            "{file}{cwd}/src/tests/data/local-file.html\n",
            file = file_url_protocol,
            cwd = cwd_normalized,
        )
    );

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}

#[test]
fn local_file_url_target_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let cwd = env::current_dir().unwrap();
    let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };
    let out = cmd
        .arg("-cji")
        .arg(if cfg!(windows) {
            format!(
                "{file}{cwd}\\src\\tests\\data\\local-file.html",
                file = file_url_protocol,
                cwd = cwd.to_str().unwrap(),
            )
        } else {
            format!(
                "{file}{cwd}/src/tests/data/local-file.html",
                file = file_url_protocol,
                cwd = cwd.to_str().unwrap(),
            )
        })
        .output()
        .unwrap();

    // STDOUT should contain HTML from the local file
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<!DOCTYPE html><html lang=\"en\"><head>\
<meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none'; script-src 'none'; img-src data:;\"></meta>\n  \
<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n  \
<title>Local HTML file</title>\n  \
<link href=\"\" rel=\"stylesheet\" type=\"text/css\">\n  \
<link href=\"\" rel=\"stylesheet\" type=\"text/css\">\n</head>\n\n<body>\n  \
<img alt=\"\" src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\">\n  \
<a href=\"file://local-file.html/\">Tricky href</a>\n  \
<a href=\"https://github.com/Y2Z/monolith\">Remote URL</a>\n  \
<script src=\"\"></script>\n\n\n\n\
</body></html>\n"
    );

    // STDERR should contain list of retrieved file URLs
    assert_eq!(
        std::str::from_utf8(&out.stderr).unwrap(),
        if cfg!(windows) {
            format!(
                "{file}{cwd}\\src\\tests\\data\\local-file.html\n",
                file = file_url_protocol,
                cwd = cwd.to_str().unwrap(),
            )
        } else {
            format!(
                "{file}{cwd}/src/tests/data/local-file.html\n",
                file = file_url_protocol,
                cwd = cwd.to_str().unwrap(),
            )
        }
    );

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}

#[test]
fn security_disallow_local_assets_within_data_url_targets() -> Result<(), Box<dyn std::error::Error>>
{
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd
        .arg("data:text/html,%3Cscript%20src=\"src/tests/data/local-script.js\"%3E%3C/script%3E")
        .output()
        .unwrap();

    // STDOUT should contain HTML with no JS in it
    assert_eq!(
        std::str::from_utf8(&out.stdout).unwrap(),
        "<html><head><script src=\"\"></script></head><body></body></html>\n"
    );

    // STDERR should be empty
    assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}
