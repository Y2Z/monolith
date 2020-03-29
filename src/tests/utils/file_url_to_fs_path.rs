use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_remove_protocl_and_fragment() {
    if cfg!(windows) {
        assert_eq!(
            utils::file_url_to_fs_path("file:///C:/documents/some-path/some-file.svg#fragment"),
            "C:\\documents\\some-path\\some-file.svg"
        );
    } else {
        assert_eq!(
            utils::file_url_to_fs_path("file:///tmp/some-path/some-file.svg#fragment"),
            "/tmp/some-path/some-file.svg"
        );
    }
}
