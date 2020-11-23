//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use html5ever::rcdom::{Handle, NodeData};

    use crate::html;

    #[test]
    fn html_lang_and_body_style() {
        let html = "<!doctype html><html lang=\"en\"><head></head><body></body></html>";
        let dom = html::html_to_dom(&html);
        let mut count = 0;

        fn test_walk(node: &Handle, i: &mut i8) {
            *i += 1;

            match &node.data {
                NodeData::Document => {
                    // Dig deeper
                    for child in node.children.borrow().iter() {
                        test_walk(child, &mut *i);
                    }
                }
                NodeData::Element { ref name, .. } => {
                    let node_name = name.local.as_ref().to_string();

                    if node_name == "html" {
                        assert_eq!(html::get_node_attr(node, "lang"), Some(str!("en")));

                        html::set_node_attr(node, "lang", Some(str!("de")));
                        assert_eq!(html::get_node_attr(node, "lang"), Some(str!("de")));

                        html::set_node_attr(node, "lang", None);
                        assert_eq!(html::get_node_attr(node, "lang"), None);

                        html::set_node_attr(node, "lang", Some(str!("")));
                        assert_eq!(html::get_node_attr(node, "lang"), Some(str!("")));
                    } else if node_name == "body" {
                        assert_eq!(html::get_node_attr(node, "style"), None);

                        html::set_node_attr(node, "style", Some(str!("display: none;")));
                        assert_eq!(
                            html::get_node_attr(node, "style"),
                            Some(str!("display: none;"))
                        );
                    }

                    for child in node.children.borrow().iter() {
                        test_walk(child, &mut *i);
                    }
                }
                _ => (),
            };
        }

        test_walk(&dom.document, &mut count);

        assert_eq!(count, 5);
    }
}
