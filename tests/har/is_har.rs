#[cfg(test)]
mod passing {
    use monolith::har::is_har;

    #[test]
    fn log_with_entries() {
        assert!(is_har(br#" {"log": {"version": "1.2", "entries": []}}"#));
    }

    #[test]
    fn byte_order_mark() {
        assert!(is_har(b"\xef\xbb\xbf{\"log\": {\"entries\": []}}"));
    }
}

#[cfg(test)]
mod failing {
    use monolith::har::is_har;

    #[test]
    fn other_json() {
        assert!(!is_har(br#"{"log": "something"}"#));
    }

    #[test]
    fn html() {
        assert!(!is_har(b"<html></html>"));
    }

    #[test]
    fn broken_json() {
        assert!(!is_har(br#"{"log": {"entries": ["#));
    }
}
