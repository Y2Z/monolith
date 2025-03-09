//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use html5ever::serialize::{SerializeOpts, serialize};
    use markup5ever_rcdom::SerializableHandle;

    use monolith::html;

    #[test]
    fn basic() {
        let html = "<div>text</div>";
        let mut dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        dom = html::add_favicon(&dom.document, "I_AM_A_FAVICON_DATA_URL".to_string());

        let mut buf: Vec<u8> = Vec::new();
        serialize(
            &mut buf,
            &SerializableHandle::from(dom.document.clone()),
            SerializeOpts::default(),
        )
        .unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head><link rel=\"icon\" href=\"I_AM_A_FAVICON_DATA_URL\"></link></head><body><div>text</div></body></html>"
        );
    }
}
