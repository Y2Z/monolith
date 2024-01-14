//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use markup5ever_rcdom::{Handle, NodeData};

    use monolith::html;

    #[test]
    fn html_lang_and_body_style() {
        let html = "<!doctype html><html lang=\"en\"><head></head><body></body></html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
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
                        assert_eq!(html::get_node_attr(node, "lang"), Some("en".to_string()));

                        html::set_node_attr(node, "lang", Some("de".to_string()));
                        assert_eq!(html::get_node_attr(node, "lang"), Some("de".to_string()));

                        html::set_node_attr(node, "lang", None);
                        assert_eq!(html::get_node_attr(node, "lang"), None);

                        html::set_node_attr(node, "lang", Some("".to_string()));
                        assert_eq!(html::get_node_attr(node, "lang"), Some("".to_string()));
                    } else if node_name == "body" {
                        assert_eq!(html::get_node_attr(node, "style"), None);

                        html::set_node_attr(node, "style", Some("display: none;".to_string()));
                        assert_eq!(
                            html::get_node_attr(node, "style"),
                            Some("display: none;".to_string())
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

    #[test]
    fn body_background() {
        let html = "<!doctype html><html lang=\"en\"><head></head><body background=\"1\" background=\"2\"></body></html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());
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

                    if node_name == "body" {
                        assert_eq!(
                            html::get_node_attr(node, "background"),
                            Some("1".to_string())
                        );

                        html::set_node_attr(node, "background", None);
                        assert_eq!(html::get_node_attr(node, "background"), None);
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
