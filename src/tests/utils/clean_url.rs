use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_removes_fragment() {
    assert_eq!(
        utils::clean_url("https://somewhere.com/font.eot#iefix"),
        "https://somewhere.com/font.eot"
    );
}

#[test]
fn passing_removes_empty_fragment() {
    assert_eq!(
        utils::clean_url("https://somewhere.com/font.eot#"),
        "https://somewhere.com/font.eot"
    );
}

#[test]
fn passing_removes_empty_query_and_empty_fragment() {
    assert_eq!(
        utils::clean_url("https://somewhere.com/font.eot?#"),
        "https://somewhere.com/font.eot"
    );
}

#[test]
fn passing_removes_empty_query_amp_and_empty_fragment() {
    assert_eq!(
        utils::clean_url("https://somewhere.com/font.eot?a=b&#"),
        "https://somewhere.com/font.eot?a=b"
    );
}

#[test]
fn passing_keeps_credentials() {
    assert_eq!(
        utils::clean_url("https://cookie:monster@gibson.internet/"),
        "https://cookie:monster@gibson.internet/"
    );
}
