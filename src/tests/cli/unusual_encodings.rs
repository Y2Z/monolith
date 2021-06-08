//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use assert_cmd::prelude::*;
    use encoding_rs::Encoding;
    use std::env;
    use std::path::MAIN_SEPARATOR;
    use std::process::{Command, Stdio};

    #[test]
    fn properly_save_document_with_gb2312() {
        let cwd = env::current_dir().unwrap();
        let cwd_normalized: String = str!(cwd.to_str().unwrap()).replace("\\", "/");
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg(format!(
                "src{s}tests{s}data{s}unusual_encodings{s}gb2312.html",
                s = MAIN_SEPARATOR
            ))
            .output()
            .unwrap();
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

        // STDERR should contain only the target file
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "{file}{cwd}/src/tests/data/unusual_encodings/gb2312.html\n",
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // STDOUT should contain original document without any modificatons
        let s: String;
        if let Some(encoding) = Encoding::for_label(b"gb2312") {
            let (string, _, _) = encoding.decode(&out.stdout);
            s = string.to_string();
        } else {
            s = String::from_utf8_lossy(&out.stdout).to_string();
        }
        assert_eq!(
            s,
            "<html>\
                <head>\n    \
                    <meta http-equiv=\"content-type\" content=\"text/html;charset=GB2312\">\n    \
                    <title>近七成人减少线下需求\u{3000}银行数字化转型提速--经济·科技--人民网 </title>\n\
                </head>\n\
                <body>\n    \
                    <h1>近七成人减少线下需求\u{3000}银行数字化转型提速</h1>\n\n\n\
                </body>\
            </html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn properly_save_document_with_gb2312_from_stdin() {
        let mut echo = Command::new("cat")
            .arg(format!(
                "src{s}tests{s}data{s}unusual_encodings{s}gb2312.html",
                s = MAIN_SEPARATOR
            ))
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let echo_out = echo.stdout.take().unwrap();
        echo.wait().unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.stdin(echo_out);
        let out = cmd.arg("-M").arg("-").output().unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML created out of STDIN
        let s: String;
        if let Some(encoding) = Encoding::for_label(b"gb2312") {
            let (string, _, _) = encoding.decode(&out.stdout);
            s = string.to_string();
        } else {
            s = String::from_utf8_lossy(&out.stdout).to_string();
        }
        assert_eq!(
            s,
            "<html>\
                <head>\n    \
                    <meta http-equiv=\"content-type\" content=\"text/html;charset=GB2312\">\n    \
                    <title>近七成人减少线下需求\u{3000}银行数字化转型提速--经济·科技--人民网 </title>\n\
                </head>\n\
                <body>\n    \
                    <h1>近七成人减少线下需求\u{3000}银行数字化转型提速</h1>\n\n\n\
                </body>\
            </html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn properly_save_document_with_gb2312_custom_charset() {
        let cwd = env::current_dir().unwrap();
        let cwd_normalized: String = str!(cwd.to_str().unwrap()).replace("\\", "/");
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-C")
            .arg("utf8")
            .arg(format!(
                "src{s}tests{s}data{s}unusual_encodings{s}gb2312.html",
                s = MAIN_SEPARATOR
            ))
            .output()
            .unwrap();
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

        // STDERR should contain only the target file
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "{file}{cwd}/src/tests/data/unusual_encodings/gb2312.html\n",
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // STDOUT should contain original document without any modificatons
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).to_string(),
            "<html>\
                <head>\n    \
                    <meta http-equiv=\"content-type\" content=\"text/html;charset=utf8\">\n    \
                    <title>近七成人减少线下需求\u{3000}银行数字化转型提速--经济·科技--人民网 </title>\n\
                </head>\n\
                <body>\n    \
                    <h1>近七成人减少线下需求\u{3000}银行数字化转型提速</h1>\n\n\n\
                </body>\
            </html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn properly_save_document_with_gb2312_custom_charset_bad() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-C")
            .arg("utf0")
            .arg(format!(
                "src{s}tests{s}data{s}unusual_encodings{s}gb2312.html",
                s = MAIN_SEPARATOR
            ))
            .output()
            .unwrap();

        // STDERR should contain error message
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            "Unknown encoding: utf0\n"
        );

        // STDOUT should be empty
        assert_eq!(String::from_utf8_lossy(&out.stdout).to_string(), "");

        // Exit code should be 1
        out.assert().code(1);
    }
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use assert_cmd::prelude::*;
    use std::env;
    use std::path::MAIN_SEPARATOR;
    use std::process::Command;

    #[test]
    fn change_iso88591_to_utf8_to_properly_display_html_entities() {
        let cwd = env::current_dir().unwrap();
        let cwd_normalized: String = str!(cwd.to_str().unwrap()).replace("\\", "/");
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg(format!(
                "src{s}tests{s}data{s}unusual_encodings{s}iso-8859-1.html",
                s = MAIN_SEPARATOR
            ))
            .output()
            .unwrap();
        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

        // STDERR should contain only the target file
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            format!(
                "{file}{cwd}/src/tests/data/unusual_encodings/iso-8859-1.html\n",
                file = file_url_protocol,
                cwd = cwd_normalized,
            )
        );

        // STDOUT should contain original document but with UTF-8 charset
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html>\
                <head>\n        \
                    <meta http-equiv=\"Content-Type\" content=\"text/html; charset=iso-8859-1\">\n    \
                </head>\n    \
                <body>\n        \
                    � Some Company\n    \
                \n\n</body>\
            </html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }
}
