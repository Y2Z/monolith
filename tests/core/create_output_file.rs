#[cfg(test)]
mod passing {
    use std::fs;
    use std::io::Write;

    use monolith::core::create_output_file;

    #[test]
    fn never_replaces_anything() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("page.html").display().to_string();

        for (number, expected) in ["page.html", "page (2).html", "page (3).html"]
            .iter()
            .enumerate()
        {
            let (mut file, created) = create_output_file(&path).unwrap();
            file.write_all(format!("{}", number).as_bytes()).unwrap();

            assert_eq!(
                created,
                directory.path().join(expected).display().to_string()
            );
        }

        assert_eq!(fs::read_to_string(&path).unwrap(), "0");
        assert_eq!(
            fs::read_to_string(directory.path().join("page (3).html")).unwrap(),
            "2"
        );
    }
}

#[cfg(test)]
mod failing {
    use monolith::core::create_output_file;

    #[test]
    fn directories_that_are_not_there() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join("missing/page.html")
            .display()
            .to_string();

        assert!(create_output_file(&path).is_err());
    }
}
