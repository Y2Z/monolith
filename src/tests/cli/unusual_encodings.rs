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
    fn change_encoding_to_utf_8() -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir().unwrap();
        let cwd_normalized: String = str!(cwd.to_str().unwrap()).replace("\\", "/");
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        let out = cmd
            .arg("-M")
            .arg(if cfg!(windows) {
                "src\\tests\\data\\unusual_encodings\\iso-8859-1.html"
            } else {
                "src/tests/data/unusual_encodings/iso-8859-1.html"
            })
            .output()
            .unwrap();
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

        // STDOUT should contain newly added base URL
        assert_eq!(
            std::str::from_utf8(&out.stdout).unwrap(),
            "<html><head>\n        <meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n    </head>\n    <body>\n        © Some Company\n    \n\n</body></html>\n"
        );

        // STDERR should contain only the target file
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "{file}{cwd}/src/tests/data/unusual_encodings/iso-8859-1.html\n",
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // The exit code should be 0
        out.assert().code(0);

        Ok(())
    }
}
