use http::{is_valid_url, resolve_url, retrieve_asset};
use std::default::Default;
use std::io;
use utils::data_to_dataurl;

use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::TendrilSink;

enum NodeMatch {
    Icon,
    Image,
    StyleSheet,
    Anchor,
    Script,
    Form,
    IFrame,
    Other,
}

const PNG_PIXEL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

const JS_DOM_EVENT_ATTRS: [&str; 21] = [
    // Input
    "onfocus",
    "onblur",
    "onselect",
    "onchange",
    "onsubmit",
    "onreset",
    "onkeydown",
    "onkeypress",
    "onkeyup",
    // Mouse
    "onmouseover",
    "onmouseout",
    "onmousedown",
    "onmouseup",
    "onmousemove",
    // Click
    "onclick",
    "ondblclick",
    // Load
    "onload",
    "onunload",
    "onabort",
    "onerror",
    "onresize",
];

pub fn walk_and_embed_assets(
    url: &str,
    node: &Handle,
    opt_no_js: bool,
    opt_no_images: bool,
    opt_user_agent: &str,
) {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child in node.children.borrow().iter() {
                walk_and_embed_assets(
                    &url, child,
                    opt_no_js,
                    opt_no_images,
                    opt_user_agent,
                );
            }
        }

        NodeData::Doctype { .. } => {}

        NodeData::Text { .. } => {}

        NodeData::Comment { .. } => {
            // Note: in case of opt_no_js being set to true, there's no need to worry about
            //       getting rid of comments that may contain scripts, e.g. <!--[if IE]><script>...
            //       since that's not part of W3C standard and gets ignored by browsers other than IE [5, 9]
        }

        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let attrs_mut = &mut attrs.borrow_mut();
            let mut found = NodeMatch::Other;

            match name.local.as_ref() {
                "link" => {
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
                }
                "img" => { found = NodeMatch::Image; }
                "a" => { found = NodeMatch::Anchor; }
                "script" => { found = NodeMatch::Script; }
                "form" => { found = NodeMatch::Form; }
                "iframe" => { found = NodeMatch::IFrame; }
                _ => {}
            }

            match found {
                NodeMatch::Icon => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "href" {
                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            let favicon_datauri = retrieve_asset(
                                &href_full_url.unwrap(),
                                true,
                                "",
                                opt_user_agent,
                            );
                            attr.value.clear();
                            attr.value.push_slice(favicon_datauri.unwrap().as_str());
                        }
                    }
                }
                NodeMatch::Image => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "src" {
                            if opt_no_images {
                                attr.value.clear();
                                attr.value.push_slice(PNG_PIXEL);
                            } else {
                                let src_full_url = resolve_url(&url, &attr.value.to_string());
                                let img_datauri = retrieve_asset(
                                    &src_full_url.unwrap(),
                                    true,
                                    "",
                                    opt_user_agent,
                                );
                                attr.value.clear();
                                attr.value.push_slice(img_datauri.unwrap().as_str());
                            }
                        }
                    }
                }
                NodeMatch::Anchor => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "href" {
                            // Don't touch email links or hrefs which begin with a hash sign
                            if attr.value.starts_with('#') || attr.value.starts_with("mailto:") {
                                continue;
                            }

                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            attr.value.clear();
                            attr.value.push_slice(href_full_url.unwrap().as_str());
                        }
                    }
                }
                NodeMatch::StyleSheet => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "href" {
                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            let css_datauri = retrieve_asset(
                                &href_full_url.unwrap(),
                                true,
                                "text/css",
                                opt_user_agent,
                            );
                            attr.value.clear();
                            attr.value.push_slice(css_datauri.unwrap().as_str());
                        }
                    }
                }
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
                                let src_full_url = resolve_url(&url, &attr.value.to_string());
                                let js_datauri = retrieve_asset(
                                    &src_full_url.unwrap(),
                                    true,
                                    "application/javascript",
                                    opt_user_agent,
                                );
                                attr.value.clear();
                                attr.value.push_slice(js_datauri.unwrap().as_str());
                            }
                        }
                    }
                }
                NodeMatch::Form => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "action" {
                            // Do not touch action props which are set to a URL
                            if is_valid_url(&attr.value) {
                                continue;
                            }

                            let href_full_url = resolve_url(&url, &attr.value.to_string());
                            attr.value.clear();
                            attr.value.push_slice(href_full_url.unwrap().as_str());
                        }
                    }
                }
                NodeMatch::IFrame => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "src" {
                            let src_full_url = resolve_url(&url, &attr.value.to_string()).unwrap();
                            let iframe_data = retrieve_asset(
                                &src_full_url,
                                false,
                                "text/html",
                                opt_user_agent,
                            );
                            let dom = html_to_dom(&iframe_data.unwrap());
                            walk_and_embed_assets(&src_full_url, &dom.document, opt_no_js, opt_no_images, opt_user_agent);
                            let mut buf: Vec<u8> = Vec::new();
                            serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();
                            let iframe_datauri = data_to_dataurl("text/html", &buf);
                            attr.value.clear();
                            attr.value.push_slice(iframe_datauri.as_str());
                        }
                    }
                }
                NodeMatch::Other => {}
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
                walk_and_embed_assets(
                    &url,
                    child,
                    opt_no_js,
                    opt_no_images,
                    opt_user_agent,
                );
            }
        }

        NodeData::ProcessingInstruction { .. } => unreachable!(),
    }
}

pub fn html_to_dom(data: &str) -> html5ever::rcdom::RcDom {
    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut data.as_bytes())
        .unwrap()
}

pub fn print_dom(handle: &Handle) {
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
