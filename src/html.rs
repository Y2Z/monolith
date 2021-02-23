use base64;
use chrono::prelude::*;
use html5ever::interface::QualName;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::{format_tendril, TendrilSink};
use html5ever::tree_builder::{Attribute, TreeSink};
use html5ever::{local_name, namespace_url, ns, LocalName};
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
    data_to_data_url, get_url_fragment, is_http_url, resolve_url, url_has_protocol,
    url_with_fragment,
};
use crate::utils::retrieve_asset;

struct SrcSetItem<'a> {
    path: &'a str,
    descriptor: &'a str,
}

const ICON_VALUES: &[&str] = &["icon", "shortcut icon"];

pub fn add_favicon(document: &Handle, favicon_data_url: String) -> RcDom {
    let mut buf: Vec<u8> = Vec::new();
    serialize(&mut buf, document, SerializeOpts::default())
        .expect("unable to serialize DOM into buffer");
    let result = String::from_utf8(buf).unwrap();

    let mut dom = html_to_dom(&result);
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
        base64::encode(hasher.finalize()) == integrity[7..]
    } else if integrity.starts_with("sha384-") {
        let mut hasher = Sha384::new();
        hasher.update(data);
        base64::encode(hasher.finalize()) == integrity[7..]
    } else if integrity.starts_with("sha512-") {
        let mut hasher = Sha512::new();
        hasher.update(data);
        base64::encode(hasher.finalize()) == integrity[7..]
    } else {
        false
    }
}

pub fn compose_csp(options: &Options) -> String {
    let mut string_list = vec![];

    if options.isolate {
        string_list.push("default-src 'unsafe-inline' data:;");
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
        // Note: data: is needed for transparent pixels
        string_list.push("img-src data:;");
    }

    string_list.join(" ")
}

pub fn create_metadata_tag(url: &str) -> String {
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    // Safe to unwrap (we just put this through an HTTP request)
    match Url::parse(url) {
        Ok(mut clean_url) => {
            clean_url.set_fragment(None);

            // Prevent credentials from getting into metadata
            if is_http_url(url) {
                // Only HTTP(S) URLs may feature credentials
                clean_url.set_username("").unwrap();
                clean_url.set_password(None).unwrap();
            }

            if is_http_url(url) {
                format!(
                    "<!-- Saved from {} at {} using {} v{} -->",
                    &clean_url,
                    timestamp,
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                )
            } else {
                format!(
                    "<!-- Saved from local source at {} using {} v{} -->",
                    timestamp,
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                )
            }
        }
        Err(_) => str!(),
    }
}

pub fn embed_srcset(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    parent_url: &str,
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

    let mut result: String = str!();
    let mut i: usize = array.len();
    for part in array {
        if options.no_images {
            result.push_str(empty_image!());
        } else {
            let image_full_url = resolve_url(&parent_url, part.path).unwrap_or_default();
            let image_url_fragment = get_url_fragment(image_full_url.clone());
            match retrieve_asset(
                cache,
                client,
                &parent_url,
                &image_full_url,
                options,
                depth + 1,
            ) {
                Ok((image_data, image_final_url, image_media_type)) => {
                    let image_data_url =
                        data_to_data_url(&image_media_type, &image_data, &image_final_url);
                    // Append retreved asset as a data URL
                    let assembled_url: String =
                        url_with_fragment(image_data_url.as_str(), image_url_fragment.as_str());
                    result.push_str(assembled_url.as_ref());
                }
                Err(_) => {
                    // Keep remote reference if unable to retrieve the asset
                    if is_http_url(image_full_url.clone()) {
                        let assembled_url: String =
                            url_with_fragment(image_full_url.as_str(), image_url_fragment.as_str());
                        result.push_str(assembled_url.as_ref());
                    } else {
                        // Avoid breaking the structure in case if not an HTTP(S) URL
                        result.push_str(empty_image!());
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

pub fn get_base_url(handle: &Handle) -> Option<String> {
    if let Some(base_node) = find_base_node(handle) {
        get_node_attr(&base_node, "href")
    } else {
        None
    }
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

pub fn get_node_name(node: &Handle) -> Option<&'_ str> {
    match &node.data {
        NodeData::Element { ref name, .. } => Some(name.local.as_ref()),
        _ => None,
    }
}

pub fn get_node_attr(node: &Handle, attr_name: &str) -> Option<String> {
    match &node.data {
        NodeData::Element { ref attrs, .. } => {
            for attr in attrs.borrow().iter() {
                if &*attr.name.local == attr_name {
                    return Some(str!(&*attr.value));
                }
            }
            None
        }
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

pub fn html_to_dom(data: &str) -> RcDom {
    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut data.as_bytes())
        .unwrap()
}

pub fn is_icon(attr_value: &str) -> bool {
    ICON_VALUES.contains(&attr_value.to_lowercase().as_str())
}

pub fn set_base_url(document: &Handle, desired_base_href: String) -> RcDom {
    let mut buf: Vec<u8> = Vec::new();
    serialize(&mut buf, document, SerializeOpts::default())
        .expect("unable to serialize DOM into buffer");
    let result = String::from_utf8(buf).unwrap();

    let mut dom = html_to_dom(&result);
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
                        &attrs_mut[i].value.clear();
                        &attrs_mut[i].value.push_slice(&attr_value.as_str());
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

pub fn stringify_document(handle: &Handle, options: &Options) -> String {
    let mut buf: Vec<u8> = Vec::new();
    serialize(&mut buf, handle, SerializeOpts::default())
        .expect("Unable to serialize DOM into buffer");

    let mut result = String::from_utf8(buf).unwrap();

    // We can't make it isolate the page right away since it may have no HEAD element,
    // ergo we have to serialize, parse the DOM again, insert the CSP meta tag, and then
    // finally serialize and return the resulting string
    if options.isolate
        || options.no_css
        || options.no_fonts
        || options.no_frames
        || options.no_js
        || options.no_images
    {
        // Take care of CSP
        let mut buf: Vec<u8> = Vec::new();
        let mut dom = html_to_dom(&result);
        let doc = dom.get_document();
        if let Some(html) = get_child_node_by_name(&doc, "html") {
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
                // Note: the CSP meta-tag has to be prepended, never appended,
                //       since there already may be one defined in the original document,
                //       and browsers don't allow re-defining them (for obvious reasons)
                head.children.borrow_mut().reverse();
                head.children.borrow_mut().push(meta.clone());
                head.children.borrow_mut().reverse();
            }
        }

        serialize(&mut buf, &doc, SerializeOpts::default())
            .expect("Unable to serialize DOM into buffer");
        result = String::from_utf8(buf).unwrap();
    }

    result
}

pub fn walk_and_embed_assets(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    url: &str,
    node: &Handle,
    options: &Options,
    depth: u32,
) {
    match node.data {
        NodeData::Document => {
            // Dig deeper
            for child in node.children.borrow().iter() {
                walk_and_embed_assets(cache, client, &url, child, options, depth);
            }
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            match name.local.as_ref() {
                "meta" => {
                    // Remove http-equiv attributes from META nodes if they're able to control the page
                    if let Some(meta_attr_http_equiv_value) = get_node_attr(node, "http-equiv") {
                        let meta_attr_http_equiv_value: &str = &meta_attr_http_equiv_value;
                        if meta_attr_http_equiv_value.eq_ignore_ascii_case("refresh")
                            || meta_attr_http_equiv_value.eq_ignore_ascii_case("location")
                        {
                            set_node_attr(
                                &node,
                                "http-equiv",
                                Some(format!(
                                    "disabled by monolith ({})",
                                    meta_attr_http_equiv_value
                                )),
                            );
                        }
                    }
                }
                "link" => {
                    // Read and remember integrity attribute value of this LINK node
                    let link_attr_integrity_value: Option<String> =
                        get_node_attr(node, "integrity");

                    // Remove integrity attribute from the LINK node
                    if link_attr_integrity_value != None {
                        set_node_attr(node, "integrity", None);
                    }

                    enum LinkType {
                        Icon,
                        Stylesheet,
                        Preload,
                        DnsPrefetch,
                        Unknown,
                    }

                    let mut link_type = LinkType::Unknown;
                    if let Some(link_attr_rel_value) = get_node_attr(node, "rel") {
                        if is_icon(&link_attr_rel_value) {
                            link_type = LinkType::Icon;
                        } else if link_attr_rel_value.eq_ignore_ascii_case("stylesheet") {
                            link_type = LinkType::Stylesheet;
                        } else if link_attr_rel_value.eq_ignore_ascii_case("preload") {
                            link_type = LinkType::Preload;
                        } else if link_attr_rel_value.eq_ignore_ascii_case("dns-prefetch") {
                            link_type = LinkType::DnsPrefetch;
                        }
                    }
                    // Shadow the variable (to make it non-mutable)
                    let link_type = link_type;

                    match link_type {
                        LinkType::Icon => {
                            // Find and resolve this LINK node's href attribute
                            if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                                if !options.no_images && !link_attr_href_value.is_empty() {
                                    let link_href_full_url =
                                        resolve_url(&url, link_attr_href_value).unwrap_or_default();
                                    let link_href_url_fragment =
                                        get_url_fragment(link_href_full_url.clone());
                                    match retrieve_asset(
                                        cache,
                                        client,
                                        &url,
                                        &link_href_full_url,
                                        options,
                                        depth + 1,
                                    ) {
                                        Ok((
                                            link_href_data,
                                            link_href_final_url,
                                            link_href_media_type,
                                        )) => {
                                            let mut ok_to_include = true;

                                            // Check integrity
                                            if let Some(link_attr_integrity_value) =
                                                link_attr_integrity_value
                                            {
                                                if !link_attr_integrity_value.is_empty() {
                                                    ok_to_include = check_integrity(
                                                        &link_href_data,
                                                        &link_attr_integrity_value,
                                                    );
                                                }
                                            }

                                            if ok_to_include {
                                                let link_href_data_url = data_to_data_url(
                                                    &link_href_media_type,
                                                    &link_href_data,
                                                    &link_href_final_url,
                                                );
                                                // Add new data URL href attribute
                                                let assembled_url: String = url_with_fragment(
                                                    link_href_data_url.as_str(),
                                                    link_href_url_fragment.as_str(),
                                                );
                                                set_node_attr(&node, "href", Some(assembled_url));
                                            }
                                        }
                                        Err(_) => {
                                            // Keep remote reference if unable to retrieve the asset
                                            if is_http_url(link_href_full_url.clone()) {
                                                let assembled_url: String = url_with_fragment(
                                                    link_href_full_url.as_str(),
                                                    link_href_url_fragment.as_str(),
                                                );
                                                set_node_attr(node, "href", Some(assembled_url));
                                            }
                                        }
                                    }
                                } else {
                                    set_node_attr(node, "href", None);
                                }
                            }
                        }
                        LinkType::Stylesheet => {
                            // Find and resolve this LINK node's href attribute
                            if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                                set_node_attr(node, "href", None);

                                if !options.no_css && !link_attr_href_value.is_empty() {
                                    let link_href_full_url =
                                        resolve_url(&url, link_attr_href_value).unwrap_or_default();
                                    match retrieve_asset(
                                        cache,
                                        client,
                                        &url,
                                        &link_href_full_url,
                                        options,
                                        depth + 1,
                                    ) {
                                        Ok((
                                            link_href_data,
                                            link_href_final_url,
                                            _link_href_media_type,
                                        )) => {
                                            let mut ok_to_include = true;

                                            // Check integrity
                                            if let Some(link_attr_integrity_value) =
                                                link_attr_integrity_value
                                            {
                                                if !link_attr_integrity_value.is_empty() {
                                                    ok_to_include = check_integrity(
                                                        &link_href_data,
                                                        &link_attr_integrity_value,
                                                    );
                                                }
                                            }

                                            if ok_to_include {
                                                let css: String = embed_css(
                                                    cache,
                                                    client,
                                                    &link_href_final_url,
                                                    &String::from_utf8_lossy(&link_href_data),
                                                    options,
                                                    depth + 1,
                                                );
                                                let link_href_data_url = data_to_data_url(
                                                    "text/css",
                                                    css.as_bytes(),
                                                    &link_href_final_url,
                                                );
                                                // Add new data URL href attribute
                                                set_node_attr(
                                                    &node,
                                                    "href",
                                                    Some(link_href_data_url),
                                                );
                                            }
                                        }
                                        Err(_) => {
                                            // Keep remote reference if unable to retrieve the asset
                                            if is_http_url(link_href_full_url.clone()) {
                                                set_node_attr(
                                                    &node,
                                                    "href",
                                                    Some(link_href_full_url),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        LinkType::Preload | LinkType::DnsPrefetch => {
                            // Since all resources are embedded as data URLs, preloading and prefetching are not necessary
                            set_node_attr(node, "rel", None);
                        }
                        LinkType::Unknown => {
                            // Make sure that all other LINKs' href attributes are full URLs
                            if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                                let href_full_url =
                                    resolve_url(&url, link_attr_href_value).unwrap_or_default();
                                set_node_attr(node, "href", Some(href_full_url));
                            }
                        }
                    }
                }
                "base" => {
                    if is_http_url(url) {
                        // Ensure the BASE node doesn't have a relative URL
                        if let Some(base_attr_href_value) = get_node_attr(node, "href") {
                            let href_full_url =
                                resolve_url(&url, base_attr_href_value).unwrap_or_default();
                            set_node_attr(node, "href", Some(href_full_url));
                        }
                    }
                }
                "body" => {
                    // Read and remember background attribute value of this BODY node
                    if let Some(body_attr_background_value) = get_node_attr(node, "background") {
                        // Remove background BODY node attribute by default
                        set_node_attr(node, "background", None);

                        if !options.no_images && !body_attr_background_value.is_empty() {
                            let background_full_url =
                                resolve_url(&url, body_attr_background_value).unwrap_or_default();
                            let background_url_fragment =
                                get_url_fragment(background_full_url.clone());
                            match retrieve_asset(
                                cache,
                                client,
                                &url,
                                &background_full_url,
                                options,
                                depth + 1,
                            ) {
                                Ok((
                                    background_data,
                                    background_final_url,
                                    background_media_type,
                                )) => {
                                    let background_data_url = data_to_data_url(
                                        &background_media_type,
                                        &background_data,
                                        &background_final_url,
                                    );
                                    // Convert background attribute to data URL
                                    let assembled_url: String = url_with_fragment(
                                        background_data_url.as_str(),
                                        background_url_fragment.as_str(),
                                    );
                                    set_node_attr(node, "background", Some(assembled_url));
                                }
                                Err(_) => {
                                    // Keep remote reference if unable to retrieve the asset
                                    if is_http_url(background_full_url.clone()) {
                                        let assembled_url: String = url_with_fragment(
                                            background_full_url.as_str(),
                                            background_url_fragment.as_str(),
                                        );
                                        set_node_attr(node, "background", Some(assembled_url));
                                    }
                                }
                            }
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
                            set_node_attr(node, "src", Some(str!(empty_image!())));
                        }
                        if img_attr_data_src_value != None {
                            set_node_attr(node, "data-src", Some(str!(empty_image!())));
                        }
                    } else {
                        if img_attr_src_value.clone().unwrap_or_default().is_empty()
                            && img_attr_data_src_value
                                .clone()
                                .unwrap_or_default()
                                .is_empty()
                        {
                            // Add empty src attribute
                            set_node_attr(node, "src", Some(str!()));
                        } else {
                            // Add data URL src attribute
                            let img_full_url = resolve_url(
                                &url,
                                if !img_attr_data_src_value
                                    .clone()
                                    .unwrap_or_default()
                                    .is_empty()
                                {
                                    img_attr_data_src_value.unwrap_or_default()
                                } else {
                                    img_attr_src_value.unwrap_or_default()
                                },
                            )
                            .unwrap_or_default();
                            let img_url_fragment = get_url_fragment(img_full_url.clone());

                            match retrieve_asset(
                                cache,
                                client,
                                &url,
                                &img_full_url,
                                options,
                                depth + 1,
                            ) {
                                Ok((img_data, img_final_url, img_media_type)) => {
                                    let img_data_url = data_to_data_url(
                                        &img_media_type,
                                        &img_data,
                                        &img_final_url,
                                    );
                                    let assembled_url: String = url_with_fragment(
                                        img_data_url.as_str(),
                                        img_url_fragment.as_str(),
                                    );
                                    set_node_attr(node, "src", Some(assembled_url));
                                }
                                Err(_) => {
                                    if is_http_url(img_full_url.clone()) {
                                        // Keep remote reference if unable to retrieve the asset
                                        let assembled_url: String = url_with_fragment(
                                            img_full_url.as_str(),
                                            img_url_fragment.as_str(),
                                        );
                                        set_node_attr(node, "src", Some(assembled_url));
                                    } else {
                                        // Don't keep original reference if it's not a remote target
                                        set_node_attr(node, "src", None);
                                    }
                                }
                            }
                        }
                    }

                    // Resolve srcset attribute
                    if let Some(img_srcset) = get_node_attr(node, "srcset") {
                        if !img_srcset.is_empty() {
                            let resolved_srcset: String =
                                embed_srcset(cache, client, &url, &img_srcset, options, depth);
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
                                        str!()
                                    } else {
                                        str!(empty_image!())
                                    };
                                    set_node_attr(node, "src", Some(value));
                                } else {
                                    let input_image_full_url =
                                        resolve_url(&url, input_attr_src_value).unwrap_or_default();
                                    let input_image_url_fragment =
                                        get_url_fragment(input_image_full_url.clone());
                                    match retrieve_asset(
                                        cache,
                                        client,
                                        &url,
                                        &input_image_full_url,
                                        options,
                                        depth + 1,
                                    ) {
                                        Ok((
                                            input_image_data,
                                            input_image_final_url,
                                            input_image_media_type,
                                        )) => {
                                            let input_image_data_url = data_to_data_url(
                                                &input_image_media_type,
                                                &input_image_data,
                                                &input_image_final_url,
                                            );
                                            // Add data URL src attribute
                                            let assembled_url: String = url_with_fragment(
                                                input_image_data_url.as_str(),
                                                input_image_url_fragment.as_str(),
                                            );
                                            set_node_attr(node, "src", Some(assembled_url));
                                        }
                                        Err(_) => {
                                            // Keep remote reference if unable to retrieve the asset
                                            if is_http_url(input_image_full_url.clone()) {
                                                let assembled_url: String = url_with_fragment(
                                                    input_image_full_url.as_str(),
                                                    input_image_url_fragment.as_str(),
                                                );
                                                set_node_attr(node, "src", Some(assembled_url));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "image" => {
                    let mut image_href: String = str!();

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
                        let image_full_url = resolve_url(&url, image_href).unwrap_or_default();
                        let image_url_fragment = get_url_fragment(image_full_url.clone());
                        match retrieve_asset(
                            cache,
                            client,
                            &url,
                            &image_full_url,
                            options,
                            depth + 1,
                        ) {
                            Ok((image_data, image_final_url, image_media_type)) => {
                                let image_data_url = data_to_data_url(
                                    &image_media_type,
                                    &image_data,
                                    &image_final_url,
                                );
                                // Add new data URL href attribute
                                let assembled_url: String = url_with_fragment(
                                    image_data_url.as_str(),
                                    image_url_fragment.as_str(),
                                );
                                set_node_attr(node, "href", Some(assembled_url));
                            }
                            Err(_) => {
                                // Keep remote reference if unable to retrieve the asset
                                if is_http_url(image_full_url.clone()) {
                                    let assembled_url: String = url_with_fragment(
                                        image_full_url.as_str(),
                                        image_url_fragment.as_str(),
                                    );
                                    set_node_attr(node, "href", Some(assembled_url));
                                }
                            }
                        }
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
                                let src_full_url: String =
                                    resolve_url(&url, source_attr_src_value.clone())
                                        .unwrap_or_else(|_| source_attr_src_value.to_string());
                                let src_url_fragment = get_url_fragment(src_full_url.clone());
                                match retrieve_asset(
                                    cache,
                                    client,
                                    &url,
                                    &src_full_url,
                                    options,
                                    depth + 1,
                                ) {
                                    Ok((src_data, src_final_url, src_media_type)) => {
                                        let src_data_url = data_to_data_url(
                                            &src_media_type,
                                            &src_data,
                                            &src_final_url,
                                        );
                                        let assembled_url: String = url_with_fragment(
                                            src_data_url.as_str(),
                                            src_url_fragment.as_str(),
                                        );
                                        set_node_attr(node, "src", Some(assembled_url));
                                    }
                                    Err(_) => {
                                        if is_http_url(src_full_url.clone()) {
                                            // Keep remote reference if unable to retrieve the asset
                                            let assembled_url: String = url_with_fragment(
                                                src_full_url.as_str(),
                                                src_url_fragment.as_str(),
                                            );
                                            set_node_attr(node, "src", Some(assembled_url));
                                        } else {
                                            // Exclude non-remote URLs
                                            set_node_attr(node, "src", None);
                                        }
                                    }
                                }
                            }
                        } else if parent_node_name == "video" {
                            if options.no_video {
                                set_node_attr(node, "src", None);
                            } else {
                                let src_full_url: String =
                                    resolve_url(&url, source_attr_src_value.clone())
                                        .unwrap_or_else(|_| source_attr_src_value.to_string());
                                let src_url_fragment = get_url_fragment(src_full_url.clone());
                                match retrieve_asset(
                                    cache,
                                    client,
                                    &url,
                                    &src_full_url,
                                    options,
                                    depth + 1,
                                ) {
                                    Ok((src_data, src_final_url, src_media_type)) => {
                                        let src_data_url = data_to_data_url(
                                            &src_media_type,
                                            &src_data,
                                            &src_final_url,
                                        );
                                        let assembled_url: String = url_with_fragment(
                                            src_data_url.as_str(),
                                            src_url_fragment.as_str(),
                                        );
                                        set_node_attr(node, "src", Some(assembled_url));
                                    }
                                    Err(_) => {
                                        if is_http_url(src_full_url.clone()) {
                                            // Keep remote reference if unable to retrieve the asset
                                            let assembled_url: String = url_with_fragment(
                                                src_full_url.as_str(),
                                                src_url_fragment.as_str(),
                                            );
                                            set_node_attr(node, "src", Some(assembled_url));
                                        } else {
                                            // Exclude non-remote URLs
                                            set_node_attr(node, "src", None);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(source_attr_srcset_value) = get_node_attr(node, "srcset") {
                        if parent_node_name == "picture" {
                            if !source_attr_srcset_value.is_empty() {
                                if options.no_images {
                                    set_node_attr(node, "srcset", Some(str!(empty_image!())));
                                } else {
                                    let resolved_srcset: String = embed_srcset(
                                        cache,
                                        client,
                                        &url,
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
                        if options.no_js
                            && anchor_attr_href_value
                                .clone()
                                .trim()
                                .starts_with("javascript:")
                        {
                            // Replace with empty JS call to preserve original behavior
                            set_node_attr(node, "href", Some(str!("javascript:;")));
                        } else if anchor_attr_href_value.clone().starts_with('#')
                            || url_has_protocol(anchor_attr_href_value.clone())
                        {
                            // Don't touch email links or hrefs which begin with a hash
                        } else {
                            let href_full_url =
                                resolve_url(&url, anchor_attr_href_value).unwrap_or_default();
                            set_node_attr(node, "href", Some(href_full_url));
                        }
                    }
                }
                "script" => {
                    // Read values of integrity and src attributes
                    let script_attr_integrity: Option<String> = get_node_attr(node, "integrity");
                    let script_attr_src: Option<String> = get_node_attr(node, "src");

                    // Wipe integrity attribute
                    if script_attr_integrity != None {
                        set_node_attr(node, "integrity", None);
                    }

                    if options.no_js {
                        // Empty inner content
                        node.children.borrow_mut().clear();
                        // Remove src attribute
                        if script_attr_src != None {
                            set_node_attr(node, "src", None);
                        }
                    } else if !script_attr_src.clone().unwrap_or_default().is_empty() {
                        let script_full_url =
                            resolve_url(&url, script_attr_src.unwrap_or_default())
                                .unwrap_or_default();
                        match retrieve_asset(
                            cache,
                            client,
                            &url,
                            &script_full_url,
                            options,
                            depth + 1,
                        ) {
                            Ok((script_data, script_final_url, _script_media_type)) => {
                                let mut ok_to_include = true;

                                // Check integrity
                                if let Some(script_attr_integrity_value) = script_attr_integrity {
                                    if !script_attr_integrity_value.is_empty() {
                                        ok_to_include = check_integrity(
                                            &script_data,
                                            &script_attr_integrity_value,
                                        );
                                    }
                                }

                                if ok_to_include {
                                    // Only embed if we're able to validate integrity
                                    let script_data_url = data_to_data_url(
                                        "application/javascript",
                                        &script_data,
                                        &script_final_url,
                                    );
                                    set_node_attr(node, "src", Some(script_data_url));
                                } else {
                                    set_node_attr(node, "src", None);
                                }
                            }
                            Err(_) => {
                                if is_http_url(script_full_url.clone()) {
                                    // Keep remote reference if unable to retrieve the asset
                                    set_node_attr(node, "src", Some(script_full_url));
                                } else {
                                    // Remove src attribute if target is not remote
                                    set_node_attr(node, "src", None);
                                }
                            }
                        };
                    }
                }
                "style" => {
                    if options.no_css {
                        // Empty inner content of STYLE tags
                        node.children.borrow_mut().clear();
                    } else {
                        for node in node.children.borrow_mut().iter_mut() {
                            if let NodeData::Text { ref contents } = node.data {
                                let mut tendril = contents.borrow_mut();
                                let replacement = embed_css(
                                    cache,
                                    client,
                                    &url,
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
                        if !is_http_url(form_attr_action_value.clone()) {
                            let form_action_full_url =
                                resolve_url(&url, form_attr_action_value).unwrap_or_default();
                            set_node_attr(node, "action", Some(form_action_full_url));
                        }
                    }
                }
                "frame" | "iframe" => {
                    if let Some(frame_attr_src_value) = get_node_attr(node, "src") {
                        if options.no_frames {
                            // Empty the src attribute
                            set_node_attr(node, "src", Some(str!()));
                        } else {
                            let frame_src = frame_attr_src_value.trim();

                            // Ignore (i)frames with empty source (they cause infinite loops)
                            if !frame_src.is_empty() {
                                let frame_full_url =
                                    resolve_url(&url, frame_src).unwrap_or_default();
                                let frame_url_fragment = get_url_fragment(frame_full_url.clone());
                                match retrieve_asset(
                                    cache,
                                    client,
                                    &url,
                                    &frame_full_url,
                                    options,
                                    depth + 1,
                                ) {
                                    Ok((frame_data, frame_final_url, frame_media_type)) => {
                                        let frame_dom =
                                            html_to_dom(&String::from_utf8_lossy(&frame_data));
                                        walk_and_embed_assets(
                                            cache,
                                            client,
                                            &frame_final_url,
                                            &frame_dom.document,
                                            &options,
                                            depth + 1,
                                        );
                                        let mut frame_data: Vec<u8> = Vec::new();
                                        serialize(
                                            &mut frame_data,
                                            &frame_dom.document,
                                            SerializeOpts::default(),
                                        )
                                        .unwrap();
                                        let frame_data_url = data_to_data_url(
                                            &frame_media_type,
                                            &frame_data,
                                            &frame_final_url,
                                        );
                                        let assembled_url: String = url_with_fragment(
                                            frame_data_url.as_str(),
                                            frame_url_fragment.as_str(),
                                        );
                                        set_node_attr(node, "src", Some(assembled_url));
                                    }
                                    Err(_) => {
                                        // Keep remote reference if unable to retrieve the asset
                                        if is_http_url(frame_full_url.clone()) {
                                            let assembled_url: String = url_with_fragment(
                                                frame_full_url.as_str(),
                                                frame_url_fragment.as_str(),
                                            );
                                            set_node_attr(node, "src", Some(assembled_url));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "audio" => {
                    if let Some(audio_attr_src_value) = get_node_attr(node, "src") {
                        if options.no_audio {
                            set_node_attr(node, "src", None);
                        } else {
                            let src_full_url: String =
                                resolve_url(&url, audio_attr_src_value.clone())
                                    .unwrap_or_else(|_| audio_attr_src_value.to_string());
                            let src_url_fragment = get_url_fragment(src_full_url.clone());
                            match retrieve_asset(
                                cache,
                                client,
                                &url,
                                &src_full_url,
                                options,
                                depth + 1,
                            ) {
                                Ok((src_data, src_final_url, src_media_type)) => {
                                    let src_data_url = data_to_data_url(
                                        &src_media_type,
                                        &src_data,
                                        &src_final_url,
                                    );
                                    let assembled_url: String = url_with_fragment(
                                        src_data_url.as_str(),
                                        src_url_fragment.as_str(),
                                    );
                                    set_node_attr(node, "src", Some(assembled_url));
                                }
                                Err(_) => {
                                    if is_http_url(src_full_url.clone()) {
                                        // Keep remote reference if unable to retrieve the asset
                                        let assembled_url: String = url_with_fragment(
                                            src_full_url.as_str(),
                                            src_url_fragment.as_str(),
                                        );
                                        set_node_attr(node, "src", Some(assembled_url));
                                    } else {
                                        // Exclude non-remote URLs
                                        set_node_attr(node, "src", None);
                                    }
                                }
                            }
                        }
                    }
                }
                "video" => {
                    if let Some(video_attr_src_value) = get_node_attr(node, "src") {
                        if options.no_video {
                            set_node_attr(node, "src", None);
                        } else {
                            let src_full_url: String =
                                resolve_url(&url, video_attr_src_value.clone())
                                    .unwrap_or_else(|_| video_attr_src_value.to_string());
                            let src_url_fragment = get_url_fragment(src_full_url.clone());
                            match retrieve_asset(
                                cache,
                                client,
                                &url,
                                &src_full_url,
                                options,
                                depth + 1,
                            ) {
                                Ok((src_data, src_final_url, src_media_type)) => {
                                    let src_data_url = data_to_data_url(
                                        &src_media_type,
                                        &src_data,
                                        &src_final_url,
                                    );
                                    let assembled_url: String = url_with_fragment(
                                        src_data_url.as_str(),
                                        src_url_fragment.as_str(),
                                    );
                                    set_node_attr(node, "src", Some(assembled_url));
                                }
                                Err(_) => {
                                    if is_http_url(src_full_url.clone()) {
                                        // Keep remote reference if unable to retrieve the asset
                                        let assembled_url: String = url_with_fragment(
                                            src_full_url.as_str(),
                                            src_url_fragment.as_str(),
                                        );
                                        set_node_attr(node, "src", Some(assembled_url));
                                    } else {
                                        // Exclude non-remote URLs
                                        set_node_attr(node, "src", None);
                                    }
                                }
                            }
                        }
                    }

                    // Embed poster images
                    if let Some(video_attr_poster_value) = get_node_attr(node, "poster") {
                        // Skip posters with empty source
                        if !video_attr_poster_value.is_empty() {
                            if options.no_images {
                                set_node_attr(node, "poster", Some(str!(empty_image!())));
                            } else {
                                let video_poster_full_url =
                                    resolve_url(&url, video_attr_poster_value).unwrap_or_default();
                                let video_poster_url_fragment =
                                    get_url_fragment(video_poster_full_url.clone());
                                match retrieve_asset(
                                    cache,
                                    client,
                                    &url,
                                    &video_poster_full_url,
                                    options,
                                    depth + 1,
                                ) {
                                    Ok((
                                        video_poster_data,
                                        video_poster_final_url,
                                        video_poster_media_type,
                                    )) => {
                                        let video_poster_data_url = data_to_data_url(
                                            &video_poster_media_type,
                                            &video_poster_data,
                                            &video_poster_final_url,
                                        );
                                        let assembled_url: String = url_with_fragment(
                                            video_poster_data_url.as_str(),
                                            video_poster_url_fragment.as_str(),
                                        );
                                        set_node_attr(node, "poster", Some(assembled_url));
                                    }
                                    Err(_) => {
                                        if is_http_url(video_poster_full_url.clone()) {
                                            // Keep remote reference if unable to retrieve the asset
                                            let assembled_url: String = url_with_fragment(
                                                video_poster_full_url.as_str(),
                                                video_poster_url_fragment.as_str(),
                                            );
                                            set_node_attr(node, "poster", Some(assembled_url));
                                        } else {
                                            // Get rid of poster attribute if the URL is not remote
                                            set_node_attr(node, "poster", None);
                                        }
                                    }
                                }
                            }
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
                    let embedded_style =
                        embed_css(cache, client, &url, &node_attr_style_value, options, depth);
                    set_node_attr(node, "style", Some(embedded_style));
                }
            }

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
                walk_and_embed_assets(cache, client, &url, child, options, depth);
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
