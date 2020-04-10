use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_parse_text_html_base64() {
    assert_eq!(
        utils::data_url_to_text("data:text/html;base64,V29yayBleHBhbmRzIHNvIGFzIHRvIGZpbGwgdGhlIHRpbWUgYXZhaWxhYmxlIGZvciBpdHMgY29tcGxldGlvbg=="),
        "Work expands so as to fill the time available for its completion"
    );
}

#[test]
fn passing_parse_text_html_utf8() {
    assert_eq!(
        utils::data_url_to_text(
            "data:text/html;utf8,Work expands so as to fill the time available for its completion"
        ),
        "Work expands so as to fill the time available for its completion"
    );
}

#[test]
fn passing_parse_text_html_plaintext() {
    assert_eq!(
        utils::data_url_to_text(
            "data:text/html,Work expands so as to fill the time available for its completion"
        ),
        "Work expands so as to fill the time available for its completion"
    );
}

#[test]
fn passing_parse_text_html_charset_utf_8_between_two_whitespaces() {
    assert_eq!(
        utils::data_url_to_text(
            " data:text/html;charset=utf-8,Work expands so as to fill the time available for its completion "
        ),
        "Work expands so as to fill the time available for its completion"
    );
}

#[test]
fn passing_parse_text_css_url_encoded() {
    assert_eq!(
        utils::data_url_to_text("data:text/css,div{background-color:%23000}"),
        "div{background-color:#000}"
    );
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn failing_just_word_data() {
    assert_eq!(utils::data_url_to_text("data"), "");
}
