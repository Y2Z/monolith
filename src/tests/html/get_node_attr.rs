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
    fn div_two_style_attributes() {
        let html = "<!doctype html><html><head></head><body><DIV STYLE=\"color: blue;\" style=\"display: none;\"></div></body></html>";
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

                    if node_name == "body" {
                        assert_eq!(html::get_node_attr(node, "class"), None);
                    } else if node_name == "div" {
                        assert_eq!(
                            html::get_node_attr(node, "style"),
                            Some(str!("color: blue;"))
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

        assert_eq!(count, 6);
    }
}
