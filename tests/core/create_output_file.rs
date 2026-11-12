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

    #[test]
    fn directories_get_created() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a/b/page.html").display().to_string();

        let (_, created) = create_output_file(&path).unwrap();

        assert_eq!(created, path);
        assert!(std::path::Path::new(&path).is_file());
    }
}

#[cfg(test)]
mod failing {
    use monolith::core::create_output_file;

    #[test]
    fn directories_that_cannot_be_created() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("file"), "").unwrap();
        let path = directory
            .path()
            .join("file/page.html")
            .display()
            .to_string();

        let error = create_output_file(&path).err().unwrap();
        assert!(
            error.to_string().starts_with("could not create directory"),
            "{}",
            error
        );
    }
}
