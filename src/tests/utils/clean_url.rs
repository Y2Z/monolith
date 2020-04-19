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
