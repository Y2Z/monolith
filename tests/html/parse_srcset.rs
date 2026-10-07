//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::html::{SrcSetItem, parse_srcset};

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
