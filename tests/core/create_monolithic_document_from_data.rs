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

#[cfg(test)]
mod destination {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    use monolith::core::{
        MonolithOptions, MonolithOutputFormat, create_monolithic_document_from_data,
    };
    use monolith::session::Session;

    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Session that writes into what's provided, along with the titles it got opened with.
    fn session(format: MonolithOutputFormat) -> (Session, Shared, Arc<Mutex<Vec<Option<String>>>>) {
        let options = MonolithOptions {
            output_format: format,
            silent: true,
            ..Default::default()
        };
        let mut session = Session::new(None, None, options);
        let written = Shared::default();
        let titles = Arc::new(Mutex::new(vec![]));
        let (writer, opened) = (written.clone(), Arc::clone(&titles));
        session.set_destination(Box::new(move |title| {
            opened.lock().unwrap().push(title.map(str::to_string));
            Ok(Box::new(writer) as Box<dyn Write + Send>)
        }));

        (session, written, titles)
    }

    const PAGE: &str = "<html><head><title>Some page</title></head><body><img src=\"data:image/png;base64,iVBORw0KGgo=\"></body></html>";

    #[test]
    fn html_gets_written_into_destination() {
        let (session, written, titles) = session(MonolithOutputFormat::HTML);

        let (result, title) = create_monolithic_document_from_data(
            session,
            PAGE.as_bytes().to_vec(),
            None,
            Some("https://example.com/".to_string()),
        )
        .unwrap();

        assert!(result.is_empty());
        assert_eq!(title.as_deref(), Some("Some page"));
        assert_eq!(*titles.lock().unwrap(), vec![Some("Some page".to_string())]);
        let written = String::from_utf8(written.0.lock().unwrap().clone()).unwrap();
        assert!(written.starts_with("<html><head>"));
        assert!(written.ends_with("</html>\n"));
    }

    #[test]
    fn mhtml_gets_written_into_destination() {
        let (session, written, titles) = session(MonolithOutputFormat::MHTML);

        let (result, _) = create_monolithic_document_from_data(
            session,
            PAGE.as_bytes().to_vec(),
            None,
            Some("https://example.com/".to_string()),
        )
        .unwrap();

        assert!(result.is_empty());
        assert_eq!(titles.lock().unwrap().len(), 1);
        let written = String::from_utf8(written.0.lock().unwrap().clone()).unwrap();
        assert!(written.starts_with("From: <Saved by Monolith>\r\n"));
        assert!(written.contains("Subject: Some page\r\n"));
        assert!(written.trim_end().ends_with("--"));
    }

    #[test]
    fn warc_records_get_written_into_destination() {
        for format in [MonolithOutputFormat::WARC, MonolithOutputFormat::WARCGZ] {
            let (session, written, titles) = session(format.clone());

            let (result, title) = create_monolithic_document_from_data(
                session,
                PAGE.as_bytes().to_vec(),
                None,
                Some("https://example.com/".to_string()),
            )
            .unwrap();

            assert!(result.is_empty());
            assert_eq!(title.as_deref(), Some("Some page"));
            assert_eq!(*titles.lock().unwrap(), vec![Some("Some page".to_string())]);
            let written = written.0.lock().unwrap().clone();
            let written = match format {
                MonolithOutputFormat::WARCGZ => {
                    let mut data = vec![];
                    io::Read::read_to_end(
                        &mut flate2::read::MultiGzDecoder::new(written.as_slice()),
                        &mut data,
                    )
                    .unwrap();
                    data
                }
                _ => written,
            };
            let written = String::from_utf8_lossy(&written);
            assert!(written.starts_with("WARC/1.1\r\n"));
            assert!(written.contains("WARC-Type: warcinfo\r\n"));
            assert!(written.contains("WARC-Target-URI: https://example.com/\r\n"));
            assert!(written.contains("<title>Some page</title>"));
        }
    }

    #[test]
    fn nothing_gets_opened_for_what_fails_early() {
        let (mut session, written, titles) = session(MonolithOutputFormat::HTML);
        session.options.encoding = Some("no-such-encoding".to_string());

        assert!(
            create_monolithic_document_from_data(session, PAGE.as_bytes().to_vec(), None, None)
                .is_err()
        );
        assert!(titles.lock().unwrap().is_empty());
        assert!(written.0.lock().unwrap().is_empty());
    }

    #[test]
    fn failing_to_open_is_an_error() {
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session = Session::new(None, None, options);
        session.set_destination(Box::new(|_| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "no way"))
        }));

        let error =
            create_monolithic_document_from_data(session, PAGE.as_bytes().to_vec(), None, None)
                .unwrap_err();
        assert_eq!(error.to_string(), "no way");
    }
}
