//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use base64::{Engine, prelude::BASE64_STANDARD};

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
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let session = Session::new(None, None, options);

        let (result, _title) =
            create_monolithic_document_from_data(session, html.into_bytes(), None, None).unwrap();

        // The stylesheet becomes a part of the document, encoded the same way it is
        let (result, _, _) = encoding_rs::SHIFT_JIS.decode(&result);
        assert!(result.contains("<style>p{font-family:\"メイリオ\"}</style>"));
    }
}
