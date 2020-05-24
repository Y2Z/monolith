//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::html;
    use html5ever::rcdom::{Handle, NodeData};

    #[test]
    fn get_node_name() {
        let html = "<!doctype html><html><HEAD></HEAD><body><div><P></P></div></body></html>";
        let dom = html::html_to_dom(&html);
        let mut count = 0;

        fn test_walk(node: &Handle, i: &mut i8) {
            *i += 1;

            match &node.data {
                NodeData::Document => {
                    for child in node.children.borrow().iter() {
                        test_walk(child, &mut *i);
                    }
                }
                NodeData::Element { ref name, .. } => {
                    let node_name = name.local.as_ref().to_string();
                    let parent = html::get_parent_node(node);
                    let parent_node_name = html::get_node_name(&parent);
                    if node_name == "head" || node_name == "body" {
                        assert_eq!(parent_node_name, Some("html"));
                    } else if node_name == "div" {
                        assert_eq!(parent_node_name, Some("body"));
                    } else if node_name == "p" {
                        assert_eq!(parent_node_name, Some("div"));
                    }

                    for child in node.children.borrow().iter() {
                        test_walk(child, &mut *i);
                    }
                }
                _ => (),
            };
        }

        test_walk(&dom.document, &mut count);

        assert_eq!(count, 7);
    }
}
