//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use url::Url;

    use monolith::core::{MonolithOptions, probe_data, probe_target, untitled_name};
    use monolith::session::Session;

    fn session() -> Session {
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        Session::new(None, None, options)
    }

    #[test]
    fn sessions_can_go_to_other_threads() {
        fn is_send<T: Send>() {}
        is_send::<Session>();
    }

    #[test]
    fn title_of_data_url() {
        let probed = probe_target(
            session(),
            "data:text/html,<title>Hello%20there</title><p>Hi</p>",
        )
        .unwrap();

        assert_eq!(probed.title.as_deref(), Some("Hello there"));
        assert_eq!(probed.url.scheme(), "data");
    }

    #[test]
    fn non_documents_have_no_title() {
        let probed = probe_target(session(), "data:text/plain,<title>Not one</title>").unwrap();

        assert_eq!(probed.title, None);
    }

    #[test]
    fn titles_get_decoded() {
        // "テスト" in Shift_JIS
        let data = b"<meta charset=\"Shift_JIS\"><title>\x83\x65\x83\x58\x83\x67</title>";

        assert_eq!(probe_data(data, None, None).as_deref(), Some("テスト"));
    }

    #[test]
    fn blank_titles_are_no_titles() {
        let data = b"<title>  </title>";

        assert_eq!(probe_data(data, None, None), None);
    }

    #[test]
    fn untitled_names() {
        let name = |url: &str| untitled_name(&Url::parse(url).unwrap());

        assert_eq!(name("https://example.com/"), "example.com");
        assert_eq!(name("https://example.com/docs/intro.html"), "intro");
        assert_eq!(name("https://example.com/docs/"), "docs");
        assert_eq!(name("https://example.com/caf%C3%A9.html?x=1"), "café");
        assert_eq!(name("https://example.com/.hidden"), ".hidden");
        assert_eq!(name("data:text/html,hi"), "Untitled");
    }
}

#[cfg(test)]
mod failing {
    use monolith::core::{MonolithOptions, probe_target};
    use monolith::session::Session;

    #[test]
    fn empty_target() {
        let session = Session::new(None, None, MonolithOptions::default());

        assert_eq!(
            probe_target(session, "").err().unwrap().to_string(),
            "no target specified"
        );
    }
}
