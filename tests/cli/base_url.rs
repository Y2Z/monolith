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
    fn add_new_when_provided() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-b")
            .arg("http://localhost:30701/")
            .arg("data:text/html,Hello%2C%20World!")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain newly added base URL
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head>\
            <base href=\"http://localhost:30701/\"></base>\
            </head><body>Hello, World!</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn keep_existing_when_none_provided() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("data:text/html,<base href=\"http://localhost:30701/\" />Hello%2C%20World!")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain newly added base URL
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head>\
            <base href=\"http://localhost:30701/\">\
            </head><body>Hello, World!</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn override_existing_when_provided() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-b")
            .arg("http://localhost/")
            .arg("data:text/html,<base href=\"http://localhost:30701/\" />Hello%2C%20World!")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain newly added base URL
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head>\
            <base href=\"http://localhost/\">\
            </head><body>Hello, World!</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn set_existing_to_empty_when_empty_provided() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-b")
            .arg("")
            .arg("data:text/html,<base href=\"http://localhost:30701/\" />Hello%2C%20World!")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain newly added base URL
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head>\
            <base href=\"\">\
            </head><body>Hello, World!</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }
}
