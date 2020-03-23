use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_unix_file_url() {
    assert!(utils::is_file_url(
        "file:///home/user/Websites/my-website/index.html"
    ));
}

#[test]
fn passing_windows_file_url() {
    assert!(utils::is_file_url(
        "file:///C:/Documents%20and%20Settings/user/Websites/my-website/assets/images/logo.png"
    ));
}

#[test]
fn passing_unix_url_with_backslashes() {
    assert!(utils::is_file_url(
        "file:\\\\\\home\\user\\Websites\\my-website\\index.html"
    ));
}

#[test]
fn passing_windows_file_url_with_backslashes() {
    assert!(utils::is_file_url(
        "file:\\\\\\C:\\Documents%20and%20Settings\\user\\Websites\\my-website\\assets\\images\\logo.png"
    ));
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn failing_url_with_no_protocl() {
    assert!(!utils::is_file_url("//kernel.org"));
}

#[test]
fn failing_dot_slash_filename() {
    assert!(!utils::is_file_url("./index.html"));
}

#[test]
fn failing_just_filename() {
    assert!(!utils::is_file_url("some-local-page.htm"));
}

#[test]
fn failing_https_ip_port_url() {
    assert!(!utils::is_file_url("https://1.2.3.4:80/www/index.html"));
}

#[test]
fn failing_data_url() {
    assert!(!utils::is_file_url(
        "data:text/html;base64,V2VsY29tZSBUbyBUaGUgUGFydHksIDxiPlBhbDwvYj4h"
    ));
}

#[test]
fn failing_just_word_file() {
    assert!(!utils::is_file_url("file"));
}
