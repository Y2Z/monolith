//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use html5ever::serialize::{serialize, SerializeOpts};

    use crate::html;

    #[test]
    fn basic() {
        let html = "<div>text</div>";
        let mut dom = html::html_to_dom(&html);

        dom = html::add_favicon(&dom.document, "I_AM_A_FAVICON_DATA_URL".to_string());

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head><link rel=\"icon\" href=\"I_AM_A_FAVICON_DATA_URL\"></link></head><body><div>text</div></body></html>"
        );
    }
}
