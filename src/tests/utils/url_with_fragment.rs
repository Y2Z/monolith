//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::utils;

    #[test]
    fn url_with_fragment_url() {
        let url = "https://localhost.localdomain/path/";
        let fragment = "test";
        let assembled_url = utils::url_with_fragment(url, fragment);

        assert_eq!(&assembled_url, "https://localhost.localdomain/path/#test");
    }
    #[test]
    fn url_with_fragment_empty_url() {
        let url = "https://localhost.localdomain/path/";
        let fragment = "";
        let assembled_url = utils::url_with_fragment(url, fragment);

        assert_eq!(&assembled_url, "https://localhost.localdomain/path/");
    }

    #[test]
    fn url_with_fragment_data_url() {
        let url = "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4K";
        let fragment = "fragment";
        let assembled_url = utils::url_with_fragment(url, fragment);

        assert_eq!(
            &assembled_url,
            "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4K#fragment"
        );
    }
}
