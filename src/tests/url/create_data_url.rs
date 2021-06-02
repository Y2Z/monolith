//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use crate::url;

    #[test]
    fn encode_string_with_specific_media_type() {
        let mime = "application/javascript";
        let data = "var word = 'hello';\nalert(word);\n";
        let data_url = url::create_data_url(mime, data.as_bytes(), &Url::parse("data:,").unwrap());

        assert_eq!(
            data_url.as_str(),
            "data:application/javascript;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
        );
    }

    #[test]
    fn encode_append_fragment() {
        let data = "<svg></svg>\n";
        let data_url = url::create_data_url(
            "image/svg+xml",
            data.as_bytes(),
            &Url::parse("data:,").unwrap(),
        );

        assert_eq!(
            data_url.as_str(),
            "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4K"
        );
    }
}
