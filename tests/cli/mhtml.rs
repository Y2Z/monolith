//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use assert_cmd::cargo_bin_cmd;
    use assert_cmd::prelude::*;
    use std::env;
    use std::path::MAIN_SEPARATOR;

    #[test]
    fn mhtml_file_target() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let out = cmd
            .arg("-M")
            .arg(format!(
                "tests{s}_data_{s}mhtml{s}chrome.mhtml",
                s = MAIN_SEPARATOR
            ))
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.starts_with("<!DOCTYPE html>"));
        assert!(stdout.contains(r#"<img src="data:image/png"#));
        assert!(stdout.contains(r#"<iframe src="data:text/html"#));
        assert!(!stdout.contains("cid:"));

        // Assets come from the archive, nothing gets requested over the network
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("https://example.com/page/style.css\n"));
        assert!(stderr.contains("https://example.com/missing.png (not found in archive)\n"));

        // Exit code should be 0
        out.assert().code(0);
    }
}
