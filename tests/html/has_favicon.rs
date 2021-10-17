//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::html;

    #[test]
    fn icon() {
        let html = "<link rel=\"icon\" href=\"\" /><div>text</div>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let res: bool = html::has_favicon(&dom.document);

        assert!(res);
    }

    #[test]
    fn shortcut_icon() {
        let html = "<link rel=\"shortcut icon\" href=\"\" /><div>text</div>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let res: bool = html::has_favicon(&dom.document);

        assert!(res);
    }
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use monolith::html;

    #[test]
    fn absent() {
        let html = "<div>text</div>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
        let res: bool = html::has_favicon(&dom.document);

        assert!(!res);
    }
}
