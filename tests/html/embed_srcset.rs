//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::blocking::Client;
    use reqwest::Url;

    use monolith::cache::Cache;
    use monolith::core::Options;
    use monolith::html;
    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn small_medium_large() {
        let cache = &mut Some(Cache::new(0, None));
        let client = Client::new();
        let srcset_value = "small.png 1x, medium.png 1.5x, large.png 2x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(
            cache,
            &client,
            &Url::parse("data:,").unwrap(),
            srcset_value,
            &options,
        );

        assert_eq!(
            embedded_css,
            format!(
                "{dataurl} 1x, {dataurl} 1.5x, {dataurl} 2x",
                dataurl = EMPTY_IMAGE_DATA_URL,
            ),
        );
    }

    #[test]
    fn small_medium_only_medium_has_scale() {
        let cache = &mut Some(Cache::new(0, None));
        let client = Client::new();
        let srcset_value = "small.png, medium.png 1.5x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(
            cache,
            &client,
            &Url::parse("data:,").unwrap(),
            srcset_value,
            &options,
        );

        assert_eq!(
            embedded_css,
            format!("{dataurl}, {dataurl} 1.5x", dataurl = EMPTY_IMAGE_DATA_URL),
        );
    }

    #[test]
    fn commas_within_file_names() {
        let cache = &mut Some(Cache::new(0, None));
        let client = Client::new();
        let srcset_value = "small,s.png 1x, large,l.png 2x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(
            cache,
            &client,
            &Url::parse("data:,").unwrap(),
            srcset_value,
            &options,
        );

        assert_eq!(
            embedded_css,
            format!("{dataurl} 1x, {dataurl} 2x", dataurl = EMPTY_IMAGE_DATA_URL),
        );
    }

    #[test]
    fn narrow_whitespaces_within_file_names() {
        let cache = &mut Some(Cache::new(0, None));
        let client = Client::new();
        let srcset_value = "small\u{202f}s.png 1x, large\u{202f}l.png 2x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(
            cache,
            &client,
            &Url::parse("data:,").unwrap(),
            srcset_value,
            &options,
        );

        assert_eq!(
            embedded_css,
            format!("{dataurl} 1x, {dataurl} 2x", dataurl = EMPTY_IMAGE_DATA_URL),
        );
    }

    #[test]
    fn tabs_and_newlines_after_commas() {
        let cache = &mut Some(Cache::new(0, None));
        let client = Client::new();
        let srcset_value = "small-s.png 1x,\tmedium,m.png 2x,\nlarge-l.png 3x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(
            cache,
            &client,
            &Url::parse("data:,").unwrap(),
            srcset_value,
            &options,
        );

        assert_eq!(
            embedded_css,
            format!(
                "{dataurl} 1x, {dataurl} 2x, {dataurl} 3x",
                dataurl = EMPTY_IMAGE_DATA_URL
            ),
        );
    }

    #[test]
    fn no_whitespace_after_commas() {
        let cache = &mut Some(Cache::new(0, None));
        let client = Client::new();
        let srcset_value = "small-s.png 1x,medium-m.png 2x,large-l.png 3x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(
            cache,
            &client,
            &Url::parse("data:,").unwrap(),
            srcset_value,
            &options,
        );

        assert_eq!(
            embedded_css,
            format!(
                "{dataurl} 1x, {dataurl} 2x, {dataurl} 3x",
                dataurl = EMPTY_IMAGE_DATA_URL
            ),
        );
    }

    #[test]
    fn last_without_descriptor() {
        let cache = &mut Some(Cache::new(0, None));
        let client = Client::new();
        let srcset_value = "small-s.png 400w, medium-m.png 800w, large-l.png";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(
            cache,
            &client,
            &Url::parse("data:,").unwrap(),
            srcset_value,
            &options,
        );

        assert_eq!(
            embedded_css,
            format!(
                "{dataurl} 400w, {dataurl} 800w, {dataurl}",
                dataurl = EMPTY_IMAGE_DATA_URL
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
    use reqwest::blocking::Client;
    use reqwest::Url;

    use monolith::cache::Cache;
    use monolith::core::Options;
    use monolith::html;
    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn trailing_comma() {
        let cache = &mut Some(Cache::new(0, None));
        let client = Client::new();
        let srcset_value = "small.png 1x, large.png 2x,";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(
            cache,
            &client,
            &Url::parse("data:,").unwrap(),
            srcset_value,
            &options,
        );

        assert_eq!(
            embedded_css,
            format!("{dataurl} 1x, {dataurl} 2x", dataurl = EMPTY_IMAGE_DATA_URL),
        );
    }
}
