//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::url;

    #[test]
    fn decode_unicode_characters() {
        assert_eq!(
            url::percent_decode(
                "%E6%A4%9C%E3%83%92%E3%83%A0%E8%A7%A3%E5%A1%97%E3%82%83%E3%83%83%20%3D%20%E3%82%B5"
                    .to_string()
            ),
            "検ヒム解塗ゃッ = サ"
        );
    }

    #[test]
    fn decode_file_url() {
        assert_eq!(
            url::percent_decode("file:///tmp/space%20here/test%231.html".to_string()),
            "file:///tmp/space here/test#1.html"
        );
    }

    #[test]
    fn plus_sign() {
        assert_eq!(
            url::percent_decode(
                "fonts.somewhere.com/css?family=Open+Sans:300,400,400italic,600,600italic"
                    .to_string()
            ),
            "fonts.somewhere.com/css?family=Open+Sans:300,400,400italic,600,600italic"
        );
    }
}
