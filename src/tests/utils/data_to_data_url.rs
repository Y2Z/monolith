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
    fn encode_string_with_specific_media_type() {
        let mime = "application/javascript";
        let data = "var word = 'hello';\nalert(word);\n";
        let data_url = utils::data_to_data_url(mime, data.as_bytes(), "", "");

        assert_eq!(
            &data_url,
            "data:application/javascript;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
        );
    }

    #[test]
    fn encode_append_fragment() {
        let data = "<svg></svg>\n";
        let data_url = utils::data_to_data_url("text/css", data.as_bytes(), "", "fragment");

        assert_eq!(&data_url, "data:text/css;base64,PHN2Zz48L3N2Zz4K#fragment");
    }
}
