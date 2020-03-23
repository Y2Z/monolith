use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_mailto() {
    assert!(utils::url_has_protocol(
        "mailto:somebody@somewhere.com?subject=hello"
    ));
}

#[test]
fn passing_tel() {
    assert!(utils::url_has_protocol("tel:5551234567"));
}

#[test]
fn passing_ftp_no_slashes() {
    assert!(utils::url_has_protocol("ftp:some-ftp-server.com"));
}

#[test]
fn passing_ftp_with_credentials() {
    assert!(utils::url_has_protocol(
        "ftp://user:password@some-ftp-server.com"
    ));
}

#[test]
fn passing_javascript() {
    assert!(utils::url_has_protocol("javascript:void(0)"));
}

#[test]
fn passing_http() {
    assert!(utils::url_has_protocol("http://news.ycombinator.com"));
}

#[test]
fn passing_https() {
    assert!(utils::url_has_protocol("https://github.com"));
}

#[test]
fn passing_mailto_uppercase() {
    assert!(utils::url_has_protocol(
        "MAILTO:somebody@somewhere.com?subject=hello"
    ));
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn failing_url_with_no_protocol() {
    assert!(!utils::url_has_protocol(
        "//some-hostname.com/some-file.html"
    ));
}

#[test]
fn failing_relative_path() {
    assert!(!utils::url_has_protocol("some-hostname.com/some-file.html"));
}

#[test]
fn failing_relative_to_root_path() {
    assert!(!utils::url_has_protocol("/some-file.html"));
}

#[test]
fn failing_empty_string() {
    assert!(!utils::url_has_protocol(""));
}
