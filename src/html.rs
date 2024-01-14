use base64::prelude::*;
use chrono::prelude::*;
use encoding_rs::Encoding;
use html5ever::interface::QualName;
use html5ever::parse_document;
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::{format_tendril, TendrilSink};
use html5ever::tree_builder::{Attribute, TreeSink};
use html5ever::{local_name, namespace_url, ns, LocalName};
use markup5ever_rcdom::{Handle, NodeData, RcDom, SerializableHandle};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::Url;
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::collections::HashMap;
use std::default::Default;

use crate::css::embed_css;
use crate::js::attr_is_event_handler;
use crate::opts::Options;
use crate::url::{
    clean_url, create_data_url, is_url_and_has_protocol, resolve_url, EMPTY_IMAGE_DATA_URL,
};
use crate::utils::{parse_content_type, retrieve_asset};

struct SrcSetItem<'a> {
    path: &'a str,
    descriptor: &'a str,
}

const ICON_VALUES: &'static [&str] = &["icon", "shortcut icon"];

pub fn add_favicon(document: &Handle, favicon_data_url: String) -> RcDom {
    let mut buf: Vec<u8> = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(document.clone()),
        SerializeOpts::default(),
    )
    .expect("unable to serialize DOM into buffer");

    let mut dom = html_to_dom(&buf, "utf-8".to_string());
    let doc = dom.get_document();
    if let Some(html) = get_child_node_by_name(&doc, "html") {
        if let Some(head) = get_child_node_by_name(&html, "head") {
            let favicon_node = dom.create_element(
                QualName::new(None, ns!(), local_name!("link")),
                vec![
                    Attribute {
                        name: QualName::new(None, ns!(), local_name!("rel")),
                        value: format_tendril!("icon"),
                    },
                    Attribute {
                        name: QualName::new(None, ns!(), local_name!("href")),
                        value: format_tendril!("{}", favicon_data_url),
                    },
                ],
                Default::default(),
            );
            // Insert favicon LINK tag into HEAD
            head.children.borrow_mut().push(favicon_node.clone());
        }
    }

    dom
}

pub fn check_integrity(data: &[u8], integrity: &str) -> bool {
    if integrity.starts_with("sha256-") {
        let mut hasher = Sha256::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == integrity[7..]
    } else if integrity.starts_with("sha384-") {
        let mut hasher = Sha384::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == integrity[7..]
    } else if integrity.starts_with("sha512-") {
        let mut hasher = Sha512::new();
        hasher.update(data);
        BASE64_STANDARD.encode(hasher.finalize()) == integrity[7..]
    } else {
        false
    }
}

pub fn compose_csp(options: &Options) -> String {
    let mut string_list = vec![];

    if options.isolate {
        string_list.push("default-src 'unsafe-eval' 'unsafe-inline' data:;");
    }

    if options.no_css {
        string_list.push("style-src 'none';");
    }

    if options.no_fonts {
        string_list.push("font-src 'none';");
    }

    if options.no_frames {
        string_list.push("frame-src 'none';");
        string_list.push("child-src 'none';");
    }

    if options.no_js {
        string_list.push("script-src 'none';");
    }

    if options.no_images {
        // Note: "data:" is required for transparent pixel images to work
        string_list.push("img-src data:;");
    }

    string_list.join(" ")
}

pub fn create_metadata_tag(url: &Url) -> String {
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut clean_url: Url = clean_url(url.clone());

    // Prevent credentials from getting into metadata
    if clean_url.scheme() == "http" || clean_url.scheme() == "https" {
        // Only HTTP(S) URLs can contain credentials
        clean_url.set_username("").unwrap();
        clean_url.set_password(None).unwrap();
    }

    format!(
        "<!-- Saved from {} at {} using {} v{} -->",
        if clean_url.scheme() == "http" || clean_url.scheme() == "https" {
            &clean_url.as_str()
        } else {
            "local source"
        },
        timestamp,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )
}

pub fn determine_link_node_type(node: &Handle) -> &str {
    let mut link_type: &str = "unknown";

    if let Some(link_attr_rel_value) = get_node_attr(node, "rel") {
        if is_icon(&link_attr_rel_value) {
            link_type = "icon";
        } else if link_attr_rel_value.eq_ignore_ascii_case("stylesheet")
            || link_attr_rel_value.eq_ignore_ascii_case("alternate stylesheet")
        {
            link_type = "stylesheet";
        } else if link_attr_rel_value.eq_ignore_ascii_case("preload") {
            link_type = "preload";
        } else if link_attr_rel_value.eq_ignore_ascii_case("dns-prefetch") {
            link_type = "dns-prefetch";
        }
    }

    link_type
}

pub fn embed_srcset(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    document_url: &Url,
    srcset: &str,
    options: &Options,
    depth: u32,
) -> String {
    let mut array: Vec<SrcSetItem> = vec![];
    let re = Regex::new(r",\s+").unwrap();
    for srcset_item in re.split(srcset) {
        let parts: Vec<&str> = srcset_item.trim().split_whitespace().collect();
        if parts.len() > 0 {
            let path = parts[0].trim();
            let descriptor = if parts.len() > 1 { parts[1].trim() } else { "" };
            let srcset_real_item = SrcSetItem { path, descriptor };
            array.push(srcset_real_item);
        }
    }

    let mut result: String = "".to_string();
    let mut i: usize = array.len();
    for part in array {
        if options.no_images {
            result.push_str(EMPTY_IMAGE_DATA_URL);
        } else {
            let image_full_url: Url = resolve_url(&document_url, part.path);
            match retrieve_asset(
                cache,
                client,
                &document_url,
                &image_full_url,
                options,
                depth + 1,
            ) {
                Ok((image_data, image_final_url, image_media_type, image_charset)) => {
                    let mut image_data_url = create_data_url(
                        &image_media_type,
                        &image_charset,
                        &image_data,
                        &image_final_url,
                    );
                    // Append retreved asset as a data URL
                    image_data_url.set_fragment(image_full_url.fragment());
                    result.push_str(image_data_url.as_ref());
                }
                Err(_) => {
                    // Keep remote reference if unable to retrieve the asset
                    if image_full_url.scheme() == "http" || image_full_url.scheme() == "https" {
                        result.push_str(image_full_url.as_ref());
                    } else {
                        // Avoid breaking the structure in case if not an HTTP(S) URL
                        result.push_str(EMPTY_IMAGE_DATA_URL);
                    }
                }
            }
        }

        if !part.descriptor.is_empty() {
            result.push_str(" ");
            result.push_str(part.descriptor);
        }

        if i > 1 {
            result.push_str(", ");
        }

        i -= 1;
    }

    result
}

pub fn find_base_node(node: &Handle) -> Option<Handle> {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child in node.children.borrow().iter() {
                if let Some(base_node) = find_base_node(child) {
                    return Some(base_node);
                }
            }
        }
        NodeData::Element { ref name, .. } => {
            match name.local.as_ref() {
                "head" => {
                    return get_child_node_by_name(node, "base");
                }
                _ => {}
            }

            // Dig deeper
            for child in node.children.borrow().iter() {
                if let Some(base_node) = find_base_node(child) {
                    return Some(base_node);
                }
            }
        }
        _ => {}
    }

    None
}

pub fn find_meta_charset_or_content_type_node(node: &Handle) -> Option<Handle> {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child in node.children.borrow().iter() {
                if let Some(meta_charset_node) = find_meta_charset_or_content_type_node(child) {
                    return Some(meta_charset_node);
                }
            }
        }
        NodeData::Element { ref name, .. } => {
            match name.local.as_ref() {
                "head" => {
                    if let Some(meta_node) = get_child_node_by_name(node, "meta") {
                        if let Some(_) = get_node_attr(&meta_node, "charset") {
                            return Some(meta_node);
                        } else if let Some(meta_node_http_equiv_attr_value) =
                            get_node_attr(&meta_node, "http-equiv")
                        {
                            if meta_node_http_equiv_attr_value.eq_ignore_ascii_case("content-type")
                            {
                                return Some(meta_node);
                            }
                        }
                    }
                }
                _ => {}
            }

            // Dig deeper
            for child in node.children.borrow().iter() {
                if let Some(meta_charset_node) = find_meta_charset_or_content_type_node(child) {
                    return Some(meta_charset_node);
                }
            }
        }
        _ => {}
    }

    None
}

pub fn get_base_url(handle: &Handle) -> Option<String> {
    if let Some(base_node) = find_base_node(handle) {
        get_node_attr(&base_node, "href")
    } else {
        None
    }
}

pub fn get_charset(node: &Handle) -> Option<String> {
    if let Some(meta_charset_node) = find_meta_charset_or_content_type_node(node) {
        if let Some(meta_charset_node_attr_value) = get_node_attr(&meta_charset_node, "charset") {
            // Processing <meta charset="..." />
            return Some(meta_charset_node_attr_value);
        } else if let Some(meta_content_type_node_attr_value) =
            get_node_attr(&meta_charset_node, "content")
        {
            // Processing <meta http-equiv="content-type" content="text/html; charset=..." />
            let (_media_type, charset, _is_base64) =
                parse_content_type(&meta_content_type_node_attr_value);
            return Some(charset);
        }
    }

    return None;
}

pub fn get_child_node_by_name(parent: &Handle, node_name: &str) -> Option<Handle> {
    let children = parent.children.borrow();
    let matching_children = children.iter().find(|child| match child.data {
        NodeData::Element { ref name, .. } => &*name.local == node_name,
        _ => false,
    });
    match matching_children {
        Some(node) => Some(node.clone()),
        _ => None,
    }
}

pub fn get_node_attr(node: &Handle, attr_name: &str) -> Option<String> {
    match &node.data {
        NodeData::Element { ref attrs, .. } => {
            for attr in attrs.borrow().iter() {
                if &*attr.name.local == attr_name {
                    return Some(attr.value.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

pub fn get_node_name(node: &Handle) -> Option<&'_ str> {
    match &node.data {
        NodeData::Element { ref name, .. } => Some(name.local.as_ref()),
        _ => None,
    }
}

pub fn get_parent_node(child: &Handle) -> Handle {
    let parent = child.parent.take().clone();
    parent.and_then(|node| node.upgrade()).unwrap()
}

pub fn has_favicon(handle: &Handle) -> bool {
    let mut found_favicon: bool = false;

    match handle.data {
        NodeData::Document => {
            // Dig deeper
            for child in handle.children.borrow().iter() {
                if has_favicon(child) {
                    found_favicon = true;
                    break;
                }
            }
        }
        NodeData::Element { ref name, .. } => {
            match name.local.as_ref() {
                "link" => {
                    if let Some(attr_value) = get_node_attr(handle, "rel") {
                        if is_icon(attr_value.trim()) {
                            found_favicon = true;
                        }
                    }
                }
                _ => {}
            }

            if !found_favicon {
                // Dig deeper
                for child in handle.children.borrow().iter() {
                    if has_favicon(child) {
                        found_favicon = true;
                        break;
                    }
                }
            }
        }
        _ => {}
    }

    found_favicon
}

pub fn html_to_dom(data: &Vec<u8>, document_encoding: String) -> RcDom {
    let s: String;

    if let Some(encoding) = Encoding::for_label(document_encoding.as_bytes()) {
        let (string, _, _) = encoding.decode(&data);
        s = string.to_string();
    } else {
        s = String::from_utf8_lossy(&data).to_string();
    }

    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut s.as_bytes())
        .unwrap()
}

pub fn is_icon(attr_value: &str) -> bool {
    ICON_VALUES.contains(&attr_value.to_lowercase().as_str())
}

pub fn set_base_url(document: &Handle, desired_base_href: String) -> RcDom {
    let mut buf: Vec<u8> = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(document.clone()),
        SerializeOpts::default(),
    )
    .expect("unable to serialize DOM into buffer");

    let mut dom = html_to_dom(&buf, "utf-8".to_string());
    let doc = dom.get_document();
    if let Some(html_node) = get_child_node_by_name(&doc, "html") {
        if let Some(head_node) = get_child_node_by_name(&html_node, "head") {
            // Check if BASE node already exists in the DOM tree
            if let Some(base_node) = get_child_node_by_name(&head_node, "base") {
                set_node_attr(&base_node, "href", Some(desired_base_href));
            } else {
                let base_node = dom.create_element(
                    QualName::new(None, ns!(), local_name!("base")),
                    vec![Attribute {
                        name: QualName::new(None, ns!(), local_name!("href")),
                        value: format_tendril!("{}", desired_base_href),
                    }],
                    Default::default(),
                );

                // Insert newly created BASE node into HEAD
                head_node.children.borrow_mut().push(base_node.clone());
            }
        }
    }

    dom
}

pub fn set_charset(mut dom: RcDom, desired_charset: String) -> RcDom {
    if let Some(meta_charset_node) = find_meta_charset_or_content_type_node(&dom.document) {
        if let Some(_) = get_node_attr(&meta_charset_node, "charset") {
            set_node_attr(&meta_charset_node, "charset", Some(desired_charset));
        } else if let Some(_) = get_node_attr(&meta_charset_node, "content") {
            set_node_attr(
                &meta_charset_node,
                "content",
                Some(format!("text/html;charset={}", desired_charset)),
            );
        }
    } else {
        let meta_charset_node = dom.create_element(
            QualName::new(None, ns!(), local_name!("meta")),
            vec![Attribute {
                name: QualName::new(None, ns!(), local_name!("charset")),
                value: format_tendril!("{}", desired_charset),
            }],
            Default::default(),
        );

        // Insert newly created META charset node into HEAD
        if let Some(html_node) = get_child_node_by_name(&dom.document, "html") {
            if let Some(head_node) = get_child_node_by_name(&html_node, "head") {
                head_node
                    .children
                    .borrow_mut()
                    .push(meta_charset_node.clone());
            }
        }
    }

    dom
}

pub fn set_node_attr(node: &Handle, attr_name: &str, attr_value: Option<String>) {
    match &node.data {
        NodeData::Element { ref attrs, .. } => {
            let attrs_mut = &mut attrs.borrow_mut();
            let mut i = 0;
            let mut found_existing_attr: bool = false;

            while i < attrs_mut.len() {
                if &attrs_mut[i].name.local == attr_name {
                    found_existing_attr = true;

                    if let Some(attr_value) = attr_value.clone() {
                        let _ = &attrs_mut[i].value.clear();
                        let _ = &attrs_mut[i].value.push_slice(&attr_value.as_str());
                    } else {
                        // Remove attr completely if attr_value is not defined
                        attrs_mut.remove(i);
                        continue;
                    }
                }

                i += 1;
            }

            if !found_existing_attr {
                // Add new attribute (since originally the target node didn't have it)
                if let Some(attr_value) = attr_value.clone() {
                    let name = LocalName::from(attr_name);

                    attrs_mut.push(Attribute {
                        name: QualName::new(None, ns!(), name),
                        value: format_tendril!("{}", attr_value),
                    });
                }
            }
        }
        _ => {}
    };
}

pub fn serialize_document(mut dom: RcDom, document_encoding: String, options: &Options) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let document = dom.get_document();

    if options.isolate
        || options.no_css
        || options.no_fonts
        || options.no_frames
        || options.no_js
        || options.no_images
    {
        // Take care of CSP
        if let Some(html) = get_child_node_by_name(&document, "html") {
            if let Some(head) = get_child_node_by_name(&html, "head") {
                let meta = dom.create_element(
                    QualName::new(None, ns!(), local_name!("meta")),
                    vec![
                        Attribute {
                            name: QualName::new(None, ns!(), local_name!("http-equiv")),
                            value: format_tendril!("Content-Security-Policy"),
                        },
                        Attribute {
                            name: QualName::new(None, ns!(), local_name!("content")),
                            value: format_tendril!("{}", compose_csp(options)),
                        },
                    ],
                    Default::default(),
                );
                // The CSP meta-tag has to be prepended, never appended,
                //  since there already may be one defined in the original document,
                //   and browsers don't allow re-defining them (for obvious reasons)
                head.children.borrow_mut().reverse();
                head.children.borrow_mut().push(meta.clone());
                head.children.borrow_mut().reverse();
            }
        }
    }

    serialize(
        &mut buf,
        &SerializableHandle::from(document.clone()),
        SerializeOpts::default(),
    )
    .expect("Unable to serialize DOM into buffer");

    // Unwrap NOSCRIPT elements
    if options.unwrap_noscript {
        let s: &str = &String::from_utf8_lossy(&buf);
        let noscript_re = Regex::new(r"<(?P<c>/?noscript[^>]*)>").unwrap();
        buf = noscript_re.replace_all(&s, "<!--$c-->").as_bytes().to_vec();
    }

    if !document_encoding.is_empty() {
        if let Some(encoding) = Encoding::for_label(document_encoding.as_bytes()) {
            let s: &str = &String::from_utf8_lossy(&buf);
            let (data, _, _) = encoding.encode(s);
            buf = data.to_vec();
        }
    }

    buf
}

pub fn retrieve_and_embed_asset(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    document_url: &Url,
    node: &Handle,
    attr_name: &str,
    attr_value: &str,
    options: &Options,
    depth: u32,
) {
    let resolved_url: Url = resolve_url(document_url, attr_value);

    match retrieve_asset(
        cache,
        client,
        &document_url.clone(),
        &resolved_url,
        options,
        depth + 1,
    ) {
        Ok((data, final_url, mut media_type, charset)) => {
            let node_name: &str = get_node_name(&node).unwrap();

            // Check integrity if it's a LINK or SCRIPT element
            let mut ok_to_include: bool = true;
            if node_name == "link" || node_name == "script" {
                // Check integrity
                if let Some(node_integrity_attr_value) = get_node_attr(node, "integrity") {
                    if !node_integrity_attr_value.is_empty() {
                        ok_to_include = check_integrity(&data, &node_integrity_attr_value);
                    }

                    // Wipe the integrity attribute
                    set_node_attr(node, "integrity", None);
                }
            }

            if ok_to_include {
                let s: String;
                if let Some(encoding) = Encoding::for_label(charset.as_bytes()) {
                    let (string, _, _) = encoding.decode(&data);
                    s = string.to_string();
                } else {
                    s = String::from_utf8_lossy(&data).to_string();
                }

                if node_name == "link" && determine_link_node_type(node) == "stylesheet" {
                    // Stylesheet LINK elements require special treatment
                    let css: String = embed_css(cache, client, &final_url, &s, options, depth + 1);

                    // Create and embed data URL
                    let css_data_url =
                        create_data_url(&media_type, &charset, css.as_bytes(), &final_url);
                    set_node_attr(&node, attr_name, Some(css_data_url.to_string()));
                } else if node_name == "frame" || node_name == "iframe" {
                    // (I)FRAMEs are also quite different from conventional resources
                    let frame_dom = html_to_dom(&data, charset.clone());
                    walk_and_embed_assets(
                        cache,
                        client,
                        &final_url,
                        &frame_dom.document,
                        &options,
                        depth + 1,
                    );

                    let mut frame_data: Vec<u8> = Vec::new();
                    serialize(
                        &mut frame_data,
                        &SerializableHandle::from(frame_dom.document.clone()),
                        SerializeOpts::default(),
                    )
                    .unwrap();

                    // Create and embed data URL
                    let mut frame_data_url =
                        create_data_url(&media_type, &charset, &frame_data, &final_url);
                    frame_data_url.set_fragment(resolved_url.fragment());
                    set_node_attr(node, attr_name, Some(frame_data_url.to_string()));
                } else {
                    // Every other type of element gets processed here

                    // Parse media type for SCRIPT elements
                    if node_name == "script" {
                        if let Some(_) = get_node_attr(node, "src") {
                            if let Some(script_node_type_attr_value) = get_node_attr(node, "type") {
                                media_type = script_node_type_attr_value.to_string();
                            } else {
                                // Fallback to default one if it's not specified
                                media_type = "application/javascript".to_string();
                            }
                        }
                    }

                    // Create and embed data URL
                    let mut data_url = create_data_url(&media_type, &charset, &data, &final_url);
                    data_url.set_fragment(resolved_url.fragment());
                    set_node_attr(node, attr_name, Some(data_url.to_string()));
                }
            }
        }
        Err(_) => {
            if resolved_url.scheme() == "http" || resolved_url.scheme() == "https" {
                // Keep remote references if unable to retrieve the asset
                set_node_attr(node, attr_name, Some(resolved_url.to_string()));
            } else {
                // Remove local references if they can't be successfully embedded as data URLs
                set_node_attr(node, attr_name, None);
            }
        }
    }
}

pub fn walk_and_embed_assets(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    document_url: &Url,
    node: &Handle,
    options: &Options,
    depth: u32,
) {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child in node.children.borrow().iter() {
                walk_and_embed_assets(cache, client, &document_url, child, options, depth);
            }
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            match name.local.as_ref() {
                "meta" => {
                    if let Some(meta_attr_http_equiv_value) = get_node_attr(node, "http-equiv") {
                        let meta_attr_http_equiv_value: &str = &meta_attr_http_equiv_value;
                        if meta_attr_http_equiv_value.eq_ignore_ascii_case("refresh")
                            || meta_attr_http_equiv_value.eq_ignore_ascii_case("location")
                        {
                            // Remove http-equiv attributes from META nodes if they're able to control the page
                            set_node_attr(&node, "http-equiv", None);
                        }
                    }
                }
                "link" => {
                    let link_type: &str = determine_link_node_type(node);

                    if link_type == "icon" {
                        // Find and resolve LINK's href attribute
                        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                            if !options.no_images && !link_attr_href_value.is_empty() {
                                retrieve_and_embed_asset(
                                    cache,
                                    client,
                                    &document_url,
                                    node,
                                    "href",
                                    &link_attr_href_value,
                                    options,
                                    depth,
                                );
                            } else {
                                set_node_attr(node, "href", None);
                            }
                        }
                    } else if link_type == "stylesheet" {
                        // Resolve LINK's href attribute
                        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                            if options.no_css {
                                set_node_attr(node, "href", None);
                                // Wipe integrity attribute
                                set_node_attr(node, "integrity", None);
                            } else {
                                if !link_attr_href_value.is_empty() {
                                    retrieve_and_embed_asset(
                                        cache,
                                        client,
                                        &document_url,
                                        node,
                                        "href",
                                        &link_attr_href_value,
                                        options,
                                        depth,
                                    );
                                }
                            }
                        }
                    } else if link_type == "preload" || link_type == "dns-prefetch" {
                        // Since all resources are embedded as data URLs, preloading and prefetching are not necessary
                        set_node_attr(node, "rel", None);
                    } else {
                        // Make sure that all other LINKs' href attributes are full URLs
                        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                            let href_full_url: Url =
                                resolve_url(&document_url, &link_attr_href_value);
                            set_node_attr(node, "href", Some(href_full_url.to_string()));
                        }
                    }
                }
                "base" => {
                    if document_url.scheme() == "http" || document_url.scheme() == "https" {
                        // Ensure the BASE node doesn't have a relative URL
                        if let Some(base_attr_href_value) = get_node_attr(node, "href") {
                            let href_full_url: Url =
                                resolve_url(document_url, &base_attr_href_value);
                            set_node_attr(node, "href", Some(href_full_url.to_string()));
                        }
                    }
                }
                "body" => {
                    // Read and remember background attribute value of this BODY node
                    if let Some(body_attr_background_value) = get_node_attr(node, "background") {
                        // Remove background BODY node attribute by default
                        set_node_attr(node, "background", None);

                        if !options.no_images && !body_attr_background_value.is_empty() {
                            retrieve_and_embed_asset(
                                cache,
                                client,
                                document_url,
                                node,
                                "background",
                                &body_attr_background_value,
                                options,
                                depth,
                            );
                        }
                    }
                }
                "img" => {
                    // Find src and data-src attribute(s)
                    let img_attr_src_value: Option<String> = get_node_attr(node, "src");
                    let img_attr_data_src_value: Option<String> = get_node_attr(node, "data-src");

                    if options.no_images {
                        // Put empty images into src and data-src attributes
                        if img_attr_src_value != None {
                            set_node_attr(node, "src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                        }
                        if img_attr_data_src_value != None {
                            set_node_attr(node, "data-src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                        }
                    } else {
                        if img_attr_src_value.clone().unwrap_or_default().is_empty()
                            && img_attr_data_src_value
                                .clone()
                                .unwrap_or_default()
                                .is_empty()
                        {
                            // Add empty src attribute
                            set_node_attr(node, "src", Some("".to_string()));
                        } else {
                            // Add data URL src attribute
                            let img_full_url: String = if !img_attr_data_src_value
                                .clone()
                                .unwrap_or_default()
                                .is_empty()
                            {
                                img_attr_data_src_value.unwrap_or_default()
                            } else {
                                img_attr_src_value.unwrap_or_default()
                            };
                            retrieve_and_embed_asset(
                                cache,
                                client,
                                document_url,
                                node,
                                "src",
                                &img_full_url,
                                options,
                                depth,
                            );
                        }
                    }

                    // Resolve srcset attribute
                    if let Some(img_srcset) = get_node_attr(node, "srcset") {
                        if !img_srcset.is_empty() {
                            let resolved_srcset: String = embed_srcset(
                                cache,
                                client,
                                &document_url,
                                &img_srcset,
                                options,
                                depth,
                            );
                            set_node_attr(node, "srcset", Some(resolved_srcset));
                        }
                    }
                }
                "svg" => {
                    if options.no_images {
                        node.children.borrow_mut().clear();
                    }
                }
                "input" => {
                    if let Some(input_attr_type_value) = get_node_attr(node, "type") {
                        if input_attr_type_value.eq_ignore_ascii_case("image") {
                            if let Some(input_attr_src_value) = get_node_attr(node, "src") {
                                if options.no_images || input_attr_src_value.is_empty() {
                                    let value = if input_attr_src_value.is_empty() {
                                        ""
                                    } else {
                                        EMPTY_IMAGE_DATA_URL
                                    };
                                    set_node_attr(node, "src", Some(value.to_string()));
                                } else {
                                    retrieve_and_embed_asset(
                                        cache,
                                        client,
                                        document_url,
                                        node,
                                        "src",
                                        &input_attr_src_value,
                                        options,
                                        depth,
                                    );
                                }
                            }
                        }
                    }
                }
                "image" => {
                    let mut image_href: String = "".to_string();

                    if let Some(image_attr_href_value) = get_node_attr(node, "href") {
                        image_href = image_attr_href_value;
                        if options.no_images {
                            set_node_attr(node, "href", None);
                        }
                    }

                    if let Some(image_attr_xlink_href_value) = get_node_attr(node, "xlink:href") {
                        image_href = image_attr_xlink_href_value;
                        if options.no_images {
                            set_node_attr(node, "xlink:href", None);
                        }
                    }

                    if !options.no_images && !image_href.is_empty() {
                        retrieve_and_embed_asset(
                            cache,
                            client,
                            document_url,
                            node,
                            "href",
                            &image_href,
                            options,
                            depth,
                        );
                    }
                }
                "source" => {
                    let parent_node = get_parent_node(node);
                    let parent_node_name: &str = get_node_name(&parent_node).unwrap_or_default();

                    if let Some(source_attr_src_value) = get_node_attr(node, "src") {
                        if parent_node_name == "audio" {
                            if options.no_audio {
                                set_node_attr(node, "src", None);
                            } else {
                                retrieve_and_embed_asset(
                                    cache,
                                    client,
                                    document_url,
                                    node,
                                    "src",
                                    &source_attr_src_value,
                                    options,
                                    depth,
                                );
                            }
                        } else if parent_node_name == "video" {
                            if options.no_video {
                                set_node_attr(node, "src", None);
                            } else {
                                retrieve_and_embed_asset(
                                    cache,
                                    client,
                                    document_url,
                                    node,
                                    "src",
                                    &source_attr_src_value,
                                    options,
                                    depth,
                                );
                            }
                        }
                    }

                    if let Some(source_attr_srcset_value) = get_node_attr(node, "srcset") {
                        if parent_node_name == "picture" {
                            if !source_attr_srcset_value.is_empty() {
                                if options.no_images {
                                    set_node_attr(
                                        node,
                                        "srcset",
                                        Some(EMPTY_IMAGE_DATA_URL.to_string()),
                                    );
                                } else {
                                    let resolved_srcset: String = embed_srcset(
                                        cache,
                                        client,
                                        &document_url,
                                        &source_attr_srcset_value,
                                        options,
                                        depth,
                                    );
                                    set_node_attr(node, "srcset", Some(resolved_srcset));
                                }
                            }
                        }
                    }
                }
                "a" | "area" => {
                    if let Some(anchor_attr_href_value) = get_node_attr(node, "href") {
                        if anchor_attr_href_value
                            .clone()
                            .trim()
                            .starts_with("javascript:")
                        {
                            if options.no_js {
                                // Replace with empty JS call to preserve original behavior
                                set_node_attr(node, "href", Some("javascript:;".to_string()));
                            }
                        } else {
                            // Don't touch mailto: links or hrefs which begin with a hash sign
                            if !anchor_attr_href_value.clone().starts_with('#')
                                && !is_url_and_has_protocol(&anchor_attr_href_value.clone())
                            {
                                let href_full_url: Url =
                                    resolve_url(document_url, &anchor_attr_href_value);
                                set_node_attr(node, "href", Some(href_full_url.to_string()));
                            }
                        }
                    }
                }
                "script" => {
                    // Read values of integrity and src attributes
                    let script_attr_src: Option<String> = get_node_attr(node, "src");

                    if options.no_js {
                        // Empty inner content
                        node.children.borrow_mut().clear();
                        // Remove src attribute
                        if script_attr_src != None {
                            set_node_attr(node, "src", None);
                            // Wipe integrity attribute
                            set_node_attr(node, "integrity", None);
                        }
                    } else if !script_attr_src.clone().unwrap_or_default().is_empty() {
                        retrieve_and_embed_asset(
                            cache,
                            client,
                            document_url,
                            node,
                            "src",
                            &script_attr_src.unwrap_or_default(),
                            options,
                            depth,
                        );
                    }
                }
                "style" => {
                    if options.no_css {
                        // Empty inner content of STYLE tags
                        node.children.borrow_mut().clear();
                    } else {
                        for child_node in node.children.borrow_mut().iter_mut() {
                            if let NodeData::Text { ref contents } = child_node.data {
                                let mut tendril = contents.borrow_mut();
                                let replacement = embed_css(
                                    cache,
                                    client,
                                    &document_url,
                                    tendril.as_ref(),
                                    options,
                                    depth,
                                );
                                tendril.clear();
                                tendril.push_slice(&replacement);
                            }
                        }
                    }
                }
                "form" => {
                    if let Some(form_attr_action_value) = get_node_attr(node, "action") {
                        // Modify action property to ensure it's a full URL
                        let form_action_full_url: Url =
                            resolve_url(document_url, &form_attr_action_value);
                        set_node_attr(node, "action", Some(form_action_full_url.to_string()));
                    }
                }
                "frame" | "iframe" => {
                    if let Some(frame_attr_src_value) = get_node_attr(node, "src") {
                        if options.no_frames {
                            // Empty the src attribute
                            set_node_attr(node, "src", Some("".to_string()));
                        } else {
                            // Ignore (i)frames with empty source (they cause infinite loops)
                            if !frame_attr_src_value.trim().is_empty() {
                                retrieve_and_embed_asset(
                                    cache,
                                    client,
                                    &document_url,
                                    node,
                                    "src",
                                    &frame_attr_src_value,
                                    options,
                                    depth,
                                );
                            }
                        }
                    }
                }
                "audio" => {
                    // Embed audio source
                    if let Some(audio_attr_src_value) = get_node_attr(node, "src") {
                        if options.no_audio {
                            set_node_attr(node, "src", None);
                        } else {
                            retrieve_and_embed_asset(
                                cache,
                                client,
                                document_url,
                                node,
                                "src",
                                &audio_attr_src_value,
                                options,
                                depth,
                            );
                        }
                    }
                }
                "video" => {
                    // Embed video source
                    if let Some(video_attr_src_value) = get_node_attr(node, "src") {
                        if options.no_video {
                            set_node_attr(node, "src", None);
                        } else {
                            retrieve_and_embed_asset(
                                cache,
                                client,
                                document_url,
                                node,
                                "src",
                                &video_attr_src_value,
                                options,
                                depth,
                            );
                        }
                    }

                    // Embed poster images
                    if let Some(video_attr_poster_value) = get_node_attr(node, "poster") {
                        // Skip posters with empty source
                        if !video_attr_poster_value.is_empty() {
                            if options.no_images {
                                set_node_attr(
                                    node,
                                    "poster",
                                    Some(EMPTY_IMAGE_DATA_URL.to_string()),
                                );
                            } else {
                                retrieve_and_embed_asset(
                                    cache,
                                    client,
                                    document_url,
                                    node,
                                    "poster",
                                    &video_attr_poster_value,
                                    options,
                                    depth,
                                );
                            }
                        }
                    }
                }
                "noscript" => {
                    for child_node in node.children.borrow_mut().iter_mut() {
                        match child_node.data {
                            NodeData::Text { ref contents } => {
                                // Get contents of NOSCRIPT node
                                let mut noscript_contents = contents.borrow_mut();
                                // Parse contents of NOSCRIPT node as DOM
                                let noscript_contents_dom: RcDom = html_to_dom(
                                    &noscript_contents.as_bytes().to_vec(),
                                    "".to_string(),
                                );
                                // Embed assets of NOSCRIPT node contents
                                walk_and_embed_assets(
                                    cache,
                                    client,
                                    &document_url,
                                    &noscript_contents_dom.document,
                                    &options,
                                    depth,
                                );
                                // Get rid of original contents
                                noscript_contents.clear();
                                // Insert HTML containing embedded assets into NOSCRIPT node
                                if let Some(html) =
                                    get_child_node_by_name(&noscript_contents_dom.document, "html")
                                {
                                    if let Some(body) = get_child_node_by_name(&html, "body") {
                                        let mut buf: Vec<u8> = Vec::new();
                                        serialize(
                                            &mut buf,
                                            &SerializableHandle::from(body.clone()),
                                            SerializeOpts::default(),
                                        )
                                        .expect("Unable to serialize DOM into buffer");
                                        let result = String::from_utf8_lossy(&buf);
                                        noscript_contents.push_slice(&result);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }

            // Process style attributes
            if options.no_css {
                // Get rid of style attributes
                set_node_attr(node, "style", None);
            } else {
                // Embed URLs found within the style attribute of this node
                if let Some(node_attr_style_value) = get_node_attr(node, "style") {
                    let embedded_style = embed_css(
                        cache,
                        client,
                        &document_url,
                        &node_attr_style_value,
                        options,
                        depth,
                    );
                    set_node_attr(node, "style", Some(embedded_style));
                }
            }

            // Strip all JS from document
            if options.no_js {
                let attrs_mut = &mut attrs.borrow_mut();
                // Get rid of JS event attributes
                let mut js_attr_indexes = Vec::new();
                for (i, attr) in attrs_mut.iter().enumerate() {
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
                walk_and_embed_assets(cache, client, &document_url, child, options, depth);
            }
        }
        _ => {
            // Note: in case of options.no_js being set to true, there's no need to worry about
            //       getting rid of comments that may contain scripts, e.g. <!--[if IE]><script>...
            //       since that's not part of W3C standard and therefore gets ignored
            //       by browsers other than IE [5, 9]
        }
    }
}
