#[cfg(test)]
mod passing {
    use monolith::cache::{Cache, Exchange};

    fn exchange(url: &str, body: &[u8]) -> Exchange {
        Exchange {
            body: body.to_vec(),
            url: url.to_string(),
            status: Some(200),
            ..Default::default()
        }
    }

    #[test]
    fn kept_in_memory() {
        let mut cache = Cache::new(0, None);
        cache.add_exchange(exchange("https://a.test/", b"page"));

        assert_eq!(cache.exchange_count(), 1);
        assert_eq!(cache.exchange(0).unwrap().body, b"page");
        assert!(cache.has_exchange("https://a.test/"));
        assert!(!cache.has_exchange("https://b.test/"));
    }

    #[test]
    fn large_bodies_go_to_disk() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut cache = Cache::new(4, Some(file.path().display().to_string()));
        cache.add_exchange(exchange("https://a.test/small", b"abc"));
        cache.add_exchange(exchange("https://a.test/large", &[7; 10_000]));

        assert_eq!(cache.exchange(0).unwrap().body, b"abc");
        assert_eq!(cache.exchange(1).unwrap().body, vec![7; 10_000]);
        assert!(cache.exchange(2).is_none());
    }
}
