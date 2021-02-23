//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::blocking::Client;
    use std::collections::HashMap;

    use crate::html;
    use crate::opts::Options;

    #[test]
    fn small_medium_large() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let srcset_value = "small.png 1x, medium.png 1.5x, large.png 2x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(cache, &client, "", &srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!(
                "{} 1x, {} 1.5x, {} 2x",
                empty_image!(),
                empty_image!(),
                empty_image!(),
            ),
        );
    }

    #[test]
    fn small_medium_only_medium_has_scale() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let srcset_value = "small.png, medium.png 1.5x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(cache, &client, "", &srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!("{}, {} 1.5x", empty_image!(), empty_image!()),
        );
    }

    #[test]
    fn commas_within_file_names() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let srcset_value = "small,s.png 1x, large,l.png 2x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(cache, &client, "", &srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!("{} 1x, {} 2x", empty_image!(), empty_image!()),
        );
    }

    #[test]
    fn tabs_and_newlines_after_commas() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let srcset_value = "small,s.png 1x,\nmedium,m.png 2x,\nlarge,l.png 3x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(cache, &client, "", &srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!(
                "{} 1x, {} 2x, {} 3x",
                empty_image!(),
                empty_image!(),
                empty_image!()
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
    use std::collections::HashMap;

    use crate::html;
    use crate::opts::Options;

    #[test]
    fn trailing_comma() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let srcset_value = "small.png 1x, large.png 2x,";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(cache, &client, "", &srcset_value, &options, 0);

        assert_eq!(
            embedded_css,
            format!("{} 1x, {} 2x,", empty_image!(), empty_image!()),
        );
    }
}
