use crate::css;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_empty_input_single_quotes() {
    assert_eq!(css::enquote(str!(""), false), "''");
}

#[test]
fn passing_empty_input_double_quotes() {
    assert_eq!(css::enquote(str!(""), true), "\"\"");
}

#[test]
fn passing_apostrophes_single_quotes() {
    assert_eq!(
        css::enquote(str!("It's a lovely day, don't you think?"), false),
        "'It\\'s a lovely day, don\\'t you think?'"
    );
}

#[test]
fn passing_apostrophes_double_quotes() {
    assert_eq!(
        css::enquote(str!("It's a lovely day, don't you think?"), true),
        "\"It's a lovely day, don't you think?\""
    );
}

#[test]
fn passing_feet_and_inches_single_quotes() {
    assert_eq!(
        css::enquote(str!("5'2\", 6'5\""), false),
        "'5\\'2\", 6\\'5\"'"
    );
}

#[test]
fn passing_feet_and_inches_double_quotes() {
    assert_eq!(
        css::enquote(str!("5'2\", 6'5\""), true),
        "\"5'2\\\", 6'5\\\"\""
    );
}
