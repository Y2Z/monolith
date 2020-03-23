use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_http_url() {
    assert!(utils::is_http_url("http://kernel.org"));
}

#[test]
fn passing_https_url() {
    assert!(utils::is_http_url("https://www.rust-lang.org/"));
}

#[test]
fn passing_http_url_with_backslashes() {
    assert!(utils::is_http_url("http:\\\\freebsd.org\\"));
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn failing_url_with_no_protocol() {
    assert!(!utils::is_http_url("//kernel.org"));
}

#[test]
fn failing_dot_slash_filename() {
    assert!(!utils::is_http_url("./index.html"));
}

#[test]
fn failing_just_filename() {
    assert!(!utils::is_http_url("some-local-page.htm"));
}

#[test]
fn failing_https_ip_port_url() {
    assert!(!utils::is_http_url("ftp://1.2.3.4/www/index.html"));
}

#[test]
fn failing_data_url() {
    assert!(!utils::is_http_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
    ));
}
