use crate::http::retrieve_asset;
use crate::js::attr_is_event_handler;
use crate::utils::{
    data_to_dataurl, is_valid_url, resolve_css_imports, resolve_url, url_has_protocol,
};
use html5ever::interface::QualName;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::{format_tendril, Tendril, TendrilSink};
use html5ever::tree_builder::{Attribute, TreeSink};
use html5ever::{local_name, namespace_url, ns};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::default::Default;

const ICON_VALUES: &[&str] = &[
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

pub fn get_node_name(node: &Handle) -> &'_ str {
    match &node.data {
        NodeData::Element { ref name, .. } => name.local.as_ref(),
        _ => "",
    }
}

pub fn is_icon(attr_value: &str) -> bool {
    ICON_VALUES
        .iter()
        .find(|a| attr_value.eq_ignore_ascii_case(a))
        .is_some()
}

pub fn walk_and_embed_assets(
    cache: &mut HashMap<String, String>,
    client: &Client,
    url: &str,
    node: &Handle,
    opt_no_css: bool,
    opt_no_js: bool,
    opt_no_images: bool,
    opt_silent: bool,
    opt_no_frames: bool,
) {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child in node.children.borrow().iter() {
                walk_and_embed_assets(
                    cache,
                    client,
                    &url,
                    child,
                    opt_no_css,
                    opt_no_js,
                    opt_no_images,
                    opt_silent,
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
                    // Remove integrity attributes
                    let mut i = 0;
                    while i < attrs_mut.len() {
                        let attr_name = attrs_mut[i].name.local.as_ref();
                        if attr_name.eq_ignore_ascii_case("integrity") {
                            attrs_mut.remove(i);
                        } else {
                            i += 1;
                        }
                    }

                    enum LinkType {
                        Icon,
                        Stylesheet,
                        Preload,
                        DnsPrefetch,
                        Unknown,
                    }

                    let mut link_type = LinkType::Unknown;
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "rel" {
                            let value = attr.value.trim();
                            if is_icon(value) {
                                link_type = LinkType::Icon;
                                break;
                            } else if value.eq_ignore_ascii_case("stylesheet") {
                                link_type = LinkType::Stylesheet;
                                break;
                            } else if value.eq_ignore_ascii_case("preload") {
                                link_type = LinkType::Preload;
                                break;
                            } else if value.eq_ignore_ascii_case("dns-prefetch") {
                                link_type = LinkType::DnsPrefetch;
                                break;
                            }
                        }
                    }
                    let link_type = link_type;

                    match link_type {
                        LinkType::Icon => {
                            for attr in attrs_mut.iter_mut() {
                                if &attr.name.local == "href" {
                                    if opt_no_images {
                                        attr.value.clear();
                                    } else {
                                        let href_full_url = resolve_url(&url, attr.value.as_ref())
                                            .unwrap_or_default();
                                        let (favicon_dataurl, _) = retrieve_asset(
                                            cache,
                                            client,
                                            &href_full_url,
                                            true,
                                            "",
                                            opt_silent,
                                        )
                                        .unwrap_or_default();
                                        attr.value.clear();
                                        attr.value.push_slice(favicon_dataurl.as_str());
                                    }
                                }
                            }
                        }
                        LinkType::Stylesheet => {
                            for attr in attrs_mut.iter_mut() {
                                if &attr.name.local == "href" {
                                    if opt_no_css {
                                        attr.value.clear();
                                    } else {
                                        let href_full_url = resolve_url(&url, &attr.value.as_ref())
                                            .unwrap_or_default();
                                        let replacement_text = match retrieve_asset(
                                            cache,
                                            client,
                                            &href_full_url,
                                            false,
                                            "text/css",
                                            opt_silent,
                                        ) {
                                            // On successful retrieval, traverse CSS
                                            Ok((css_data, _)) => resolve_css_imports(
                                                cache,
                                                client,
                                                &css_data,
                                                true,
                                                &href_full_url,
                                                opt_no_images,
                                                opt_silent,
                                            ),

                                            // If a network error occured, warn
                                            Err(e) => {
                                                eprintln!("Warning: {}", e);

                                                // If failed to resolve, replace with absolute URL
                                                href_full_url
                                            }
                                        };

                                        attr.value.clear();
                                        attr.value.push_slice(&replacement_text);
                                    }
                                }
                            }
                        }
                        LinkType::Preload | LinkType::DnsPrefetch => {
                            // Since all resources are embedded as data URL, preloading and prefetching are unnecessary
                            if let Some(attr) =
                                attrs_mut.iter_mut().find(|a| &a.name.local == "href")
                            {
                                attr.value.clear();
                            }
                        }
                        LinkType::Unknown => {
                            for attr in attrs_mut.iter_mut() {
                                if &attr.name.local == "href" {
                                    let href_full_url =
                                        resolve_url(&url, attr.value.as_ref()).unwrap_or_default();
                                    attr.value.clear();
                                    attr.value.push_slice(&href_full_url.as_str());
                                }
                            }
                        }
                    }
                }
                "img" => {
                    // Find source tags
                    let mut found_src: Option<Attribute> = None;
                    let mut found_datasrc: Option<Attribute> = None;
                    let mut i = 0;
                    while i < attrs_mut.len() {
                        let name = attrs_mut[i].name.local.as_ref();
                        if name.eq_ignore_ascii_case("src") {
                            found_src = Some(attrs_mut.remove(i));
                        } else if name.eq_ignore_ascii_case("data-src") {
                            found_datasrc = Some(attrs_mut.remove(i));
                        } else {
                            i += 1;
                        }
                    }

                    // If images are disabled, clear both sources
                    if opt_no_images {
                        attrs_mut.push(Attribute {
                            name: QualName::new(None, ns!(), local_name!("src")),
                            value: Tendril::from_slice(TRANSPARENT_PIXEL),
                        });
                    } else if let Some((dataurl, _)) = found_datasrc
                        .iter()
                        .chain(&found_src) // Give dataurl priority
                        .map(|attr| attr.value.trim())
                        .filter(|src| !src.is_empty()) // Ignore empty srcs
                        .next()
                        .and_then(|src| resolve_url(&url, src).ok()) // Make absolute
                        .and_then(|abs_src| // Download and convert to dataurl
                            retrieve_asset(
                                cache,
                                client,
                                &abs_src,
                                true,
                                "",
                                opt_silent,
                            ).ok())
                    {
                        // Add the new dataurl src attribute
                        attrs_mut.push(Attribute {
                            name: QualName::new(None, ns!(), local_name!("src")),
                            value: Tendril::from_slice(dataurl.as_ref()),
                        });
                    }
                }
                "source" => {
                    for attr in attrs_mut.iter_mut() {
                        let attr_name: &str = &attr.name.local;

                        if attr_name == "src" {
                            let src_full_url = resolve_url(&url, attr.value.trim())
                                .unwrap_or_else(|_| attr.value.to_string());
                            attr.value.clear();
                            attr.value.push_slice(src_full_url.as_str());
                        } else if attr_name == "srcset" {
                            if get_node_name(&get_parent_node(&node)) == "picture" {
                                if opt_no_images {
                                    attr.value.clear();
                                    attr.value.push_slice(TRANSPARENT_PIXEL);
                                } else {
                                    let srcset_full_url =
                                        resolve_url(&url, attr.value.trim()).unwrap_or_default();
                                    let (source_dataurl, _) = retrieve_asset(
                                        cache,
                                        client,
                                        &srcset_full_url,
                                        true,
                                        "",
                                        opt_silent,
                                    )
                                    .unwrap_or((str!(), str!()));
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
                            let attr_value = attr.value.trim();
                            // Don't touch email links or hrefs which begin with a hash sign
                            if attr_value.starts_with('#') || url_has_protocol(attr_value) {
                                continue;
                            }

                            let href_full_url = resolve_url(&url, attr_value).unwrap_or_default();
                            attr.value.clear();
                            attr.value.push_slice(href_full_url.as_str());
                        }
                    }
                }
                "script" => {
                    // Remove integrity attributes
                    let mut i = 0;
                    while i < attrs_mut.len() {
                        let attr_name = attrs_mut[i].name.local.as_ref();
                        if attr_name.eq_ignore_ascii_case("integrity") {
                            attrs_mut.remove(i);
                        } else {
                            i += 1;
                        }
                    }

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
                                let src_full_url =
                                    resolve_url(&url, attr.value.trim()).unwrap_or_default();
                                let (js_dataurl, _) = retrieve_asset(
                                    cache,
                                    client,
                                    &src_full_url,
                                    true,
                                    "application/javascript",
                                    opt_silent,
                                )
                                .unwrap_or((str!(), str!()));
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
                    } else {
                        for node in node.children.borrow_mut().iter_mut() {
                            if let NodeData::Text { ref contents } = node.data {
                                let mut tendril = contents.borrow_mut();
                                let replacement = resolve_css_imports(
                                    cache,
                                    client,
                                    tendril.as_ref(),
                                    false,
                                    &url,
                                    opt_no_images,
                                    opt_silent,
                                );
                                tendril.clear();
                                tendril.push_slice(&replacement);
                            }
                        }
                    }
                }
                "form" => {
                    for attr in attrs_mut.iter_mut() {
                        if &attr.name.local == "action" {
                            let attr_value = attr.value.trim();
                            // Modify action to be a full URL
                            if !is_valid_url(attr_value) {
                                let href_full_url =
                                    resolve_url(&url, attr_value).unwrap_or_default();
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

                            let iframe_src = attr.value.trim();

                            // Ignore iframes with empty source (they cause infinite loops)
                            if iframe_src.is_empty() {
                                continue;
                            }

                            let src_full_url = resolve_url(&url, iframe_src).unwrap_or_default();
                            let (iframe_data, iframe_final_url) = retrieve_asset(
                                cache,
                                client,
                                &src_full_url,
                                false,
                                "text/html",
                                opt_silent,
                            )
                            .unwrap_or((str!(), src_full_url));
                            let dom = html_to_dom(&iframe_data);
                            walk_and_embed_assets(
                                cache,
                                client,
                                &iframe_final_url,
                                &dom.document,
                                opt_no_css,
                                opt_no_js,
                                opt_no_images,
                                opt_silent,
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
                            let video_poster = attr.value.trim();

                            // Skip posters with empty source
                            if video_poster.is_empty() {
                                continue;
                            }

                            if opt_no_images {
                                attr.value.clear();
                            } else {
                                let poster_full_url =
                                    resolve_url(&url, video_poster).unwrap_or_default();
                                let (poster_dataurl, _) = retrieve_asset(
                                    cache,
                                    client,
                                    &poster_full_url,
                                    true,
                                    "",
                                    opt_silent,
                                )
                                .unwrap_or((poster_full_url, str!()));
                                attr.value.clear();
                                attr.value.push_slice(poster_dataurl.as_str());
                            }
                        }
                    }
                }
                _ => {}
            }

            // Process style attributes
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
            } else {
                // Otherwise, parse any links found in the attributes
                for attribute in attrs_mut
                    .iter_mut()
                    .filter(|a| a.name.local.as_ref().eq_ignore_ascii_case("style"))
                {
                    let replacement = resolve_css_imports(
                        cache,
                        client,
                        attribute.value.as_ref(),
                        false,
                        &url,
                        opt_no_images,
                        opt_silent,
                    );
                    attribute.value.clear();
                    attribute.value.push_slice(&replacement);
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
                    cache,
                    client,
                    &url,
                    child,
                    opt_no_css,
                    opt_no_js,
                    opt_no_images,
                    opt_silent,
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
        _ => handle.clone(),
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

    let mut result = String::from_utf8(buf).unwrap();

    if opt_isolate || opt_no_css || opt_no_frames || opt_no_js || opt_no_images {
        let mut buf: Vec<u8> = Vec::new();
        let mut dom = html_to_dom(&result);
        let doc = dom.get_document();
        let html = get_child_node_by_name(&doc, "html");
        let head = get_child_node_by_name(&html, "head");
        let mut content_attr = str!();
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

        let meta = dom.create_element(
            QualName::new(None, ns!(), local_name!("meta")),
            vec![
                Attribute {
                    name: QualName::new(None, ns!(), local_name!("http-equiv")),
                    value: format_tendril!("Content-Security-Policy"),
                },
                Attribute {
                    name: QualName::new(None, ns!(), local_name!("content")),
                    value: format_tendril!("{}", content_attr.trim()),
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
