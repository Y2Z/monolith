use url::ParseError;

use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_from_https_to_level_up_relative() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url("https://www.kernel.org", "../category/signatures.html")?;

    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    Ok(())
}

#[test]
fn passing_from_just_filename_to_full_https_url() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "saved_page.htm",
        "https://www.kernel.org/category/signatures.html",
    )?;

    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    Ok(())
}

#[test]
fn passing_from_https_url_to_url_with_no_protocol() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "https://www.kernel.org",
        "//www.kernel.org/theme/images/logos/tux.png",
    )?;

    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/theme/images/logos/tux.png"
    );

    Ok(())
}

#[test]
fn passing_from_https_url_to_url_with_no_protocol_and_on_different_hostname(
) -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "https://www.kernel.org",
        "//another-host.org/theme/images/logos/tux.png",
    )?;

    assert_eq!(
        resolved_url.as_str(),
        "https://another-host.org/theme/images/logos/tux.png"
    );

    Ok(())
}

#[test]
fn passing_from_https_url_to_relative_root_path() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "https://www.kernel.org/category/signatures.html",
        "/theme/images/logos/tux.png",
    )?;

    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/theme/images/logos/tux.png"
    );

    Ok(())
}

#[test]
fn passing_from_https_to_just_filename() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
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
fn passing_from_data_url_to_https() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
        "https://www.kernel.org/category/signatures.html",
    )?;

    assert_eq!(
        resolved_url.as_str(),
        "https://www.kernel.org/category/signatures.html"
    );

    Ok(())
}

#[test]
fn passing_from_data_url_to_data_url() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
        "data:text/html;base64,PGEgaHJlZj0iaW5kZXguaHRtbCI+SG9tZTwvYT4K",
    )?;

    assert_eq!(
        resolved_url.as_str(),
        "data:text/html;base64,PGEgaHJlZj0iaW5kZXguaHRtbCI+SG9tZTwvYT4K"
    );

    Ok(())
}

#[test]
fn passing_from_file_url_to_relative_path() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "file:///home/user/Websites/my-website/index.html",
        "assets/images/logo.png",
    )
    .unwrap_or(str!());

    assert_eq!(
        resolved_url.as_str(),
        "file:///home/user/Websites/my-website/assets/images/logo.png"
    );

    Ok(())
}

#[test]
fn passing_from_file_url_to_relative_path_with_backslashes() -> Result<(), ParseError> {
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
fn passing_from_data_url_to_file_url() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
        "file:///etc/passwd",
    )
    .unwrap_or(str!());

    assert_eq!(resolved_url.as_str(), "file:///etc/passwd");

    Ok(())
}

#[test]
fn passing_preserve_fragment() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "http://doesnt-matter.local/",
        "css/fonts/fontmarvelous.svg#fontmarvelous",
    )
    .unwrap_or(str!());

    assert_eq!(
        resolved_url.as_str(),
        "http://doesnt-matter.local/css/fonts/fontmarvelous.svg#fontmarvelous"
    );

    Ok(())
}

#[test]
fn passing_resolve_from_file_url_to_file_url() -> Result<(), ParseError> {
    let resolved_url = if cfg!(windows) {
        utils::resolve_url("file:///c:/index.html", "file:///c:/image.png").unwrap_or(str!())
    } else {
        utils::resolve_url("file:///tmp/index.html", "file:///tmp/image.png").unwrap_or(str!())
    };

    assert_eq!(
        resolved_url.as_str(),
        if cfg!(windows) {
            "file:///c:/image.png"
        } else {
            "file:///tmp/image.png"
        }
    );

    Ok(())
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn failing_from_data_url_to_url_with_no_protocol() -> Result<(), ParseError> {
    let resolved_url = utils::resolve_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h",
        "//www.w3schools.com/html/html_iframe.asp",
    )
    .unwrap_or(str!());

    assert_eq!(resolved_url.as_str(), "");

    Ok(())
}
