//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::html::{SrcSetItem, parse_srcset};

    fn pairs(srcset: &str) -> Vec<(&str, &str)> {
        parse_srcset(srcset)
            .iter()
            .map(|i| (i.path, i.descriptor))
            .collect()
    }

    #[test]
    fn uppercase_descriptors() {
        assert_eq!(
            pairs("a.png 2X, b.png 3X"),
            vec![("a.png", "2X"), ("b.png", "3X")]
        );
    }

    #[test]
    fn multiple_descriptors_stay_with_their_candidate() {
        assert_eq!(
            pairs("a.png 100w 50h, b.png 2x"),
            vec![("a.png", "100w 50h"), ("b.png", "2x")]
        );
        assert_eq!(
            pairs("a.png 50h 100w,b.png 2x"),
            vec![("a.png", "50h 100w"), ("b.png", "2x")]
        );
    }

    #[test]
    fn commas_inside_urls_and_parentheses() {
        assert_eq!(
            pairs("data:image/png;base64,iVBOR 1x, b.png 2x"),
            vec![("data:image/png;base64,iVBOR", "1x"), ("b.png", "2x")]
        );
        assert_eq!(pairs("a.png,b.png"), vec![("a.png,b.png", "")]);
        assert_eq!(
            pairs("a.png 1x (foo, bar), b.png 2x"),
            vec![("a.png", "1x (foo, bar)"), ("b.png", "2x")]
        );
    }

    #[test]
    fn stray_commas_and_whitespace() {
        assert_eq!(
            pairs("  ,, a.png  ,,, b.png 2x ,,"),
            vec![("a.png", ""), ("b.png", "2x")]
        );
        assert_eq!(
            pairs("a.png, b.png 2x"),
            vec![("a.png", ""), ("b.png", "2x")]
        );
        assert_eq!(pairs("a.png 1x,"), vec![("a.png", "1x")]);
    }

    #[test]
    fn empty_and_separator_only() {
        assert!(pairs("").is_empty());
        assert!(pairs("   ,  ,").is_empty());
    }

    #[test]
    fn non_ascii_paths() {
        assert_eq!(
            pairs("ünï.png 1x,b.png 2x"),
            vec![("ünï.png", "1x"), ("b.png", "2x")]
        );
    }

    #[test]
    fn candidates_without_space_after_comma() {
        for descriptor in ["1x", "100w"] {
            let input = format!("first.png {},second.png 2x", descriptor);
            let items = parse_srcset(&input);
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].path, "first.png");
            assert_eq!(items[0].descriptor, descriptor);
            assert_eq!(items[1].path, "second.png");
            assert_eq!(items[1].descriptor, "2x");
        }
    }

    #[test]
    fn three_items_with_width_descriptors_and_newlines() {
        let srcset = r#"https://some-site.com/width/600/https://media2.some-site.com/2021/07/some-image-073362.jpg 600w,
                        https://some-site.com/width/960/https://media2.some-site.com/2021/07/some-image-073362.jpg 960w,
                        https://some-site.com/width/1200/https://media2.some-site.com/2021/07/some-image-073362.jpg 1200w"#;
        let srcset_items: Vec<SrcSetItem> = parse_srcset(srcset);

        assert_eq!(srcset_items.len(), 3);
        assert_eq!(
            srcset_items[0].path,
            "https://some-site.com/width/600/https://media2.some-site.com/2021/07/some-image-073362.jpg"
        );
        assert_eq!(srcset_items[0].descriptor, "600w");
        assert_eq!(
            srcset_items[1].path,
            "https://some-site.com/width/960/https://media2.some-site.com/2021/07/some-image-073362.jpg"
        );
        assert_eq!(srcset_items[1].descriptor, "960w");
        assert_eq!(
            srcset_items[2].path,
            "https://some-site.com/width/1200/https://media2.some-site.com/2021/07/some-image-073362.jpg"
        );
        assert_eq!(srcset_items[2].descriptor, "1200w");
    }
}
