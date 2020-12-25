//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use assert_cmd::prelude::*;
    use std::env;
    use std::process::Command;

    #[test]
    fn add_new_when_provided() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-b")
            .arg("http://localhost:8000/")
            .arg("data:text/html,Hello%2C%20World!")
            .output()
            .unwrap();

        // STDOUT should contain newly added base URL
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\
            <base href=\"http://localhost:8000/\"></base>\
            </head><body>Hello, World!</body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn keep_existing_when_none_provided() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("data:text/html,<base href=\"http://localhost:8000/\" />Hello%2C%20World!")
            .output()
            .unwrap();

        // STDOUT should contain newly added base URL
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\
            <base href=\"http://localhost:8000/\">\
            </head><body>Hello, World!</body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn override_existing_when_provided() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-b")
            .arg("http://localhost/")
            .arg("data:text/html,<base href=\"http://localhost:8000/\" />Hello%2C%20World!")
            .output()
            .unwrap();

        // STDOUT should contain newly added base URL
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\
            <base href=\"http://localhost/\">\
            </head><body>Hello, World!</body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }

    #[test]
    fn remove_existing_when_empty_provided() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg("-b")
            .arg("")
            .arg("data:text/html,<base href=\"http://localhost:8000/\" />Hello%2C%20World!")
            .output()
            .unwrap();

        // STDOUT should contain newly added base URL
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\
            <base href=\"\">\
            </head><body>Hello, World!</body></html>\n"
        );

        // STDERR should be empty
        assert_eq!(std::str::from_utf8(&out.stderr).unwrap(), "");

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }
}
