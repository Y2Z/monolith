//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use url::Url;

    use monolith::csp::{Destination, Inline, Policies, Restrictions, Saved};

    fn rewritten(page: &str, header: &str, saved: &Saved) -> Vec<String> {
        Policies::of(&Url::parse(page).unwrap(), Some(header))
            .rewritten(saved)
            .iter()
            .map(|policy| policy.serialize())
            .collect()
    }

    fn saved() -> Saved {
        Saved {
            nonce: "N".to_string(),
            has_nonce: true,
            ..Default::default()
        }
    }

    #[test]
    fn spelled_out_for_where_it_is_now() {
        assert_eq!(
            rewritten(
                "https://a.b:8443/c",
                "img-src 'self' c.d https://e.f; frame-ancestors 'none'; report-uri /r",
                &saved()
            ),
            ["img-src https://a.b:8443 https://c.d https://e.f;"]
        );
        // Files are where they are
        assert_eq!(
            rewritten("file:///a/b.html", "img-src 'self'", &saved()),
            ["img-src 'self';"]
        );
    }

    #[test]
    fn elements_named_by_nonce() {
        let mut saved = saved();
        saved.named.insert(Destination::Script);
        assert_eq!(
            rewritten(
                "https://a.b/",
                "script-src 'self'; style-src 'self'",
                &saved
            ),
            [
                "script-src https://a.b; style-src https://a.b; script-src-elem https://a.b 'nonce-N';"
            ]
        );
        // What allows every bit of inline code stays that way
        assert_eq!(
            rewritten("https://a.b/", "script-src 'unsafe-inline'", &saved),
            ["script-src 'unsafe-inline';"]
        );
        // Nonces of the page stay, scripts may know them
        saved.nonces.insert("abc".to_string());
        assert_eq!(
            rewritten("https://a.b/", "script-src 'nonce-abc'", &saved),
            ["script-src 'nonce-abc' 'nonce-N';"]
        );
        // What isn't up to policies stays that way
        assert_eq!(
            rewritten("https://a.b/", "img-src 'none'", &saved),
            ["img-src 'none';"]
        );
    }

    #[test]
    fn data_urls_and_attributes() {
        let mut saved = saved();
        saved.data.insert(Destination::Image);
        saved.data.insert(Destination::Frame);
        saved.has_imports = true;
        saved.add_inline(Inline::StyleAttribute, "color: red");
        assert_eq!(
            rewritten(
                "https://a.b/",
                "default-src 'self'; img-src * ; style-src 'self'",
                &saved
            ),
            [
                "default-src https://a.b; img-src * data:; style-src https://a.b; \
              style-src-elem https://a.b data:; \
              style-src-attr https://a.b 'unsafe-hashes' 'sha256-NerDAUWfwD31YdZHveMrq0GLjsNFMwxLpZl0dPUeCcw='; \
              frame-src https://a.b data:;"
            ]
        );
    }

    #[test]
    fn restrictions_come_on_top() {
        let mut saved = saved();
        saved.named.insert(Destination::Script);
        saved.add_inline(Inline::Script, "alert(1)");
        let mut policies = Policies::of(
            &Url::parse("https://a.b/").unwrap(),
            Some("script-src 'self' 'nonce-abc' 'strict-dynamic'; img-src *; connect-src 'self'"),
        )
        .rewritten(&saved);
        let restrictions = Restrictions {
            isolate: true,
            no_styles: true,
            ..Default::default()
        };
        restrictions.apply(&mut policies[0], &saved);

        // Nothing gets to reach out, not even what's named by nonce: it's named by hash instead
        assert_eq!(
            policies[0].serialize(),
            "script-src 'sha256-bhHHL3z2vDgxUt0W3dWQOrprscmda2Y5pLsLg4GF+pI='; img-src 'none'; \
             connect-src 'none'; default-src 'unsafe-eval' 'unsafe-inline' data:; style-src 'none';"
        );
        assert_eq!(
            Restrictions {
                no_scripts: true,
                no_images: true,
                ..Default::default()
            }
            .as_policy()
            .unwrap()
            .serialize(),
            "script-src 'none'; object-src 'none'; img-src data:;"
        );
        assert!(Restrictions::default().as_policy().is_none());
    }
}
