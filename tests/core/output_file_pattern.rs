//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use std::path::MAIN_SEPARATOR;

    use monolith::core::output_file_pattern;

    const NAME: &str = "%title%.%extension%";

    #[test]
    fn files() {
        for path in [
            "page.html",
            "page.HTM",
            "archive/%url%.%ext%",
            "archive/%timestamp%/%title%.%extension%",
            "a.mht",
            "b.MHTML",
            "c.warc",
            "d.warc.gz",
            "/home/user/x.html",
        ] {
            assert_eq!(output_file_pattern(path, NAME), path);
        }
    }

    #[test]
    fn directories_ending_with_slashes() {
        assert_eq!(
            output_file_pattern("saved/", NAME),
            "saved/%title%.%extension%"
        );
        assert_eq!(
            output_file_pattern("archive/%extension%/", NAME),
            "archive/%extension%/%title%.%extension%"
        );
        assert_eq!(
            output_file_pattern("page.html/", NAME),
            "page.html/%title%.%extension%"
        );
    }

    #[test]
    fn directories_without_extensions() {
        for path in [
            "saved",
            "archive/%extension%",
            "%title%",
            "notes.txt",
            "~/Downloads",
        ] {
            assert_eq!(
                output_file_pattern(path, NAME),
                format!("{}{}{}", path, MAIN_SEPARATOR, NAME)
            );
        }
    }

    #[test]
    fn extensions_only_count_in_file_names() {
        assert_eq!(
            output_file_pattern("site.html/pages", NAME),
            format!("site.html/pages{}{}", MAIN_SEPARATOR, NAME)
        );
    }

    #[cfg(windows)]
    #[test]
    fn backslashes_on_windows() {
        assert_eq!(
            output_file_pattern("C:\\saved\\", NAME),
            "C:\\saved\\%title%.%extension%"
        );
        assert_eq!(
            output_file_pattern("C:\\saved\\a.html", NAME),
            "C:\\saved\\a.html"
        );
    }
}
