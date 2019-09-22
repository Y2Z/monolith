use html5ever::interface::QualName;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::{format_tendril, TendrilSink};
use html5ever::tree_builder::{Attribute, TreeSink};
use html5ever::{local_name, namespace_url, ns};
use http::{is_valid_url, resolve_url, retrieve_asset};
use regex::Regex;
use std::default::Default;
use utils::data_to_dataurl;

lazy_static! {
    static ref EMPTY_STRING: String = String::new();
    static ref HAS_PROTOCOL: Regex = Regex::new(r"^[a-z0-9]+:").unwrap();
    static ref ICON_VALUES: Regex =
        Regex::new(r"^icon|shortcut icon|mask-icon|apple-touch-icon|fluid-icon$").unwrap();
}

const TRANSPARENT_PIXEL: &str =
    "data:image/png;base64,\
     iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

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

fn get_parent_node(node: &Handle) -> Handle {
    let parent = node.parent.take().clone();
    parent.and_then(|node| node.upgrade()).unwrap()
}

fn get_node_name(node: &Handle) -> String {
    match &node.data {
        NodeData::Element { ref name, .. } => name.local.as_ref().to_string(),
        _ => EMPTY_STRING.clone(),
    }
}

pub fn walk_and_embed_assets(
    url: &str,
    node: &Handle,
    opt_no_css: bool,
    opt_no_js: bool,
    opt_no_images: bool,
    opt_user_agent: &str,
    opt_silent: bool,
    opt_insecure: bool,
    opt_no_frames: bool,
) {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child in node.children.borrow().iter() {
                walk_and_embed_assets(
                    &url,
                    child,
                    opt_no_css,
                    opt_no_js,
                    opt_no_images,
                    opt_user_agent,
                    opt_silent,
                    opt_insecure,
                    opt_no_frames,
                );
            }
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let attrs_mut = &mut attrs.borrow_mut();

            match name.local.as_ref() {
                "link" => {
                    let mut link_type: &str = "";

                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "rel" {
                            if is_icon(&attr.value.to_string()) {
                                link_type = "icon";
                                break;
                            } else if attr.value.to_string() == "stylesheet" {
                                link_type = "stylesheet";
                                break;
                            }
                        }
                    }

                    if link_type == "icon" {
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "href" {
                                if opt_no_images {
                                    attr.value.clear();
                                } else {
                                    let href_full_url: String =
                                        resolve_url(&url, &attr.value.to_string())
                                            .unwrap_or(EMPTY_STRING.clone());
                                    let favicon_datauri = retrieve_asset(
                                        &href_full_url,
                                        true,
                                        "",
                                        opt_user_agent,
                                        opt_silent,
                                        opt_insecure,
                                    )
                                    .unwrap_or(EMPTY_STRING.clone());
                                    attr.value.clear();
                                    attr.value.push_slice(favicon_datauri.as_str());
                                }
                            }
                        }
                    } else if link_type == "stylesheet" {
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "href" {
                                if opt_no_css {
                                    attr.value.clear();
                                } else {
                                    let href_full_url: String =
                                        resolve_url(&url, &attr.value.to_string())
                                            .unwrap_or(EMPTY_STRING.clone());
                                    let css_datauri = retrieve_asset(
                                        &href_full_url,
                                        true,
                                        "text/css",
                                        opt_user_agent,
                                        opt_silent,
                                        opt_insecure,
                                    )
                                    .unwrap_or(EMPTY_STRING.clone());
                                    attr.value.clear();
                                    attr.value.push_slice(css_datauri.as_str());
                                }
                            }
                        }
                    } else {
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "href" {
                                let href_full_url: String =
                                    resolve_url(&url, &attr.value.to_string())
                                        .unwrap_or(EMPTY_STRING.clone());
                                attr.value.clear();
                                attr.value.push_slice(&href_full_url.as_str());
                            }
                        }
                    }
                }
                "img" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "src" {
                            let value = attr.value.to_string();

                            // Ignore images with empty source
                            if value == EMPTY_STRING.clone() {
                                continue;
                            }

                            if opt_no_images {
                                attr.value.clear();
                                attr.value.push_slice(TRANSPARENT_PIXEL);
                            } else {
                                let src_full_url: String =
                                    resolve_url(&url, &value).unwrap_or(EMPTY_STRING.clone());
                                let img_datauri = retrieve_asset(
                                    &src_full_url,
                                    true,
                                    "",
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                )
                                .unwrap_or(EMPTY_STRING.clone());
                                attr.value.clear();
                                attr.value.push_slice(img_datauri.as_str());
                            }
                        }
                    }
                }
                "source" => {
                    for attr in attrs_mut.iter_mut() {
                        let attr_name: &str = &attr.name.local;

                        if attr_name == "src" {
                            let src_full_url: String = resolve_url(&url, &attr.value.to_string())
                                .unwrap_or(attr.value.to_string());
                            attr.value.clear();
                            attr.value.push_slice(src_full_url.as_str());
                        } else if attr_name == "srcset" {
                            if get_node_name(&get_parent_node(&node)) == "picture" {
                                if opt_no_images {
                                    attr.value.clear();
                                    attr.value.push_slice(TRANSPARENT_PIXEL);
                                } else {
                                    let srcset_full_url: String =
                                        resolve_url(&url, &attr.value.to_string())
                                            .unwrap_or(EMPTY_STRING.clone());
                                    let source_datauri = retrieve_asset(
                                        &srcset_full_url,
                                        true,
                                        "",
                                        opt_user_agent,
                                        opt_silent,
                                        opt_insecure,
                                    )
                                    .unwrap_or(EMPTY_STRING.clone());
                                    attr.value.clear();
                                    attr.value.push_slice(source_datauri.as_str());
                                }
                            }
                        }
                    }
                }
                "a" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "href" {
                            // Don't touch email links or hrefs which begin with a hash sign
                            if attr.value.starts_with('#') || has_protocol(&attr.value) {
                                continue;
                            }

                            let href_full_url: String = resolve_url(&url, &attr.value.to_string())
                                .unwrap_or(EMPTY_STRING.clone());
                            attr.value.clear();
                            attr.value.push_slice(href_full_url.as_str());
                        }
                    }
                }
                "script" => {
                    if opt_no_js {
                        // Empty src and inner content of SCRIPT tags
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "src" {
                                attr.value.clear();
                            }
                        }
                        node.children.borrow_mut().clear();
                    } else {
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "src" {
                                let src_full_url: String =
                                    resolve_url(&url, &attr.value.to_string())
                                        .unwrap_or(EMPTY_STRING.clone());
                                let js_datauri = retrieve_asset(
                                    &src_full_url,
                                    true,
                                    "application/javascript",
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                )
                                .unwrap_or(EMPTY_STRING.clone());
                                attr.value.clear();
                                attr.value.push_slice(js_datauri.as_str());
                            }
                        }
                    }
                }
                "style" => {
                    if opt_no_css {
                        // Empty  inner content of STYLE tags
                        node.children.borrow_mut().clear();
                    }
                }
                "form" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "action" {
                            // Don't modify action that's already a full URL
                            if is_valid_url(&attr.value) {
                                continue;
                            }

                            let href_full_url: String = resolve_url(&url, &attr.value.to_string())
                                .unwrap_or(EMPTY_STRING.clone());
                            attr.value.clear();
                            attr.value.push_slice(href_full_url.as_str());
                        }
                    }
                }
                "iframe" => {
                    if opt_no_frames {
                        // Empty the src attribute
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "src" {
                                attr.value.clear();
                            }
                        }
                    } else {
                        for attr in attrs_mut.iter_mut() {
                            if &attr.name.local == "src" {
                                let iframe_src = attr.value.to_string();

                                // Ignore iframes with empty source (they cause infinite loops)
                                if iframe_src == EMPTY_STRING.clone() {
                                    continue;
                                }

                                let src_full_url: String =
                                    resolve_url(&url, &iframe_src).unwrap_or(EMPTY_STRING.clone());
                                let iframe_data = retrieve_asset(
                                    &src_full_url,
                                    false,
                                    "text/html",
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                )
                                .unwrap_or(EMPTY_STRING.clone());
                                let dom = html_to_dom(&iframe_data);
                                walk_and_embed_assets(
                                    &src_full_url,
                                    &dom.document,
                                    opt_no_css,
                                    opt_no_js,
                                    opt_no_images,
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                    opt_no_frames,
                                );
                                let mut buf: Vec<u8> = Vec::new();
                                serialize(&mut buf, &dom.document, SerializeOpts::default())
                                    .unwrap();
                                let iframe_datauri = data_to_dataurl("text/html", &buf);
                                attr.value.clear();
                                attr.value.push_slice(iframe_datauri.as_str());
                            }
                        }
                    }
                }
                "video" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "poster" {
                            let video_poster = attr.value.to_string();

                            // Ignore posters with empty source
                            if video_poster == EMPTY_STRING.clone() {
                                continue;
                            }

                            if opt_no_images {
                                attr.value.clear();
                            } else {
                                let poster_full_url: String = resolve_url(&url, &video_poster)
                                    .unwrap_or(EMPTY_STRING.clone());
                                let img_datauri = retrieve_asset(
                                    &poster_full_url,
                                    true,
                                    "",
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                )
                                .unwrap_or(poster_full_url);
                                attr.value.clear();
                                attr.value.push_slice(img_datauri.as_str());
                            }
                        }
                    }
                }
                _ => {}
            }

            if opt_no_css {
                // Get rid of style attributes
                for attr in attrs_mut.iter_mut() {
                    if attr.name.local.to_lowercase() == "style" {
                        attr.value.clear();
                    }
                }
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
                    opt_no_css,
                    opt_no_js,
                    opt_no_images,
                    opt_user_agent,
                    opt_silent,
                    opt_insecure,
                    opt_no_frames,
                );
            }
        }
        _ => {
            // Note: in case of opt_no_js being set to true, there's no need to worry about
            //       getting rid of comments that may contain scripts, e.g. <!--[if IE]><script>...
            //       since that's not part of W3C standard and therefore gets ignored
            //       by browsers other than IE [5, 9]
        }
    }
}

fn has_protocol(url: &str) -> bool {
    HAS_PROTOCOL.is_match(&url.to_lowercase())
}

pub fn html_to_dom(data: &str) -> html5ever::rcdom::RcDom {
    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut data.as_bytes())
        .unwrap()
}

fn get_child_node_by_name(handle: &Handle, node_name: &str) -> Handle {
    let children = handle.children.borrow();
    let matching_children = children.iter().find(|child| match child.data {
        NodeData::Element { ref name, .. } => &*name.local == node_name,
        _ => false,
    });
    match matching_children {
        Some(node) => node.clone(),
        _ => {
            return handle.clone();
        }
    }
}

pub fn stringify_document(
    handle: &Handle,
    opt_no_css: bool,
    opt_no_frames: bool,
    opt_no_js: bool,
    opt_no_images: bool,
    opt_isolate: bool,
) -> String {
    let mut buf: Vec<u8> = Vec::new();
    serialize(&mut buf, handle, SerializeOpts::default())
        .expect("unable to serialize DOM into buffer");

    let mut result: String = String::from_utf8(buf).unwrap();

    if opt_isolate || opt_no_css || opt_no_frames || opt_no_js || opt_no_images {
        let mut buf: Vec<u8> = Vec::new();
        let mut dom = html_to_dom(&result);
        let doc = dom.get_document();
        let html = get_child_node_by_name(&doc, "html");
        let head = get_child_node_by_name(&html, "head");
        {
            let mut content_attr = EMPTY_STRING.clone();
            if opt_isolate {
                content_attr += "default-src 'unsafe-inline' data:;"
            }
            if opt_no_css {
                content_attr += "style-src 'none';"
            }
            if opt_no_frames {
                content_attr += "frame-src 'none';child-src 'none';"
            }
            if opt_no_js {
                content_attr += "script-src 'none';"
            }
            if opt_no_images {
                content_attr += "img-src data:;"
            }
            let meta = dom.create_element(
                QualName::new(None, ns!(), local_name!("meta")),
                vec![
                    Attribute {
                        name: QualName::new(None, ns!(), local_name!("http-equiv")),
                        value: format_tendril!("Content-Security-Policy"),
                    },
                    Attribute {
                        name: QualName::new(None, ns!(), local_name!("content")),
                        value: format_tendril!("{}", content_attr),
                    },
                ],
                Default::default(),
            );
            head.children.borrow_mut().reverse();
            head.children.borrow_mut().push(meta.clone());
            head.children.borrow_mut().reverse();
            // Note: the CSP meta-tag has to be prepended, never appended,
            //       since there already may be one defined in the document,
            //       and browsers don't allow re-defining them (for obvious reasons)
        }
        serialize(&mut buf, &doc, SerializeOpts::default())
            .expect("unable to serialize DOM into buffer");
        result = String::from_utf8(buf).unwrap();
        // Note: we can't make it isolate the page right away since it may have no HEAD element,
        //       ergo we have to serialize, parse DOM again, and then finally serialize the result
    }

    result
}

fn is_icon(attr_value: &str) -> bool {
    ICON_VALUES.is_match(&attr_value.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_icon() {
        assert_eq!(is_icon("icon"), true);
        assert_eq!(is_icon("Shortcut Icon"), true);
        assert_eq!(is_icon("ICON"), true);
        assert_eq!(is_icon("mask-icon"), true);
        assert_eq!(is_icon("fluid-icon"), true);
        assert_eq!(is_icon("stylesheet"), false);
        assert_eq!(is_icon(""), false);
    }

    #[test]
    fn test_has_protocol() {
        assert_eq!(
            has_protocol("mailto:somebody@somewhere.com?subject=hello"),
            true
        );
        assert_eq!(has_protocol("tel:5551234567"), true);
        assert_eq!(has_protocol("ftp:user:password@some-ftp-server.com"), true);
        assert_eq!(has_protocol("javascript:void(0)"), true);
        assert_eq!(has_protocol("http://news.ycombinator.com"), true);
        assert_eq!(has_protocol("https://github.com"), true);
        assert_eq!(has_protocol("//some-hostname.com/some-file.html"), false);
        assert_eq!(has_protocol("some-hostname.com/some-file.html"), false);
        assert_eq!(has_protocol("/some-file.html"), false);
        assert_eq!(has_protocol(""), false);
        assert_eq!(
            has_protocol("MAILTO:somebody@somewhere.com?subject=hello"),
            true
        );
    }

    #[test]
    fn test_get_parent_node_name() {
        let html = "<!doctype html><html><HEAD></HEAD><body><div><P></P></div></body></html>";
        let dom = html_to_dom(&html);
        let mut count = 0;

        fn test_walk(node: &Handle, i: &mut i8) {
            *i += 1;

            match &node.data {
                NodeData::Document => {
                    for child in node.children.borrow().iter() {
                        test_walk(child, &mut *i);
                    }
                }
                NodeData::Doctype { .. } => (),
                NodeData::Text { .. } => (),
                NodeData::Comment { .. } => (),
                NodeData::Element { ref name, .. } => {
                    let node_name = name.local.as_ref().to_string();
                    let parent_node_name = get_node_name(&get_parent_node(node));
                    if node_name == "head" || node_name == "body" {
                        assert_eq!(parent_node_name, "html");
                    } else if node_name == "div" {
                        assert_eq!(parent_node_name, "body");
                    } else if node_name == "p" {
                        assert_eq!(parent_node_name, "div");
                    }

                    println!("{}", node_name);

                    for child in node.children.borrow().iter() {
                        test_walk(child, &mut *i);
                    }
                }
                NodeData::ProcessingInstruction { .. } => unreachable!(),
            };
        }

        test_walk(&dom.document, &mut count);

        assert_eq!(count, 7);
    }

    #[test]
    fn test_walk_and_embed_assets() {
        let html = "<div><P></P></div>";
        let dom = html_to_dom(&html);
        let url = "http://localhost";

        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let opt_silent = true;
        let opt_insecure = false;

        walk_and_embed_assets(
            &url,
            &dom.document,
            opt_no_css,
            opt_no_js,
            opt_no_images,
            "",
            opt_silent,
            opt_insecure,
            opt_no_frames,
        );

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><div><p></p></div></body></html>"
        );
    }

    #[test]
    fn test_walk_and_embed_assets_no_recursive_iframe() {
        let html = "<div><P></P><iframe src=\"\"></iframe></div>";
        let dom = html_to_dom(&html);
        let url = "http://localhost";

        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let opt_silent = true;
        let opt_insecure = false;

        walk_and_embed_assets(
            &url,
            &dom.document,
            opt_no_css,
            opt_no_js,
            opt_no_images,
            "",
            opt_silent,
            opt_insecure,
            opt_no_frames,
        );

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><div><p></p><iframe src=\"\"></iframe></div></body></html>"
        );
    }

    #[test]
    fn test_walk_and_embed_assets_no_css() {
        let html = "<link rel=\"stylesheet\" href=\"main.css\">\
                    <style>html{background-color: #000;}</style>\
                    <div style=\"display: none;\"></div>";
        let dom = html_to_dom(&html);
        let url = "http://localhost";

        let opt_no_css: bool = true;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let opt_silent = true;
        let opt_insecure = false;

        walk_and_embed_assets(
            &url,
            &dom.document,
            opt_no_css,
            opt_no_js,
            opt_no_images,
            "",
            opt_silent,
            opt_insecure,
            opt_no_frames,
        );

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head><link rel=\"stylesheet\" href=\"\"><style></style></head>\
             <body><div style=\"\"></div></body></html>"
        );
    }

    #[test]
    fn test_walk_and_embed_assets_no_images() {
        let html = "<link rel=\"icon\" href=\"favicon.ico\">\
                    <div><img src=\"http://localhost/assets/mono_lisa.png\" /></div>";
        let dom = html_to_dom(&html);
        let url = "http://localhost";

        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = true;
        let opt_silent = true;
        let opt_insecure = false;

        walk_and_embed_assets(
            &url,
            &dom.document,
            opt_no_css,
            opt_no_js,
            opt_no_images,
            "",
            opt_silent,
            opt_insecure,
            opt_no_frames,
        );

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head><link rel=\"icon\" href=\"\"></head><body><div>\
             <img src=\"data:image/png;base64,\
             iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0\
             lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\">\
             </div></body></html>"
        );
    }

    #[test]
    fn test_walk_and_embed_assets_no_frames() {
        let html = "<iframe src=\"http://trackbook.com\"></iframe>";
        let dom = html_to_dom(&html);
        let url = "http://localhost";

        let opt_no_css: bool = false;
        let opt_no_frames: bool = true;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let opt_silent = true;
        let opt_insecure = false;

        walk_and_embed_assets(
            &url,
            &dom.document,
            opt_no_css,
            opt_no_js,
            opt_no_images,
            "",
            opt_silent,
            opt_insecure,
            opt_no_frames,
        );

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><iframe src=\"\"></iframe></body></html>"
        );
    }

    #[test]
    fn test_walk_and_embed_assets_no_js() {
        let html =
            "<div onClick=\"void(0)\"><script src=\"http://localhost/assets/some.js\"></script>\
             <script>alert(1)</script></div>";
        let dom = html_to_dom(&html);
        let url = "http://localhost";

        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = true;
        let opt_no_images: bool = false;
        let opt_silent = true;
        let opt_insecure = false;

        walk_and_embed_assets(
            &url,
            &dom.document,
            opt_no_css,
            opt_no_js,
            opt_no_images,
            "",
            opt_silent,
            opt_insecure,
            opt_no_frames,
        );

        let mut buf: Vec<u8> = Vec::new();
        serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

        assert_eq!(
            buf.iter().map(|&c| c as char).collect::<String>(),
            "<html><head></head><body><div onclick=\"\"><script src=\"\"></script>\
             <script></script></div></body></html>"
        );
    }

    #[test]
    fn test_stringify_document() {
        let html = "<div><script src=\"some.js\"></script></div>";
        let dom = html_to_dom(&html);

        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let opt_isolate: bool = false;

        assert_eq!(
            stringify_document(
                &dom.document,
                opt_no_css,
                opt_no_frames,
                opt_no_js,
                opt_no_images,
                opt_isolate,
            ),
            "<html><head></head><body><div><script src=\"some.js\"></script></div></body></html>"
        );
    }

    #[test]
    fn test_stringify_document_isolate() {
        let html = "<title>Isolated document</title><link rel=\"something\"/>\
                    <div><script src=\"some.js\"></script></div>";
        let dom = html_to_dom(&html);

        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let opt_isolate: bool = true;

        assert_eq!(
            stringify_document(
                &dom.document,
                opt_no_css,
                opt_no_frames,
                opt_no_js,
                opt_no_images,
                opt_isolate,
            ),
            "<html>\
             <head>\
             <meta \
             http-equiv=\"Content-Security-Policy\" \
             content=\"default-src 'unsafe-inline' data:;\"></meta>\
             <title>Isolated document</title>\
             <link rel=\"something\">\
             </head>\
             <body><div><script src=\"some.js\"></script></div></body>\
             </html>"
        );
    }

    #[test]
    fn test_stringify_document_no_css() {
        let html = "<!doctype html>\
                    <title>Unstyled document</title>\
                    <link rel=\"stylesheet\" href=\"main.css\"/>\
                    <div style=\"display: none;\"></div>";
        let dom = html_to_dom(&html);

        let opt_no_css: bool = true;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let opt_isolate: bool = false;

        assert_eq!(
            stringify_document(
                &dom.document,
                opt_no_css,
                opt_no_frames,
                opt_no_js,
                opt_no_images,
                opt_isolate,
            ),
            "<!DOCTYPE html>\
             <html>\
             <head>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none';\"></meta>\
             <title>Unstyled document</title>\
             <link rel=\"stylesheet\" href=\"main.css\">\
             </head>\
             <body><div style=\"display: none;\"></div></body>\
             </html>"
        );
    }

    #[test]
    fn test_stringify_document_no_frames() {
        let html = "<!doctype html><title>Frameless document</title><link rel=\"something\"/>\
                    <div><script src=\"some.js\"></script></div>";
        let dom = html_to_dom(&html);

        let opt_no_css: bool = false;
        let opt_no_frames: bool = true;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let opt_isolate: bool = false;

        assert_eq!(
            stringify_document(
                &dom.document,
                opt_no_css,
                opt_no_frames,
                opt_no_js,
                opt_no_images,
                opt_isolate,
            ),
            "<!DOCTYPE html>\
             <html>\
             <head>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"frame-src 'none';child-src 'none';\"></meta>\
             <title>Frameless document</title>\
             <link rel=\"something\">\
             </head>\
             <body><div><script src=\"some.js\"></script></div></body>\
             </html>"
        );
    }
}
