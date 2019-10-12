use html5ever::interface::QualName;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::{format_tendril, TendrilSink};
use html5ever::tree_builder::{Attribute, TreeSink};
use html5ever::{local_name, namespace_url, ns};
use http::retrieve_asset;
use js::attr_is_event_handler;
use std::default::Default;
use utils::{data_to_dataurl, is_valid_url, resolve_url, url_has_protocol};

lazy_static! {
    static ref EMPTY_STRING: String = String::new();
}

const ICON_VALUES: [&str; 5] = [
    "icon",
    "shortcut icon",
    "mask-icon",
    "apple-touch-icon",
    "fluid-icon",
];

const TRANSPARENT_PIXEL: &str =
    "data:image/png;base64,\
     iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

pub fn get_parent_node(node: &Handle) -> Handle {
    let parent = node.parent.take().clone();
    parent.and_then(|node| node.upgrade()).unwrap()
}

pub fn get_node_name(node: &Handle) -> String {
    match &node.data {
        NodeData::Element { ref name, .. } => name.local.as_ref().to_string(),
        _ => EMPTY_STRING.clone(),
    }
}

pub fn is_icon(attr_value: &str) -> bool {
    ICON_VALUES.contains(&&*attr_value.to_lowercase())
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
                                    let (favicon_dataurl, _) = retrieve_asset(
                                        &href_full_url,
                                        true,
                                        "",
                                        opt_user_agent,
                                        opt_silent,
                                        opt_insecure,
                                    )
                                    .unwrap_or((EMPTY_STRING.clone(), EMPTY_STRING.clone()));
                                    attr.value.clear();
                                    attr.value.push_slice(favicon_dataurl.as_str());
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
                                    let (css_dataurl, _) = retrieve_asset(
                                        &href_full_url,
                                        true,
                                        "text/css",
                                        opt_user_agent,
                                        opt_silent,
                                        opt_insecure,
                                    )
                                    .unwrap_or((EMPTY_STRING.clone(), EMPTY_STRING.clone()));
                                    attr.value.clear();
                                    attr.value.push_slice(css_dataurl.as_str());
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
                                let (img_dataurl, _) = retrieve_asset(
                                    &src_full_url,
                                    true,
                                    "",
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                )
                                .unwrap_or((EMPTY_STRING.clone(), EMPTY_STRING.clone()));
                                attr.value.clear();
                                attr.value.push_slice(img_dataurl.as_str());
                            }
                        }
                    }

                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "srcset" {
                            let value = attr.value.to_string();

                            // Ignore images with empty source
                            if value == EMPTY_STRING.clone() {
                                continue;
                            }

                            if opt_no_images {
                                attr.value.clear();
                                attr.value.push_slice(TRANSPARENT_PIXEL);
                            } else {
                                let splitted_src_set:Vec<&str> = value.split(' ').collect();
                                let src_full_url: String =
                                    resolve_url(&url, &splitted_src_set[0]).unwrap_or(EMPTY_STRING.clone());
                                let (img_dataurl, _) = retrieve_asset(
                                    &src_full_url,
                                    true,
                                    "",
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                )
                                .unwrap_or((EMPTY_STRING.clone(), EMPTY_STRING.clone()));
                                attr.value.clear();
                                attr.value.push_slice(img_dataurl.as_str());
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
                                    let (source_dataurl, _) = retrieve_asset(
                                        &srcset_full_url,
                                        true,
                                        "",
                                        opt_user_agent,
                                        opt_silent,
                                        opt_insecure,
                                    )
                                    .unwrap_or((EMPTY_STRING.clone(), EMPTY_STRING.clone()));
                                    attr.value.clear();
                                    attr.value.push_slice(source_dataurl.as_str());
                                }
                            }
                        }
                    }
                }
                "a" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "href" {
                            // Don't touch email links or hrefs which begin with a hash sign
                            if attr.value.starts_with('#') || url_has_protocol(&attr.value) {
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
                                let (js_dataurl, _) = retrieve_asset(
                                    &src_full_url,
                                    true,
                                    "application/javascript",
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                )
                                .unwrap_or((EMPTY_STRING.clone(), EMPTY_STRING.clone()));
                                attr.value.clear();
                                attr.value.push_slice(js_dataurl.as_str());
                            }
                        }
                    }
                }
                "style" => {
                    if opt_no_css {
                        // Empty inner content of STYLE tags
                        node.children.borrow_mut().clear();
                    }
                }
                "form" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "action" {
                            // Modify action to be a full URL
                            if !is_valid_url(&attr.value) {
                                let href_full_url: String =
                                    resolve_url(&url, &attr.value.to_string())
                                        .unwrap_or(EMPTY_STRING.clone());
                                attr.value.clear();
                                attr.value.push_slice(href_full_url.as_str());
                            }
                        }
                    }
                }
                "iframe" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "src" {
                            if opt_no_frames {
                                // Empty the src attribute
                                attr.value.clear();
                                continue;
                            }

                            let iframe_src: String = attr.value.to_string();

                            // Ignore iframes with empty source (they cause infinite loops)
                            if iframe_src == EMPTY_STRING.clone() {
                                continue;
                            }

                            let src_full_url: String =
                                resolve_url(&url, &iframe_src).unwrap_or(EMPTY_STRING.clone());
                            let (iframe_data, iframe_final_url) = retrieve_asset(
                                &src_full_url,
                                false,
                                "text/html",
                                opt_user_agent,
                                opt_silent,
                                opt_insecure,
                            )
                            .unwrap_or((EMPTY_STRING.clone(), src_full_url));
                            let dom = html_to_dom(&iframe_data);
                            walk_and_embed_assets(
                                &iframe_final_url,
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
                            serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();
                            let iframe_dataurl = data_to_dataurl("text/html", &buf);
                            attr.value.clear();
                            attr.value.push_slice(iframe_dataurl.as_str());
                        }
                    }
                }
                "video" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "poster" {
                            let video_poster = attr.value.to_string();

                            // Skip posters with empty source
                            if video_poster == EMPTY_STRING.clone() {
                                continue;
                            }

                            if opt_no_images {
                                attr.value.clear();
                            } else {
                                let poster_full_url: String = resolve_url(&url, &video_poster)
                                    .unwrap_or(EMPTY_STRING.clone());
                                let (poster_dataurl, _) = retrieve_asset(
                                    &poster_full_url,
                                    true,
                                    "",
                                    opt_user_agent,
                                    opt_silent,
                                    opt_insecure,
                                )
                                .unwrap_or((poster_full_url, EMPTY_STRING.clone()));
                                attr.value.clear();
                                attr.value.push_slice(poster_dataurl.as_str());
                            }
                        }
                    }
                }
                _ => {}
            }

            if opt_no_css {
                // Get rid of style attributes
                let mut style_attr_indexes = Vec::new();
                for (i, attr) in attrs_mut.iter_mut().enumerate() {
                    if attr.name.local.to_lowercase() == "style" {
                        style_attr_indexes.push(i);
                    }
                }
                style_attr_indexes.reverse();
                for attr_index in style_attr_indexes {
                    attrs_mut.remove(attr_index);
                }
            }

            if opt_no_js {
                // Get rid of JS event attributes
                let mut js_attr_indexes = Vec::new();
                for (i, attr) in attrs_mut.iter_mut().enumerate() {
                    if attr_is_event_handler(&attr.name.local) {
                        js_attr_indexes.push(i);
                    }
                }
                js_attr_indexes.reverse();
                for attr_index in js_attr_indexes {
                    attrs_mut.remove(attr_index);
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
        let mut content_attr = EMPTY_STRING.clone();
        if opt_isolate {
            content_attr += " default-src 'unsafe-inline' data:;";
        }
        if opt_no_css {
            content_attr += " style-src 'none';";
        }
        if opt_no_frames {
            content_attr += " frame-src 'none';child-src 'none';";
        }
        if opt_no_js {
            content_attr += " script-src 'none';";
        }
        if opt_no_images {
            content_attr += " img-src data:;";
        }
        content_attr = content_attr.trim().to_string();

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

        serialize(&mut buf, &doc, SerializeOpts::default())
            .expect("unable to serialize DOM into buffer");
        result = String::from_utf8(buf).unwrap();
        // Note: we can't make it isolate the page right away since it may have no HEAD element,
        //       ergo we have to serialize, parse DOM again, and then finally serialize the result
    }

    result
}
