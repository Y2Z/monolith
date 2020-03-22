use crate::utils;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::env;
use url::ParseError;

#[test]
fn data_to_data_url() {
    let mime = "application/javascript";
    let data = "var word = 'hello';\nalert(word);\n";
    let datauri = utils::data_to_data_url(mime, data.as_bytes());
    assert_eq!(
        &datauri,
        "data:application/javascript;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
    );
}

#[test]
fn detect_mimetype() {
    // Image
    assert_eq!(utils::detect_mimetype(b"GIF87a"), "image/gif");
    assert_eq!(utils::detect_mimetype(b"GIF89a"), "image/gif");
    assert_eq!(utils::detect_mimetype(b"\xFF\xD8\xFF"), "image/jpeg");
    assert_eq!(
        utils::detect_mimetype(b"\x89PNG\x0D\x0A\x1A\x0A"),
        "image/png"
    );
    assert_eq!(utils::detect_mimetype(b"<?xml "), "image/svg+xml");
    assert_eq!(utils::detect_mimetype(b"<svg "), "image/svg+xml");
    assert_eq!(utils::detect_mimetype(b"RIFF....WEBPVP8 "), "image/webp");
    assert_eq!(utils::detect_mimetype(b"\x00\x00\x01\x00"), "image/x-icon");

    // Audio
    assert_eq!(utils::detect_mimetype(b"ID3"), "audio/mpeg");
    assert_eq!(utils::detect_mimetype(b"\xFF\x0E"), "audio/mpeg");
    assert_eq!(utils::detect_mimetype(b"\xFF\x0F"), "audio/mpeg");
    assert_eq!(utils::detect_mimetype(b"OggS"), "audio/ogg");
    assert_eq!(utils::detect_mimetype(b"RIFF....WAVEfmt "), "audio/wav");
    assert_eq!(utils::detect_mimetype(b"fLaC"), "audio/x-flac");

    // Video
    assert_eq!(utils::detect_mimetype(b"RIFF....AVI LIST"), "video/avi");
    assert_eq!(utils::detect_mimetype(b"....ftyp"), "video/mp4");
    assert_eq!(utils::detect_mimetype(b"\x00\x00\x01\x0B"), "video/mpeg");
    assert_eq!(utils::detect_mimetype(b"....moov"), "video/quicktime");
    assert_eq!(utils::detect_mimetype(b"\x1A\x45\xDF\xA3"), "video/webm");
}

#[test]
fn url_has_protocol() {
    // Passing
    assert_eq!(
        utils::url_has_protocol("mailto:somebody@somewhere.com?subject=hello"),
        true
    );
    assert_eq!(utils::url_has_protocol("tel:5551234567"), true);
    assert_eq!(
        utils::url_has_protocol("ftp:user:password@some-ftp-server.com"),
        true
    );
    assert_eq!(utils::url_has_protocol("javascript:void(0)"), true);
    assert_eq!(utils::url_has_protocol("http://news.ycombinator.com"), true);
    assert_eq!(utils::url_has_protocol("https://github.com"), true);
    assert_eq!(
        utils::url_has_protocol("MAILTO:somebody@somewhere.com?subject=hello"),
        true
    );

    // Failing
    assert_eq!(
        utils::url_has_protocol("//some-hostname.com/some-file.html"),
        false
    );
    assert_eq!(
        utils::url_has_protocol("some-hostname.com/some-file.html"),
        false
    );
    assert_eq!(utils::url_has_protocol("/some-file.html"), false);
    assert_eq!(utils::url_has_protocol(""), false);
}

#[test]
fn is_file_url() {
    // Passing
    assert!(utils::is_file_url(
        "file:///home/user/Websites/my-website/index.html"
    ));
    assert!(utils::is_file_url(
        "file:///C:/Documents%20and%20Settings/user/Websites/my-website/assets/images/logo.png"
    ));
    assert!(utils::is_file_url(
        "file:\\\\\\home\\user\\Websites\\my-website\\index.html"
    ));

    // Failing
    assert!(!utils::is_file_url("//kernel.org"));
    assert!(!utils::is_file_url("./index.html"));
    assert!(!utils::is_file_url("some-local-page.htm"));
    assert!(!utils::is_file_url("https://1.2.3.4:80/www/index.html"));
    assert!(!utils::is_file_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
    ));
}

#[test]
fn is_http_url() {
    // Passing
    assert!(utils::is_http_url("https://www.rust-lang.org/"));
    assert!(utils::is_http_url("http://kernel.org"));
    assert!(utils::is_http_url("http:\\\\freebsd.org\\"));

    // Failing
    assert!(!utils::is_http_url("//kernel.org"));
    assert!(!utils::is_http_url("./index.html"));
    assert!(!utils::is_http_url("some-local-page.htm"));
    assert!(!utils::is_http_url("ftp://1.2.3.4/www/index.html"));
    assert!(!utils::is_http_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
    ));
}

#[test]
fn resolve_url() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url("https://www.kernel.org", "../category/signatures.html")?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    let resolved_url = utils::resolve_url("https://www.kernel.org", "category/signatures.html")?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    let resolved_url = utils::resolve_url(
        "saved_page.htm",
        "https://www.kernel.org/category/signatures.html",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    let resolved_url = utils::resolve_url(
        "https://www.kernel.org",
        "//www.kernel.org/theme/images/logos/tux.png",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/theme/images/logos/tux.png"
    );

    let resolved_url = utils::resolve_url(
        "https://www.kernel.org",
        "//another-host.org/theme/images/logos/tux.png",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://another-host.org/theme/images/logos/tux.png"
    );

    let resolved_url = utils::resolve_url(
        "https://www.kernel.org/category/signatures.html",
        "/theme/images/logos/tux.png",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/theme/images/logos/tux.png"
    );

    let resolved_url = utils::resolve_url(
        "https://www.w3schools.com/html/html_iframe.asp",
        "default.asp",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.w3schools.com/html/default.asp"
    );

    let resolved_url = utils::resolve_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
        "https://www.kernel.org/category/signatures.html",
    )?;
    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    let resolved_url = utils::resolve_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
        "//www.w3schools.com/html/html_iframe.asp",
    )
    .unwrap_or(str!());
    assert_eq!(resolved_url.as_str(), "");

    let resolved_url = utils::resolve_url(
        "file:///home/user/Websites/my-website/index.html",
        "assets/images/logo.png",
    )
    .unwrap_or(str!());
    assert_eq!(
        resolved_url.as_str(),
        "file:///home/user/Websites/my-website/assets/images/logo.png"
    );

    let resolved_url = utils::resolve_url(
        "file:\\\\\\home\\user\\Websites\\my-website\\index.html",
        "assets\\images\\logo.png",
    )
    .unwrap_or(str!());
    assert_eq!(
        resolved_url.as_str(),
        "file:///home/user/Websites/my-website/assets/images/logo.png"
    );

    Ok(())
}

#[test]
fn is_data_url() {
    // Passing
    assert!(utils::is_data_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
    ));

    // Failing
    assert!(!utils::is_data_url("https://kernel.org"));
    assert!(!utils::is_data_url("//kernel.org"));
    assert!(!utils::is_data_url(""));
}

#[test]
fn clean_url() {
    assert_eq!(
        utils::clean_url("https://somewhere.com/font.eot#iefix"),
        "https://somewhere.com/font.eot"
    );
    assert_eq!(
        utils::clean_url("https://somewhere.com/font.eot#"),
        "https://somewhere.com/font.eot"
    );
    assert_eq!(
        utils::clean_url("https://somewhere.com/font.eot?#"),
        "https://somewhere.com/font.eot"
    );
}

#[test]
fn data_url_to_text() {
    assert_eq!(
        utils::data_url_to_text("data:text/html;base64,V29yayBleHBhbmRzIHNvIGFzIHRvIGZpbGwgdGhlIHRpbWUgYXZhaWxhYmxlIGZvciBpdHMgY29tcGxldGlvbg=="),
        "Work expands so as to fill the time available for its completion"
    );

    assert_eq!(
        utils::data_url_to_text(
            "data:text/html;utf8,Work expands so as to fill the time available for its completion"
        ),
        "Work expands so as to fill the time available for its completion"
    );

    assert_eq!(
        utils::data_url_to_text(
            "data:text/html,Work expands so as to fill the time available for its completion"
        ),
        "Work expands so as to fill the time available for its completion"
    );

    assert_eq!(
        utils::data_url_to_text(
            " data:text/html;charset=utf-8,Work expands so as to fill the time available for its completion "
        ),
        "Work expands so as to fill the time available for its completion"
    );
}

#[test]
fn decode_url() {
    assert_eq!(
        utils::decode_url(str!(
            "%E6%A4%9C%E3%83%92%E3%83%A0%E8%A7%A3%E5%A1%97%E3%82%83%E3%83%83%20%3D%20%E3%82%B5"
        )),
        "検ヒム解塗ゃッ = サ"
    );

    assert_eq!(utils::decode_url(str!("%20 %20")), "   ");
}

#[test]
fn retrieve_asset() {
    let cache = &mut HashMap::new();
    let client = Client::new();

    let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

    // If both source and target are data URLs,
    // ensure the result contains target data URL
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        "data:text/html;base64,SoUrCe",
        "data:text/html;base64,TaRgEt",
        true,
        "",
        false,
    )
    .unwrap();
    assert_eq!(&data, "data:text/html;base64,TaRgEt");
    assert_eq!(&final_url, "data:text/html;base64,TaRgEt");

    // Media type parameter should not influence data URLs
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        "data:text/html;base64,SoUrCe",
        "data:text/html;base64,TaRgEt",
        true,
        "image/png",
        false,
    )
    .unwrap();
    assert_eq!(&data, "data:text/html;base64,TaRgEt");
    assert_eq!(&final_url, "data:text/html;base64,TaRgEt");

    // Inclusion of local assets from data URL sources should not be allowed
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        "data:text/html;base64,SoUrCe",
        "file:///etc/passwd",
        true,
        "",
        false,
    )
    .unwrap();
    assert_eq!(&data, "");
    assert_eq!(&final_url, "");

    // Inclusion of local assets from remote sources should not be allowed
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        "https://kernel.org/",
        "file:///etc/passwd",
        true,
        "",
        false,
    )
    .unwrap();
    assert_eq!(&data, "");
    assert_eq!(&final_url, "");

    // Inclusion of local assets from local sources should be allowed
    let cwd = env::current_dir().unwrap();
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        &format!(
            "{file}{cwd}/src/tests/data/local-file.html",
            file = file_url_protocol,
            cwd = cwd.to_str().unwrap()
        ),
        &format!(
            "{file}{cwd}/src/tests/data/local-script.js",
            file = file_url_protocol,
            cwd = cwd.to_str().unwrap()
        ),
        true,
        "application/javascript",
        false,
    )
    .unwrap();
    assert_eq!(&data, "data:application/javascript;base64,ZG9jdW1lbnQuYm9keS5zdHlsZS5iYWNrZ3JvdW5kQ29sb3IgPSAiZ3JlZW4iOwpkb2N1bWVudC5ib2R5LnN0eWxlLmNvbG9yID0gInJlZCI7Cg==");
    assert_eq!(
        &final_url,
        &format!(
            "{file}{cwd}/src/tests/data/local-script.js",
            file = file_url_protocol,
            cwd = cwd.to_str().unwrap()
        )
    );
}
