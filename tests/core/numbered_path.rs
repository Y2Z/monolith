#[cfg(test)]
mod passing {
    use monolith::core::numbered_path;

    #[test]
    fn number_goes_before_the_extension() {
        assert_eq!(numbered_path("page.html", 2), "page (2).html");
        assert_eq!(numbered_path("my.page.mhtml", 3), "my.page (3).mhtml");
    }

    #[test]
    fn compressed_archives_keep_their_whole_extension() {
        assert_eq!(numbered_path("crawl.warc.gz", 2), "crawl (2).warc.gz");
        assert_eq!(numbered_path("CRAWL.WARC.GZ", 2), "CRAWL (2).WARC.GZ");
    }

    #[test]
    fn names_without_extensions() {
        assert_eq!(numbered_path("page", 2), "page (2)");
        assert_eq!(numbered_path(".page", 2), ".page (2)");
    }

    #[test]
    fn directories_stay_the_way_they_are() {
        assert_eq!(
            numbered_path("saved.d/page.html", 2),
            "saved.d/page (2).html"
        );
        assert_eq!(numbered_path("saved.d/page", 2), "saved.d/page (2)");
    }
}
