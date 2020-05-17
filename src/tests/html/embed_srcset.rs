//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::html;
    use reqwest::blocking::Client;
    use std::collections::HashMap;

    #[test]
    fn replace_with_empty_images() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let srcset_value = "small.png 1x, large.png 2x";
        let embedded_css = html::embed_srcset(cache, &client, "", &srcset_value, true, true);

        assert_eq!(
            format!("{} 1x, {} 2x", empty_image!(), empty_image!()),
            embedded_css
        );
    }
}
