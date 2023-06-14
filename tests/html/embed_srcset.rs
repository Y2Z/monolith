//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use monolith::html;
    use monolith::opts::Options;
    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn small_medium_large() {
        let srcset_value = "small.png 1x, medium.png 1.5x, large.png 2x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css =
            html::embed_srcset(&Url::parse("data:,").unwrap(), srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!(
                "{} 1x, {} 1.5x, {} 2x",
                EMPTY_IMAGE_DATA_URL, EMPTY_IMAGE_DATA_URL, EMPTY_IMAGE_DATA_URL,
            ),
        );
    }

    #[test]
    fn small_medium_only_medium_has_scale() {
        let srcset_value = "small.png, medium.png 1.5x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css =
            html::embed_srcset(&Url::parse("data:,").unwrap(), srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!("{}, {} 1.5x", EMPTY_IMAGE_DATA_URL, EMPTY_IMAGE_DATA_URL),
        );
    }

    #[test]
    fn commas_within_file_names() {
        let srcset_value = "small,s.png 1x, large,l.png 2x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css =
            html::embed_srcset(&Url::parse("data:,").unwrap(), srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!("{} 1x, {} 2x", EMPTY_IMAGE_DATA_URL, EMPTY_IMAGE_DATA_URL),
        );
    }

    #[test]
    fn tabs_and_newlines_after_commas() {
        let srcset_value = "small,s.png 1x,\nmedium,m.png 2x,\nlarge,l.png 3x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css =
            html::embed_srcset(&Url::parse("data:,").unwrap(), srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!(
                "{} 1x, {} 2x, {} 3x",
                EMPTY_IMAGE_DATA_URL, EMPTY_IMAGE_DATA_URL, EMPTY_IMAGE_DATA_URL
            ),
        );
    }
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use reqwest::Url;

    use monolith::html;
    use monolith::opts::Options;
    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn trailing_comma() {
        let srcset_value = "small.png 1x, large.png 2x,";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css =
            html::embed_srcset(&Url::parse("data:,").unwrap(), srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!("{} 1x, {} 2x,", EMPTY_IMAGE_DATA_URL, EMPTY_IMAGE_DATA_URL),
        );
    }
}
