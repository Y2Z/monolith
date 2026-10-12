//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use base64::{Engine, prelude::BASE64_STANDARD};
    use url::Url;

    use monolith::core::{MonolithOptions, create_monolithic_document_from_data};
    use monolith::session::Session;

    #[test]
    fn unlabeled_stylesheet_uses_document_encoding() {
        // "p{font-family:'メイリオ'}" in Shift_JIS, without any charset declared
        let css = b"p{font-family:'\x83\x81\x83\x43\x83\x8a\x83\x49'}";
        let html = format!(
            "<html><head><meta charset=\"Shift_JIS\"><link rel=\"stylesheet\" href=\"data:text/css;base64,{}\"></head><body></body></html>",
            BASE64_STANDARD.encode(css)
        );
        let mut options = MonolithOptions::default();
        options.silent = true;
        let session = Session::new(None, None, options);

        let (result, _title) =
            create_monolithic_document_from_data(session, html.into_bytes(), None, None).unwrap();

        // The data URL itself is ASCII, no matter what the document gets saved as
        let result = String::from_utf8_lossy(&result);
        let start = result
            .find("data:text/css")
            .expect("no stylesheet data URL");
        let end = start + result[start..].find('"').unwrap();
        let (_, charset, data) =
            monolith::url::parse_data_url(&Url::parse(&result[start..end]).unwrap());
        assert_eq!(charset, "utf-8");
        assert_eq!(
            String::from_utf8(data).unwrap(),
            "p{font-family:\"メイリオ\"}"
        );
    }
}
