extern crate html5ever;

use std::default::Default;
use std::io;
use http::{is_url, retrieve_asset, resolve_url};

use self::html5ever::parse_document;
use self::html5ever::rcdom::{Handle, NodeData, RcDom};
use self::html5ever::tendril::TendrilSink;
use self::html5ever::serialize::{SerializeOpts, serialize};

enum NodeMatch {
    Icon,
    Image,
    StyleSheet,
    Anchor,
    Script,
    Form,
    Other,
}

static JS_DOM_EVENT_ATTRS: [&str; 21] = [
    // Input
    "onfocus", "onblur", "onselect", "onchange", "onsubmit", "onreset", "onkeydown", "onkeypress", "onkeyup",
    // Mouse
    "onmouseover", "onmouseout", "onmousedown", "onmouseup", "onmousemove",
    // Click
    "onclick", "ondblclick",
    // Load
    "onload", "onunload", "onabort", "onerror", "onresize",
];

pub fn walk_and_embed_assets(url: &str, node: &Handle, opt_no_js: bool) {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child in node.children.borrow().iter() {
                walk_and_embed_assets(&url, child, opt_no_js);
            }
        },

        NodeData::Doctype {
            name: _,
            public_id: _,
            system_id: _,
        } => {},

        NodeData::Text { contents: _, } => {},

        NodeData::Comment { contents: _, } => {
            // Note: in case of opt_no_js being set to true, there's no need to worry about
            //       getting rid of comments that may contain scripts, e.g. <!--[if IE]><script>...
            //       since that's not part of W3C standard and gets ignored by browsers other than IE [5, 9]
        },

        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let ref mut attrs_mut = attrs.borrow_mut();
            let mut found = NodeMatch::Other;

            if &name.local == "link" {
                for attr in attrs_mut.iter_mut() {
                    if &attr.name.local == "rel" {
                        if is_icon(&attr.value.to_string()) {
                            found = NodeMatch::Icon;
                            break;
                        } else if attr.value.to_string() == "stylesheet" {
                            found = NodeMatch::StyleSheet;
                            break;
                        }
                    }
                }
            } else if &name.local == "img" {
                found = NodeMatch::Image;
            } else if &name.local == "a" {
                found = NodeMatch::Anchor;
            } else if &name.local == "script" {
                found = NodeMatch::Script;
            } else if &name.local == "form" {
                found = NodeMatch::Form;
            }

            match found {
                NodeMatch::Icon => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "href" {
                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            let favicon_datauri = retrieve_asset(&href_full_url.unwrap(), true, "");
                            attr.value.clear();
                            attr.value.push_slice(favicon_datauri.unwrap().as_str());
                        }
                    }
                },
                NodeMatch::Image => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "src" {
                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            let favicon_datauri = retrieve_asset(&href_full_url.unwrap(), true, "");
                            attr.value.clear();
                            attr.value.push_slice(favicon_datauri.unwrap().as_str());
                        }
                    }
                },
                NodeMatch::Anchor => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "href" {
                            // Do not touch hrefs which begin with a hash sign
                            if attr.value.to_string().chars().nth(0) == Some('#') {
                                continue;
                            }

                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            attr.value.clear();
                            attr.value.push_slice(href_full_url.unwrap().as_str());
                        }
                    }
                },
                NodeMatch::StyleSheet => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "href" {
                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            let favicon_datauri = retrieve_asset(&href_full_url.unwrap(), true, "text/css");
                            attr.value.clear();
                            attr.value.push_slice(favicon_datauri.unwrap().as_str());
                        }
                    }
                },
                NodeMatch::Script => {
                    if opt_no_js {
                        // Get rid of src and inner content of SCRIPT tags
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "src" {
                                attr.value.clear();
                            }
                        }
                        node.children.borrow_mut().clear();
                    } else {
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "src" {
                                let href_full_url = resolve_url(&url, &attr.value.to_string());
                                let favicon_datauri = retrieve_asset(&href_full_url.unwrap(), true, "application/javascript");
                                attr.value.clear();
                                attr.value.push_slice(favicon_datauri.unwrap().as_str());
                            }
                        }
                    }
                },
                NodeMatch::Form => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "action" {
                            // Do not touch action props which are set to a URL
                            if is_url(&attr.value) {
                                continue;
                            }

                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            attr.value.clear();
                            attr.value.push_slice(href_full_url.unwrap().as_str());
                        }
                    }
                },
                NodeMatch::Other => {},
            }

            if opt_no_js {
                // Get rid of JS event attributes
                for attr in attrs_mut.iter_mut() {
                    if JS_DOM_EVENT_ATTRS.contains(&attr.name.local.to_lowercase().as_str()) {
                        attr.value.clear();
                    }
                }
            } 

            // Dig deeper
            for child in node.children.borrow().iter() {
                walk_and_embed_assets(&url, child, opt_no_js);
            }
        },

        NodeData::ProcessingInstruction { .. } => unreachable!(),
    }
}

pub fn html_to_dom(data: &str) -> html5ever::rcdom::RcDom {
    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut data.as_bytes())
        .unwrap()
}

pub fn print_dom(handle: &Handle, _opt_isolate: bool) {
    // TODO: append <meta http-equiv="Access-Control-Allow-Origin" content="'self'"/> to the <head> if opt_isolate
    serialize(&mut io::stdout(), handle, SerializeOpts::default()).unwrap();
}

fn is_icon(attr_value: &str) -> bool {
    attr_value == "icon"
        || attr_value == "shortcut icon"
        || attr_value == "mask-icon"
        || attr_value == "apple-touch-icon"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_icon() {
        assert_eq!(is_icon("icon"), true);
        assert_eq!(is_icon("stylesheet"), false);
    }
}
