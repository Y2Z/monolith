//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use url::Url;

    use monolith::csp::{Destination, Inline, Policies, Policy};

    fn url(url: &str) -> Url {
        Url::parse(url).unwrap()
    }

    fn policies(page: &str, header: &str) -> Policies {
        Policies::of(&url(page), Some(header))
    }

    fn allows(policies: &Policies, destination: Destination, target: &str) -> bool {
        policies.allows(destination, &url(target), None, None, false)
    }

    #[test]
    fn parsing() {
        assert_eq!(
            Policy::parse(" Script-Src 'self'  https://a.b ;; script-src 'none'; img-src")
                .serialize(),
            "script-src 'self' https://a.b; img-src;"
        );
        assert_eq!(
            Policy::parse_header("img-src 'none', script-src 'self'").len(),
            2
        );
        // Some directives only work in headers
        assert_eq!(
            Policy::parse_meta("frame-ancestors 'none'; sandbox; img-src 'none'")
                .unwrap()
                .serialize(),
            "img-src 'none';"
        );
        assert!(Policy::parse_meta("report-uri /x").is_none());
    }

    #[test]
    fn directives_fall_back() {
        let policies = policies("https://a.b/", "default-src 'self'; img-src *");
        assert!(allows(&policies, Destination::Image, "https://c.d/i.png"));
        assert!(!allows(&policies, Destination::Font, "https://c.d/f.woff"));
        assert!(allows(&policies, Destination::Font, "https://a.b/f.woff"));
        // What nothing governs is allowed
        let policies = self::policies("https://a.b/", "img-src 'none'");
        assert!(allows(&policies, Destination::Script, "https://c.d/s.js"));
        assert!(!allows(&policies, Destination::Image, "https://a.b/i.png"));
        // Frames fall back to child-src, workers to child-src and then script-src
        let policies = self::policies("https://a.b/", "child-src 'none'; script-src *");
        assert!(!allows(&policies, Destination::Frame, "https://a.b/f.html"));
        assert!(!allows(&policies, Destination::Worker, "https://c.d/w.js"));
        let policies = self::policies("https://a.b/", "frame-src *; script-src 'none'");
        assert!(allows(&policies, Destination::Frame, "https://a.b/f.html"));
        assert!(!allows(&policies, Destination::Worker, "https://c.d/w.js"));
    }

    #[test]
    fn self_and_schemes() {
        let policies = policies(
            "http://a.b/",
            "img-src 'self'; font-src https:; media-src data:",
        );
        assert!(allows(&policies, Destination::Image, "http://a.b/i.png"));
        // Secure versions of the page's own origin are its own too
        assert!(allows(&policies, Destination::Image, "https://a.b/i.png"));
        assert!(!allows(
            &policies,
            Destination::Image,
            "http://a.b:8080/i.png"
        ));
        assert!(!allows(&policies, Destination::Image, "http://c.a.b/i.png"));
        assert!(allows(&policies, Destination::Font, "https://c.d/f.woff"));
        assert!(!allows(&policies, Destination::Font, "http://c.d/f.woff"));
        assert!(allows(&policies, Destination::Media, "data:audio/wav,"));
        // Stars aren't for data URLs
        let policies = self::policies("https://a.b/", "img-src *");
        assert!(!allows(&policies, Destination::Image, "data:image/png,"));
        assert!(allows(&policies, Destination::Image, "http://c.d/i.png"));
    }

    #[test]
    fn hosts_ports_and_paths() {
        let policies = policies(
            "https://a.b/",
            "img-src *.c.d e.f:8080 https://g.h/images/ https://i.j/one.png",
        );
        assert!(allows(&policies, Destination::Image, "https://x.c.d/i.png"));
        assert!(!allows(&policies, Destination::Image, "https://c.d/i.png"));
        assert!(allows(
            &policies,
            Destination::Image,
            "https://e.f:8080/i.png"
        ));
        assert!(!allows(&policies, Destination::Image, "https://e.f/i.png"));
        // Sources without schemes get the page's
        assert!(!allows(
            &policies,
            Destination::Image,
            "http://e.f:8080/i.png"
        ));
        assert!(allows(
            &policies,
            Destination::Image,
            "https://g.h/images/a/b.png"
        ));
        assert!(!allows(
            &policies,
            Destination::Image,
            "https://g.h/other/b.png"
        ));
        assert!(allows(&policies, Destination::Image, "https://i.j/one.png"));
        assert!(!allows(
            &policies,
            Destination::Image,
            "https://i.j/two.png"
        ));
        // Where requests got redirected to only has to be of the right host
        assert!(policies.allows(
            Destination::Image,
            &url("https://g.h/other/b.png"),
            None,
            None,
            true
        ));
    }

    #[test]
    fn every_policy_has_to_allow() {
        let policies = policies("https://a.b/", "img-src *, img-src https://c.d");
        assert!(allows(&policies, Destination::Image, "https://c.d/i.png"));
        assert!(!allows(&policies, Destination::Image, "https://e.f/i.png"));
        let mut policies = self::policies("https://a.b/", "img-src *");
        policies.add_meta("img-src 'self'");
        assert!(!allows(&policies, Destination::Image, "https://c.d/i.png"));
    }

    #[test]
    fn nonces_integrity_and_strict_dynamic() {
        let policies = policies(
            "https://a.b/",
            "script-src 'self' 'nonce-abc' 'sha256-ZGVm'; style-src 'nonce-abc'",
        );
        let script = url("https://c.d/s.js");
        assert!(!policies.allows(Destination::Script, &script, None, None, false));
        assert!(policies.allows(Destination::Script, &script, Some("abc"), None, false));
        assert!(!policies.allows(Destination::Script, &script, Some("abd"), None, false));
        assert!(policies.allows(
            Destination::Script,
            &script,
            None,
            Some("sha256-ZGVm"),
            false
        ));
        assert!(policies.allows(Destination::Style, &script, Some("abc"), None, false));
        // Nonces are for scripts and styles only
        let policies = self::policies("https://a.b/", "default-src 'nonce-abc'");
        assert!(!policies.allows(Destination::Image, &script, Some("abc"), None, false));

        let policies = self::policies(
            "https://a.b/",
            "script-src 'self' 'strict-dynamic' 'nonce-abc'",
        );
        assert!(!allows(&policies, Destination::Script, "https://a.b/s.js"));
        assert!(policies.allows(
            Destination::Script,
            &url("https://a.b/s.js"),
            Some("abc"),
            None,
            false
        ));
    }

    #[test]
    fn inline_code() {
        // sha256 of "alert(1)"
        let hash = "'sha256-bhHHL3z2vDgxUt0W3dWQOrprscmda2Y5pLsLg4GF+pI='";
        let policies = policies(
            "https://a.b/",
            &format!("script-src 'unsafe-inline'; style-src 'nonce-abc' {}", hash),
        );
        assert!(policies.allows_inline(Inline::Script, None, "anything"));
        assert!(policies.allows_inline(Inline::ScriptAttribute, None, "anything"));
        assert!(policies.allows_inline(Inline::Style, Some("abc"), "p {}"));
        assert!(!policies.allows_inline(Inline::Style, None, "p {}"));
        assert!(policies.allows_inline(Inline::Style, None, "alert(1)"));
        // Attributes only get to go by hash with 'unsafe-hashes'
        assert!(!policies.allows_inline(Inline::StyleAttribute, Some("abc"), "alert(1)"));
        let policies = self::policies(
            "https://a.b/",
            &format!("style-src 'unsafe-hashes' {}", hash),
        );
        assert!(policies.allows_inline(Inline::StyleAttribute, None, "alert(1)"));
        assert!(!policies.allows_inline(Inline::StyleAttribute, None, "alert(2)"));
        // Naming inline code makes 'unsafe-inline' do nothing
        let policies = self::policies("https://a.b/", "script-src 'unsafe-inline' 'nonce-abc'");
        assert!(!policies.allows_inline(Inline::Script, None, "alert(1)"));
        assert!(!policies.allows_inline(Inline::ScriptAttribute, None, "alert(1)"));
        let policies = self::policies(
            "https://a.b/",
            "script-src 'unsafe-inline' 'strict-dynamic'",
        );
        assert!(!policies.allows_inline(Inline::Script, None, "alert(1)"));
    }

    #[test]
    fn upgrades_bases_and_sandboxes() {
        let policies = policies(
            "https://a.b/",
            "upgrade-insecure-requests; base-uri 'self'; sandbox allow-forms",
        );
        assert_eq!(
            policies
                .upgrade(&url("http://c.d/i.png"), Destination::Image)
                .as_str(),
            "https://c.d/i.png"
        );
        assert_eq!(
            policies
                .upgrade(&url("http://c.d:80/i.png"), Destination::Image)
                .as_str(),
            "https://c.d/i.png"
        );
        // Frames of other hosts stay the way they are
        assert_eq!(
            policies
                .upgrade(&url("http://c.d/f.html"), Destination::Frame)
                .as_str(),
            "http://c.d/f.html"
        );
        assert!(policies.allows_base(&url("https://a.b/x/")));
        assert!(!policies.allows_base(&url("https://c.d/")));
        assert!(policies.forbids_scripts());
        assert!(policies.has_opaque_origin());
        assert!(
            !self::policies("https://a.b/", "sandbox allow-scripts allow-same-origin")
                .forbids_scripts()
        );
        assert!(!self::policies("https://a.b/", "img-src 'none'").forbids_scripts());
    }

    #[test]
    fn files() {
        let policies = policies("file:///a/b.html", "img-src 'self'");
        assert!(allows(&policies, Destination::Image, "file:///c/d.png"));
        assert!(!allows(&policies, Destination::Image, "https://c.d/i.png"));
    }
}
