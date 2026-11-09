#[cfg(test)]
mod passing {
    use monolith::net::request_headers;
    use monolith::url::Url;

    #[test]
    fn secure_pages_refer_to_themselves() {
        let page = Url::parse("https://a.test/page?token=secret").unwrap();
        let asset = Url::parse("https://b.test/a.css").unwrap();

        let headers = request_headers(None, &page, &asset);

        assert_eq!(headers["referer"], "https://a.test/page?token=secret");
    }
}

#[cfg(test)]
mod failing {
    use monolith::net::request_headers;
    use monolith::url::Url;

    #[test]
    fn secure_pages_are_not_given_away_over_connections_that_are_not() {
        let page = Url::parse("https://a.test/page?token=secret").unwrap();
        let asset = Url::parse("http://a.test/a.css").unwrap();

        assert!(
            request_headers(None, &page, &asset)
                .get("referer")
                .is_none()
        );
    }
}
