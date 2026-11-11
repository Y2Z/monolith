//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::MonolithOptions;
    use monolith::html;
    use monolith::simple::simplify_document;

    fn simplify(html: &str, add_styles: bool) -> String {
        let dom = html::html_to_dom(html.as_bytes(), "".to_string());
        simplify_document(&dom, add_styles);

        String::from_utf8_lossy(&html::serialize_document(
            dom,
            "".to_string(),
            &MonolithOptions::default(),
        ))
        .to_string()
    }

    const PARAGRAPH: &str = "This paragraph is long enough to count as a part of the article, \
        and it has got a few commas in it, which is what actual prose tends to look like, \
        unlike the things you would find in menus and footers of a website.";

    fn page() -> String {
        format!(
            "<html><head><title>Story | Site</title><script src=\"a.js\"></script>\
            <link rel=\"stylesheet\" href=\"a.css\"></head><body>\
            <div class=\"site-header\"><a href=\"/\">Home</a></div>\
            <nav><a href=\"/a\">Link</a></nav>\
            <div id=\"content\"><h1>Story</h1><p class=\"lead\" style=\"color: red\">{0}</p>\
            <p>{0}</p><img src=\"a.png\"><p>{0}</p></div>\
            <div class=\"sidebar\"><p>{0}</p></div>\
            <footer><p>{0}</p></footer>\
            </body></html>",
            PARAGRAPH
        )
    }

    #[test]
    fn keeps_the_article() {
        let result = simplify(&page(), false);

        assert_eq!(result.matches("This paragraph").count(), 3);
        assert!(result.contains("<h1>Story</h1>"));
        assert!(result.contains("<img src=\"a.png\">"));
    }

    #[test]
    fn drops_everything_around_the_article() {
        let result = simplify(&page(), false);

        assert!(!result.contains("Home"));
        assert!(!result.contains("<nav"));
        assert!(!result.contains("<footer"));
        assert!(!result.contains("sidebar"));
        assert!(!result.contains("<script"));
        assert!(!result.contains("a.css"));
    }

    #[test]
    fn drops_attributes_that_affect_looks() {
        let result = simplify(&page(), false);

        assert!(!result.contains("class="));
        assert!(!result.contains("style="));
    }

    #[test]
    fn adds_styles_only_when_asked_to() {
        assert!(simplify(&page(), true).contains("color-scheme: light dark"));
        assert!(!simplify(&page(), false).contains("<style"));
    }

    #[test]
    fn keeps_title_and_favicon() {
        let result = simplify(
            "<html><head><title>a</title><link rel=\"icon\" href=\"a.ico\"></head><body></body></html>",
            false,
        );

        assert!(result.contains("<title>a</title>"));
        assert!(result.contains("a.ico"));
    }

    #[test]
    fn adds_heading_if_article_has_none() {
        let result = simplify(
            &format!(
                "<html><head><title>Story</title></head><body><article><p>{0}</p><p>{0}</p></article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(result.contains("<h1>Story</h1><hr>"));
    }

    #[test]
    fn keeps_contents_of_deeply_nested_articles() {
        let result = simplify(
            &format!(
                "<html><head></head><body class=\"a\"><div><main><div><div><article>\
                <p>{0}</p><p>{0}</p></article></div></div></main></div></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert_eq!(result.matches("This paragraph").count(), 2);
        assert!(!result.contains("class="));
    }

    #[test]
    fn utility_classes_do_not_count() {
        let result = simplify(
            &format!(
                "<html><head></head><body><div class=\"[--sticky-top:var(--header-h)] md:nav-x\">\
                <div><p>{0}</p><p>{0}</p></div></div></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert_eq!(result.matches("This paragraph").count(), 2);
    }

    #[test]
    fn only_whole_words_count() {
        let result = simplify(
            &format!(
                "<html><head></head><body><div class=\"threads\"><p>{0}</p><p>{0}</p></div></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert_eq!(result.matches("This paragraph").count(), 2);
    }

    #[test]
    fn plurals_count() {
        let result = simplify(
            &format!(
                "<html><head></head><body><article><p>{0}</p><p>{0}</p></article>\
                <div class=\"post-comments\"><p>Comment</p></div>\
                <div class=\"comments\"><p>Comment</p></div></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(!result.contains("Comment"));
    }

    #[test]
    fn drops_attributes_of_root_element_and_comments() {
        let result = simplify(
            "<html lang=\"en\" style=\"a\"><head></head><body><!-- a --><p>b</p></body></html>",
            false,
        );

        assert!(result.contains("<html lang=\"en\">"));
        assert!(!result.contains("<!--"));
    }

    #[test]
    fn reports_pages_with_barely_any_text() {
        let dom = html::html_to_dom(
            "<html><head></head><body><div id=\"root\"></div></body></html>".as_bytes(),
            "".to_string(),
        );

        assert!(!simplify_document(&dom, false));
    }

    #[test]
    fn single_article_element_is_all_that_matters() {
        let result = simplify(
            &format!(
                "<html><head></head><body><div><span>Site</span><main><span>Site</span>\
                <article><p>{0}</p><p>{0}</p></article></main></div></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert_eq!(result.matches("This paragraph").count(), 2);
        assert!(!result.contains("Site"));
    }

    #[test]
    fn drops_citations_and_links_to_other_articles() {
        let result = simplify(
            &format!(
                "<html><head></head><body><article><p>{0}</p><p>{0}</p>\
                <section id=\"citations\"><h2>Author</h2><div>Someone</div></section>\
                <div><h2>Keep</h2><div><img src=\"a.png\"><a href=\"b\">Some other article on the same website</a></div></div>\
                </article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(!result.contains("Someone"));
        assert!(!result.contains("a.png"));
    }

    #[test]
    fn drops_lifeless_widgets() {
        let result = simplify(
            &format!(
                "<html><head></head><body><article><div><span>Loading\u{2026}</span></div>\
                <audio></audio><p>{0}</p><p>{0}</p></article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(!result.contains("Loading"));
        assert!(!result.contains("<audio"));
    }

    #[test]
    fn places_streamed_contents() {
        let result = simplify(
            &format!(
                "<html><head></head><body><article><p>{0}</p><div><template id=\"B:0\"></template></div>\
                <p>{0}</p></article><div hidden=\"\" id=\"S:0\"><p>Streamed</p></div></body></html>",
                PARAGRAPH
            ),
            false,
        );

        let streamed = result.find("Streamed").unwrap();
        assert!(result.find("This paragraph").unwrap() < streamed);
        assert!(streamed < result.rfind("This paragraph").unwrap());
        assert!(!result.contains("<template"));
    }

    #[test]
    fn drops_headings_with_nothing_but_links() {
        let result = simplify(
            &format!(
                "<html><head></head><body><article><p>{0}</p><p>{0}</p>\
                <div><div><h2>Keep reading</h2></div><a href=\"a\">View all</a></div>\
                <div><h2>Next part</h2></div></article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(!result.contains("Keep reading"));
        assert!(result.contains("Next part"));
    }

    #[test]
    fn keeps_subtitle_and_byline_from_outside_of_the_article() {
        let result = simplify(
            &format!(
                "<html><head></head><body><h1>Title</h1>\
                <h2 class=\"widget__subheadline-text\">Short summary</h2>\
                <div class=\"social-author\"><a class=\"social-author__name\">Some One</a></div>\
                <div class=\"story\"><p>{0}</p><p>{0}</p></div></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(result.contains("<h1>Title</h1><p>Short summary</p><p>By Some One</p><hr>"));
    }

    #[test]
    fn does_not_repeat_what_the_article_already_has() {
        let result = simplify(
            &format!(
                "<html><head></head><body><article><h1>Title</h1>\
                <p class=\"byline\">By Some One</p><p>{0}</p><p>{0}</p></article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert_eq!(result.matches("Some One").count(), 1);
    }

    #[test]
    fn drops_labels_of_removed_link_blocks() {
        let result = simplify(
            &format!(
                "<html><head></head><body><article><p>{0}</p><p>{0}</p>\
                <div><div>From Your Site Articles</div><div>Related Articles Around the Web</div></div>\
                </article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(!result.contains("Related Articles"));
    }

    #[test]
    fn subtitles_have_to_follow_the_title() {
        let result = simplify(
            &format!(
                "<html><head></head><body><span class=\"Subtitle__a\">Menu item</span>\
                <article><p>{0}</p><p>{0}</p></article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(!result.contains("Menu item"));
    }

    #[test]
    fn takes_name_of_the_website_out_of_the_title() {
        let result = simplify(
            &format!(
                "<html><head><title>Site - A title \u{b7} Site</title>\
                <meta property=\"og:site_name\" content=\"Site\"></head>\
                <body><article><p>{0}</p><p>{0}</p></article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(result.contains("<h1>A title</h1>"));
    }

    #[test]
    fn keeps_headings_made_up_of_links() {
        let result = simplify(
            &format!(
                "<html><head></head><body><article><p>{0}</p><p>{0}</p>\
                <div><h4>Using <a href=\"a\">FreeBSD packages</a></h4><a href=\"#b\"></a></div>\
                <pre>pkg install monolith</pre></article></body></html>",
                PARAGRAPH
            ),
            false,
        );

        assert!(result.contains("FreeBSD packages"));
    }

    #[test]
    fn measures_code_only_if_there_are_styles() {
        let page = format!(
            "<html><head></head><body><article><p>{0}</p><p>{0}</p>\
            <pre>{1}\nshort</pre><pre>short</pre></article></body></html>",
            PARAGRAPH,
            "a".repeat(100)
        );

        assert!(simplify(&page, true).contains("<pre style=\"--columns: 100\">"));
        assert_eq!(simplify(&page, true).matches("<pre style=").count(), 1);
        assert!(!simplify(&page, false).contains("--columns"));
    }

    #[test]
    fn pages_without_articles_keep_their_contents() {
        let result = simplify(
            "<html><head></head><body><nav>a</nav><p>Short page</p></body></html>",
            false,
        );

        assert!(result.contains("Short page"));
        assert!(!result.contains("<nav"));
    }

    #[test]
    fn framesets_get_left_alone() {
        let result = simplify(
            "<html><head></head><frameset><frame src=\"a.html\"></frameset></html>",
            false,
        );

        assert!(result.contains("<frame"));
    }
}
