//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::html::{parse_srcset, SrcSetItem};

    #[test]
    fn three_items_with_width_descriptors_and_newlines() {
        let srcset = r#"https://some-site.com/width/600/https://media2.some-site.com/2021/07/some-image-073362.jpg 600w,
                        https://some-site.com/width/960/https://media2.some-site.com/2021/07/some-image-073362.jpg 960w,
                        https://some-site.com/width/1200/https://media2.some-site.com/2021/07/some-image-073362.jpg 1200w"#;
        let srcset_items: Vec<SrcSetItem> = parse_srcset(srcset);

        assert_eq!(srcset_items.len(), 3);
        assert_eq!(srcset_items[0].path, "https://some-site.com/width/600/https://media2.some-site.com/2021/07/some-image-073362.jpg");
        assert_eq!(srcset_items[0].descriptor, "600w");
        assert_eq!(srcset_items[1].path, "https://some-site.com/width/960/https://media2.some-site.com/2021/07/some-image-073362.jpg");
        assert_eq!(srcset_items[1].descriptor, "960w");
        assert_eq!(srcset_items[2].path, "https://some-site.com/width/1200/https://media2.some-site.com/2021/07/some-image-073362.jpg");
        assert_eq!(srcset_items[2].descriptor, "1200w");
    }
}
