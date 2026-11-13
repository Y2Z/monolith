//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use assert_cmd::prelude::*;
    use std::path::Path;

    fn save(arguments: &[&str]) -> String {
        let mut cmd = crate::cli::monolith();
        let path_html: &Path = Path::new("tests/_data_/csp/index.html");
        let out = cmd
            .args(arguments)
            .arg(path_html.as_os_str())
            .output()
            .unwrap();
        let html = crate::cli::without_metadata(&out.stdout);
        out.assert().code(0);
        html
    }

    #[test]
    fn what_the_policy_does_not_allow_is_left_out() {
        let html = save(&["-q"]);

        assert!(html.contains("var kept = 1;"), "{html}");
        assert!(!html.contains("var dropped"), "{html}");
        assert!(!html.contains("color: red"), "{html}");
        assert!(!html.contains("color: blue"), "{html}");
        assert!(!html.contains("onclick"), "{html}");
        assert!(!html.contains("example.com"), "{html}");
        assert!(html.contains("p{color:green}"), "{html}");
        assert!(html.contains("<img src=\"data:image/png;base64,"), "{html}");
    }

    #[test]
    fn the_policy_allows_what_the_document_has_become() {
        let html = save(&["-q"]);

        // One policy, the page's own, elements named by its nonce
        assert_eq!(html.matches("Content-Security-Policy").count(), 1, "{html}");
        assert!(
            html.contains(
                "<meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'nonce-abc'; \
                 img-src 'self' data:; style-src 'self'; style-src-elem 'self' 'nonce-abc';\">"
            ),
            "{html}"
        );
        assert!(
            html.contains("<script nonce=\"abc\">var kept = 1;</script>"),
            "{html}"
        );
        assert!(
            html.contains("<style nonce=\"abc\">p{color:green}"),
            "{html}"
        );
    }

    #[test]
    fn isolation_goes_into_the_policy() {
        let html = save(&["-q", "-I"]);

        assert_eq!(html.matches("Content-Security-Policy").count(), 1, "{html}");
        // Nonces let anything be loaded, what's in the document gets named by hash instead
        assert!(
            html.contains(
                "content=\"script-src 'sha256-bV/iFonLVHzK854OCniu0KtOcetWq3c6ysd/aiCwR+4='; \
                 img-src data:; style-src 'none'; style-src-elem 'sha256-"
            ),
            "{html}"
        );
        assert!(
            html.contains("default-src 'unsafe-eval' 'unsafe-inline' data:;"),
            "{html}"
        );
    }

    #[test]
    fn removing_goes_into_the_policy() {
        let html = save(&["-q", "-j", "-s"]);

        assert_eq!(html.matches("Content-Security-Policy").count(), 1, "{html}");
        assert!(
            html.contains(
                "content=\"script-src 'none'; img-src 'self' data:; style-src 'none'; object-src 'none';\""
            ),
            "{html}"
        );
        assert!(!html.contains("var kept"), "{html}");
    }
}
