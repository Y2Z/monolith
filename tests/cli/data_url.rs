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

    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn isolate_data_url() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-I")
            .arg("data:text/html,Hello%2C%20World!")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain isolated HTML
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-eval' 'unsafe-inline' data:;\"></meta>\
            </head><body>Hello, World!</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn remove_css_from_data_url() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-c")
            .arg("data:text/html,<style>body{background-color:pink}</style>Hello")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML with no CSS
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none';\"></meta>\
            <style></style>\
            </head><body>Hello</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn remove_fonts_from_data_url() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-F")
            .arg("data:text/html,<style>@font-face { font-family: myFont; src: url(font.woff); }</style>Hi")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML with no web fonts
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"font-src 'none';\"></meta>\
            <style></style>\
            </head><body>Hi</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn remove_frames_from_data_url() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-f")
            .arg("data:text/html,<iframe src=\"https://duckduckgo.com\"></iframe>Hi")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML with no iframes
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"frame-src 'none'; child-src 'none';\"></meta>\
            </head><body><iframe src=\"\"></iframe>Hi</body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn remove_images_from_data_url() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-i")
            .arg("data:text/html,<img src=\"https://google.com\"/>Hi")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML with no images
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            format!(
                "<html>\
                <head>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"img-src data:;\"></meta>\
                </head>\
                <body>\
                <img src=\"{empty_image}\">\
                Hi\
                </body>\
                </html>\n",
                empty_image = EMPTY_IMAGE_DATA_URL,
            )
        );

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn remove_js_from_data_url() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("-j")
            .arg("data:text/html,<script>alert(2)</script>Hi")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML with no JS
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html>\
            <head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'none';\"></meta>\
            <script></script></head>\
            <body>Hi</body>\
            </html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
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
    use std::process::Command;

    #[test]
    fn bad_input_data_url() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd.arg("data:,Hello%2C%20World!").output().unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain text
        assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, World!\n");

        // Exit code should be 0
        out.assert().code(0);
    }

    #[test]
    fn security_disallow_local_assets_within_data_url_targets() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let out = cmd
            .arg("-M")
            .arg("data:text/html,%3Cscript%20src=\"src/tests/data/basic/local-script.js\"%3E%3C/script%3E")
            .output()
            .unwrap();

        // STDERR should be empty
        assert_eq!(String::from_utf8_lossy(&out.stderr), "");

        // STDOUT should contain HTML without contents of local JS file
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "<html><head><script src=\"data:text/javascript;base64,\"></script></head><body></body></html>\n"
        );

        // Exit code should be 0
        out.assert().code(0);
    }
}
