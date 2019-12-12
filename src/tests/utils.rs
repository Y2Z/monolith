use crate::utils::{
    clean_url, data_to_dataurl, detect_mimetype, is_data_url, is_valid_url, resolve_url,
    url_has_protocol,
};
use url::ParseError;

#[test]
fn test_data_to_dataurl() {
    let mime = "application/javascript";
    let data = "var word = 'hello';\nalert(word);\n";
    let datauri = data_to_dataurl(mime, data.as_bytes());
    assert_eq!(
        &datauri,
        "data:application/javascript;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
    );
}

#[test]
fn test_detect_mimetype() {
    // image
    assert_eq!(detect_mimetype(b"GIF87a"), "image/gif");
    assert_eq!(detect_mimetype(b"GIF89a"), "image/gif");
    assert_eq!(detect_mimetype(b"\xFF\xD8\xFF"), "image/jpeg");
    assert_eq!(detect_mimetype(b"\x89PNG\x0D\x0A\x1A\x0A"), "image/png");
    assert_eq!(detect_mimetype(b"<?xml "), "image/svg+xml");
    assert_eq!(detect_mimetype(b"<svg "), "image/svg+xml");
    assert_eq!(detect_mimetype(b"RIFF....WEBPVP8 "), "image/webp");
    assert_eq!(detect_mimetype(b"\x00\x00\x01\x00"), "image/x-icon");
    // audio
    assert_eq!(detect_mimetype(b"ID3"), "audio/mpeg");
    assert_eq!(detect_mimetype(b"\xFF\x0E"), "audio/mpeg");
    assert_eq!(detect_mimetype(b"\xFF\x0F"), "audio/mpeg");
    assert_eq!(detect_mimetype(b"OggS"), "audio/ogg");
    assert_eq!(detect_mimetype(b"RIFF....WAVEfmt "), "audio/wav");
    assert_eq!(detect_mimetype(b"fLaC"), "audio/x-flac");
    // video
    assert_eq!(detect_mimetype(b"RIFF....AVI LIST"), "video/avi");
    assert_eq!(detect_mimetype(b"....ftyp"), "video/mp4");
    assert_eq!(detect_mimetype(b"\x00\x00\x01\x0B"), "video/mpeg");
    assert_eq!(detect_mimetype(b"....moov"), "video/quicktime");
    assert_eq!(detect_mimetype(b"\x1A\x45\xDF\xA3"), "video/webm");
}

#[test]
fn test_url_has_protocol() {
    // succeeding
    assert_eq!(
        url_has_protocol("mailto:somebody@somewhere.com?subject=hello"),
        true
    );
    assert_eq!(url_has_protocol("tel:5551234567"), true);
    assert_eq!(
        url_has_protocol("ftp:user:password@some-ftp-server.com"),
        true
    );
    assert_eq!(url_has_protocol("javascript:void(0)"), true);
    assert_eq!(url_has_protocol("http://news.ycombinator.com"), true);
    assert_eq!(url_has_protocol("https://github.com"), true);
    assert_eq!(
        url_has_protocol("MAILTO:somebody@somewhere.com?subject=hello"),
        true
    );
    // failing
    assert_eq!(
        url_has_protocol("//some-hostname.com/some-file.html"),
        false
    );
    assert_eq!(url_has_protocol("some-hostname.com/some-file.html"), false);
    assert_eq!(url_has_protocol("/some-file.html"), false);
    assert_eq!(url_has_protocol(""), false);
}

#[test]
fn test_is_valid_url() {
    // succeeding
    assert!(is_valid_url("https://www.rust-lang.org/"));
    assert!(is_valid_url("http://kernel.org"));
    // failing
    assert!(!is_valid_url("//kernel.org"));
    assert!(!is_valid_url("./index.html"));
    assert!(!is_valid_url("some-local-page.htm"));
    assert!(!is_valid_url("ftp://1.2.3.4/www/index.html"));
    assert!(!is_valid_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
    ));
}

#[test]
fn test_resolve_url() -> Result<(), ParseError> {
    let resolved_url = resolve_url("https://www.kernel.org", "../category/signatures.html")?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    let resolved_url = resolve_url("https://www.kernel.org", "category/signatures.html")?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    let resolved_url = resolve_url(
        "saved_page.htm",
        "https://www.kernel.org/category/signatures.html",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    let resolved_url = resolve_url(
        "https://www.kernel.org",
        "//www.kernel.org/theme/images/logos/tux.png",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/theme/images/logos/tux.png"
    );

    let resolved_url = resolve_url(
        "https://www.kernel.org",
        "//another-host.org/theme/images/logos/tux.png",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://another-host.org/theme/images/logos/tux.png"
    );

    let resolved_url = resolve_url(
        "https://www.kernel.org/category/signatures.html",
        "/theme/images/logos/tux.png",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/theme/images/logos/tux.png"
    );

    let resolved_url = resolve_url(
        "https://www.w3schools.com/html/html_iframe.asp",
        "default.asp",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.w3schools.com/html/default.asp"
    );

    Ok(())
}

#[test]
fn test_is_data_url() {
    // succeeding
    assert!(
        is_data_url("data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h")
            .unwrap_or(false)
    );
    // failing
    assert!(!is_data_url("https://kernel.org").unwrap_or(false));
    assert!(!is_data_url("//kernel.org").unwrap_or(false));
    assert!(!is_data_url("").unwrap_or(false));
}

#[test]
fn test_clean_url() {
    assert_eq!(
        clean_url("https://somewhere.com/font.eot#iefix"),
        "https://somewhere.com/font.eot"
    );
    assert_eq!(
        clean_url("https://somewhere.com/font.eot#"),
        "https://somewhere.com/font.eot"
    );
    assert_eq!(
        clean_url("https://somewhere.com/font.eot?#"),
        "https://somewhere.com/font.eot"
    );
}
