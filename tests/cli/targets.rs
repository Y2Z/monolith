//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use std::fs;
    use std::path::Path;
    use std::process::Output;

    use tempfile::TempDir;

    /// Pages that link to each other, and to something that doesn't get saved.
    fn site() -> TempDir {
        let site = tempfile::tempdir().unwrap();
        fs::write(
            site.path().join("a.html"),
            "<html><head><title>Page A</title></head><body>\
             <a href=\"b.html\">B</a><a href=\"b.html#x\">X</a><a href=\"c.html\">C</a>\
             <a href=\"#top\">top</a></body></html>",
        )
        .unwrap();
        fs::write(
            site.path().join("b.html"),
            "<html><head><title>Page B</title></head><body id=\"x\"><a href=\"a.html\">A</a></body></html>",
        )
        .unwrap();
        fs::write(site.path().join("untitled.html"), "<p>No title</p>").unwrap();

        site
    }

    /// Runs monolith the way it runs by default, saving into given directory.
    fn run(directory: &Path, args: &[&str]) -> Output {
        assert_cmd::cargo_bin_cmd!(env!("CARGO_PKG_NAME"))
            .current_dir(directory)
            .args(args)
            .output()
            .unwrap()
    }

    fn target(site: &TempDir, name: &str) -> String {
        site.path().join(name).display().to_string()
    }

    fn read(directory: &Path, name: &str) -> String {
        fs::read_to_string(directory.join(name)).unwrap()
    }

    fn names(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn documents_get_saved_into_files_named_after_them() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(saved.path(), &["-q", &target(&site, "a.html")]);

        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(names(saved.path()), vec!["Page A.html"]);
    }

    #[test]
    fn untitled_documents_get_named_after_their_urls() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(saved.path(), &["-q", &target(&site, "untitled.html")]);

        assert!(output.status.success());
        assert_eq!(names(saved.path()), vec!["untitled.html"]);
    }

    #[test]
    fn several_targets_link_to_each_other() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &["-q", &target(&site, "a.html"), &target(&site, "b.html")],
        );

        assert!(output.status.success());
        assert_eq!(names(saved.path()), vec!["Page A.html", "Page B.html"]);
        let a = read(saved.path(), "Page A.html");
        assert!(a.contains("<a href=\"Page%20B.html\">B</a>"));
        assert!(a.contains("<a href=\"Page%20B.html#x\">X</a>"));
        assert!(a.contains("<a href=\"#top\">top</a>"));
        // What doesn't get saved stays where it is
        assert!(a.contains("c.html\">C</a>"));
        assert!(a.contains("<a href=\"file://"));
        assert!(read(saved.path(), "Page B.html").contains("<a href=\"Page%20A.html\">A</a>"));
    }

    #[test]
    fn links_lead_to_numbered_files() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());
        fs::write(saved.path().join("Page A.html"), "already there").unwrap();

        let output = run(
            saved.path(),
            &["-q", &target(&site, "a.html"), &target(&site, "b.html")],
        );

        assert!(output.status.success());
        assert_eq!(read(saved.path(), "Page A.html"), "already there");
        assert!(
            read(saved.path(), "Page B.html").contains("<a href=\"Page%20A%20(2).html\">A</a>")
        );
    }

    #[test]
    fn same_target_gets_saved_once() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &["-q", &target(&site, "a.html"), &target(&site, "a.html")],
        );

        assert!(output.status.success());
        assert_eq!(names(saved.path()), vec!["Page A.html"]);
    }

    #[test]
    fn failing_target_does_not_stop_others() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &[&target(&site, "a.html"), &site.path().display().to_string()],
        );

        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(&format!("Error: {}: local target", site.path().display()))
        );
        assert_eq!(names(saved.path()), vec!["Page A.html"]);
    }

    #[test]
    fn several_documents_do_not_go_to_stdout() {
        let site = site();

        let output = run(
            site.path(),
            &[
                "-o",
                "-",
                &target(&site, "a.html"),
                &target(&site, "b.html"),
            ],
        );

        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Error: several documents can't all go to STDOUT, they can as WARC\n"
        );
    }

    #[test]
    fn warc_takes_all_targets() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &[
                "-q",
                "-F",
                "WARC",
                &target(&site, "a.html"),
                &target(&site, "b.html"),
            ],
        );

        assert!(output.status.success());
        assert_eq!(names(saved.path()), vec!["Page A.warc"]);
        let warc = read(saved.path(), "Page A.warc");
        // Each target starts a part of its own, the way concatenated archives do
        assert_eq!(warc.matches("WARC-Type: warcinfo\r\n").count(), 2);
        assert!(warc.contains("<title>Page A</title>"));
        assert!(warc.contains("<title>Page B</title>"));
    }

    #[test]
    fn last_output_option_wins() {
        let (site, saved) = (site(), tempfile::tempdir().unwrap());

        let output = run(
            saved.path(),
            &[
                "-q",
                "-o",
                "first.html",
                "-o",
                "-",
                &target(&site, "a.html"),
            ],
        );

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("<title>Page A</title>"));
        assert!(names(saved.path()).is_empty());
    }
}
