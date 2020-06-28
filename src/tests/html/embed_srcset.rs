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
    fn replace_with_empty_images() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let srcset_value = "small.png 1x, large.png 2x";
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;
        let embedded_css = html::embed_srcset(cache, &client, "", &srcset_value, &options, 0);

        assert_eq!(
            format!("{} 1x, {} 2x", empty_image!(), empty_image!()),
            embedded_css
        );
    }
}
