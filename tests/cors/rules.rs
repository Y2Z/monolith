//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::header::{HeaderMap, HeaderValue};
    use url::Url;

    use monolith::cors::{CorsRequest, PageOrigin, RequestMode};

    fn url(url: &str) -> Url {
        Url::parse(url).unwrap()
    }

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(*name, HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    fn request(page: &str, with_credentials: bool) -> CorsRequest {
        CorsRequest {
            origin: PageOrigin::of(&url(page)).unwrap(),
            with_credentials,
        }
    }

    #[test]
    fn modes_of_elements() {
        assert_eq!(RequestMode::of_attribute(None), RequestMode::NoCors);
        for value in ["", "anonymous", "ANONYMOUS", "whatever"] {
            assert_eq!(
                RequestMode::of_attribute(Some(value)),
                RequestMode::Cors {
                    with_credentials: false
                },
                "{:?}",
                value
            );
        }
        assert_eq!(
            RequestMode::of_attribute(Some(" Use-Credentials ")),
            RequestMode::Cors {
                with_credentials: true
            }
        );
    }

    #[test]
    fn modes_of_scripts() {
        assert_eq!(RequestMode::of_script(None, None), RequestMode::NoCors);
        assert_eq!(
            RequestMode::of_script(Some("text/javascript"), None),
            RequestMode::NoCors
        );
        assert_eq!(
            RequestMode::of_script(Some("module"), None),
            RequestMode::Cors {
                with_credentials: false
            }
        );
        assert_eq!(
            RequestMode::of_script(Some("module"), Some("use-credentials")),
            RequestMode::Cors {
                with_credentials: true
            }
        );
    }

    #[test]
    fn origins() {
        assert_eq!(
            PageOrigin::of(&url("https://example.com:443/a")),
            Some(PageOrigin::Tuple("https://example.com".to_string()))
        );
        assert_eq!(
            PageOrigin::of(&url("file:///site/page.html")),
            Some(PageOrigin::File(url("file:///site/")))
        );
        assert_eq!(
            PageOrigin::of(&url("data:text/html,hi")),
            Some(PageOrigin::Opaque)
        );
        assert_eq!(PageOrigin::of(&url("about:blank")), None);
    }

    #[test]
    fn what_is_of_the_same_origin() {
        let site = PageOrigin::of(&url("https://example.com/page")).unwrap();
        assert!(site.is_same_origin(&url("https://example.com/font.woff")));
        assert!(!site.is_same_origin(&url("http://example.com/font.woff")));
        assert!(!site.is_same_origin(&url("https://example.com:8443/font.woff")));
        assert!(!site.is_same_origin(&url("https://cdn.example.com/font.woff")));

        // The way Firefox has it: the page's directory, and what's within it
        let file = PageOrigin::of(&url("file:///site/page.html")).unwrap();
        assert!(file.is_same_origin(&url("file:///site/font.woff")));
        assert!(file.is_same_origin(&url("file:///site/fonts/font.woff")));
        assert!(!file.is_same_origin(&url("file:///font.woff")));
        assert!(!file.is_same_origin(&url("file:///site-other/font.woff")));
        assert!(!file.is_same_origin(&url("https://example.com/font.woff")));
    }

    #[test]
    fn what_other_origins_allow() {
        let font = url("https://cdn.test/font.woff");
        let anonymous = request("https://site.test/", false);
        let with_credentials = request("https://site.test/", true);

        assert!(anonymous.allows(&font, &headers(&[("access-control-allow-origin", "*")])));
        assert!(anonymous.allows(
            &font,
            &headers(&[("access-control-allow-origin", "https://site.test")])
        ));
        assert!(!anonymous.allows(&font, &headers(&[])));
        assert!(!anonymous.allows(
            &font,
            &headers(&[("access-control-allow-origin", "https://other.test")])
        ));
        // Only one is allowed to be there
        assert!(!anonymous.allows(
            &font,
            &headers(&[
                ("access-control-allow-origin", "*"),
                ("access-control-allow-origin", "*")
            ])
        ));

        assert!(!with_credentials.allows(&font, &headers(&[("access-control-allow-origin", "*")])));
        assert!(!with_credentials.allows(
            &font,
            &headers(&[("access-control-allow-origin", "https://site.test")])
        ));
        assert!(with_credentials.allows(
            &font,
            &headers(&[
                ("access-control-allow-origin", "https://site.test"),
                ("access-control-allow-credentials", "true")
            ])
        ));

        // The page's own origin needs no one's permission
        assert!(anonymous.allows(&url("https://site.test/font.woff"), &headers(&[])));
    }

    #[test]
    fn pages_with_no_origin_of_their_own() {
        let font = url("https://cdn.test/font.woff");
        let file = request("file:///site/page.html", false);

        assert_eq!(file.origin_header_for(&font), Some("null"));
        assert!(file.allows(&font, &headers(&[("access-control-allow-origin", "*")])));
        assert!(file.allows(&font, &headers(&[("access-control-allow-origin", "null")])));
        assert!(!file.allows(&font, &headers(&[])));
    }

    #[test]
    fn what_requests_carry() {
        let anonymous = request("https://site.test/", false);
        let with_credentials = request("https://site.test/", true);

        assert_eq!(
            anonymous.origin_header_for(&url("https://cdn.test/a")),
            Some("https://site.test")
        );
        assert_eq!(
            anonymous.origin_header_for(&url("https://site.test/a")),
            None
        );
        assert!(!anonymous.sends_credentials_to(&url("https://cdn.test/a")));
        assert!(anonymous.sends_credentials_to(&url("https://site.test/a")));
        assert!(with_credentials.sends_credentials_to(&url("https://cdn.test/a")));
        assert_ne!(
            anonymous.cache_key_suffix(),
            with_credentials.cache_key_suffix()
        );
    }

    #[test]
    fn what_browsers_decided() {
        use monolith::cors::was_blocked;

        let pairs = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
            pairs
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect()
        };
        let cors_request = pairs(&[("Origin", "https://site.test")]);
        let other = url("https://cdn.test/font.woff");

        // What wasn't requested in CORS mode wasn't up to anyone to allow
        assert!(!was_blocked(&other, &pairs(&[]), &pairs(&[])));
        assert!(was_blocked(&other, &cors_request, &pairs(&[])));
        assert!(!was_blocked(
            &other,
            &cors_request,
            &pairs(&[("Access-Control-Allow-Origin", "*")])
        ));
        assert!(!was_blocked(
            &other,
            &cors_request,
            &pairs(&[("access-control-allow-origin", "https://site.test")])
        ));
        // Cookies mean credentials were asked for, and "*" isn't enough for those
        assert!(was_blocked(
            &other,
            &pairs(&[("Origin", "https://site.test"), ("Cookie", "a=1")]),
            &pairs(&[("Access-Control-Allow-Origin", "*")])
        ));
        // The page's own origin needs no one's permission, even if requests say where they're from
        assert!(!was_blocked(
            &url("https://site.test/module.js"),
            &cors_request,
            &pairs(&[])
        ));
    }
}
