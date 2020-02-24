use assert_cmd::prelude::*;
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
fn bad_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    let out = cmd.arg("kernel.org").output().unwrap();

    // STDOUT should be empty
    assert_eq!(std::str::from_utf8(&out.stdout).unwrap(), "");

    // STDERR should contain error description
    assert_eq!(
        std::str::from_utf8(&out.stderr).unwrap(),
        "Only HTTP(S) or data URLs are supported but got: kernel.org\n"
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
        "<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-inline' data:;\"></meta></head><body>Hello, World!</body></html>\n"
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
        "<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none';\"></meta><style></style></head><body>Hello</body></html>\n"
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
        "<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"frame-src 'none';child-src 'none';\"></meta></head><body><iframe src=\"\"></iframe>Hi</body></html>\n"
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
        "<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"img-src data:;\"></meta></head><body><img src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\">Hi</body></html>\n"
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
        "<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'none';\"></meta><script></script></head><body>Hi</body></html>\n"
    );

    // STDERR should be empty
    assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

    // The exit code should be 0
    out.assert().code(0);

    Ok(())
}
