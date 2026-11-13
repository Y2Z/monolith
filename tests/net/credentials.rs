//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use url::Url;

    use monolith::net::{Credentials, without_credentials};

    fn url(url: &str) -> Url {
        Url::parse(url).unwrap()
    }

    #[test]
    fn get_taken_out_of_urls() {
        let mut credentials = Credentials::default();

        let taken = credentials.take_from(&url("https://us%40er:p%3Ass@example.com/a#b"));

        assert_eq!(taken.as_str(), "https://example.com/a#b");
        assert_eq!(
            credentials.of(&url("https://example.com/style.css")),
            Some(&("us@er".to_string(), Some("p:ss".to_string())))
        );
    }

    #[test]
    fn are_for_their_origin_only() {
        let mut credentials = Credentials::default();
        credentials.take_from(&url("https://user:pass@example.com/"));

        assert!(credentials.of(&url("https://example.com:443/x")).is_some());
        assert!(credentials.of(&url("http://example.com/")).is_none());
        assert!(credentials.of(&url("https://example.com:8443/")).is_none());
        assert!(credentials.of(&url("https://cdn.example.com/")).is_none());
    }

    #[test]
    fn keep_cache_keys_apart_without_passwords() {
        let mut credentials = Credentials::default();
        let anonymous = Credentials::default();
        credentials.take_from(&url("https://user:pass@example.com/"));

        let key = credentials.cache_key(&url("https://example.com/style.css#x"));
        assert_eq!(key, "https://user@example.com/style.css");
        assert_eq!(
            anonymous.cache_key(&url("https://example.com/style.css")),
            "https://example.com/style.css"
        );
        assert_eq!(
            credentials.cache_key(&url("https://cdn.example.com/a.js")),
            "https://cdn.example.com/a.js"
        );
    }

    #[test]
    fn password_alone_is_someone() {
        let mut credentials = Credentials::default();
        credentials.take_from(&url("https://:pass@example.com/"));

        assert_eq!(
            credentials.cache_key(&url("https://example.com/")),
            "(password) https://example.com/"
        );
    }

    #[test]
    fn credentials_in_urls_count_for_their_cache_keys() {
        let credentials = Credentials::default();

        assert_eq!(
            credentials.cache_key(&url("https://user:pass@example.com/a#b")),
            "https://user@example.com/a"
        );
        assert_eq!(
            credentials.cache_key(&url("https://:pass@example.com/a")),
            "(password) https://example.com/a"
        );
    }

    #[test]
    fn urls_without_credentials() {
        assert_eq!(
            without_credentials(&url("https://user:pass@example.com/a")).as_str(),
            "https://example.com/a"
        );
    }
}
