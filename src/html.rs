use base64;
use chrono::prelude::*;
use html5ever::interface::QualName;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::serialize::{serialize, SerializeOpts};
use html5ever::tendril::{format_tendril, Tendril, TendrilSink};
use html5ever::tree_builder::{Attribute, TreeSink};
use html5ever::{local_name, namespace_url, ns};
use reqwest::blocking::Client;
use reqwest::Url;
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::collections::HashMap;
use std::default::Default;

use crate::css::embed_css;
use crate::js::attr_is_event_handler;
use crate::url::{
    data_to_data_url, get_url_fragment, is_http_url, resolve_url, url_has_protocol,
    url_with_fragment,
};
use crate::utils::retrieve_asset;

struct SrcSetItem<'a> {
    path: &'a str,
    descriptor: &'a str,
}

const ICON_VALUES: &[&str] = &[
    "icon",
    "shortcut icon",
    "mask-icon",
    "apple-touch-icon",
    "fluid-icon",
];

pub fn get_parent_node(node: &Handle) -> Handle {
    let parent = node.parent.take().clone();
    parent.and_then(|node| node.upgrade()).unwrap()
}

pub fn get_node_name(node: &Handle) -> Option<&'_ str> {
    match &node.data {
        NodeData::Element { ref name, .. } => Some(name.local.as_ref()),
        _ => None,
    }
}

pub fn is_icon(attr_value: &str) -> bool {
    ICON_VALUES.contains(&attr_value.to_lowercase().as_str())
}

pub fn has_proper_integrity(data: &[u8], integrity: &str) -> bool {
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

pub fn embed_srcset(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    parent_url: &str,
    srcset: &str,
    opt_no_images: bool,
    opt_silent: bool,
) -> String {
    let mut array: Vec<SrcSetItem> = vec![];
    let srcset_items: Vec<&str> = srcset.split(',').collect();
    for srcset_item in srcset_items {
        let parts: Vec<&str> = srcset_item.trim().split_whitespace().collect();
        let path = parts[0].trim();
        let descriptor = if parts.len() > 1 { parts[1].trim() } else { "" };
        let srcset_real_item = SrcSetItem { path, descriptor };
        array.push(srcset_real_item);
    }

    let mut result: String = str!();
    let mut i: usize = array.len();
    for part in array {
        if opt_no_images {
            result.push_str(empty_image!());
        } else {
            let image_full_url = resolve_url(&parent_url, part.path).unwrap_or_default();
            let image_url_fragment = get_url_fragment(image_full_url.clone());
            match retrieve_asset(cache, client, &parent_url, &image_full_url, opt_silent) {
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

pub fn walk_and_embed_assets(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    url: &str,
    node: &Handle,
    opt_no_css: bool,
    opt_no_fonts: bool,
    opt_no_frames: bool,
    opt_no_js: bool,
    opt_no_images: bool,
    opt_silent: bool,
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
                    opt_no_fonts,
                    opt_no_frames,
                    opt_no_js,
                    opt_no_images,
                    opt_silent,
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
                "meta" => {
                    // Determine type
                    let mut is_unwanted_meta: bool = false;
                    for attr in attrs_mut.iter_mut() {
                        let attr_name: &str = &attr.name.local;
                        if attr_name.eq_ignore_ascii_case("http-equiv") {
                            let value: String = attr.value.to_string();
                            is_unwanted_meta = value.eq_ignore_ascii_case("refresh")
                                || value.eq_ignore_ascii_case("location");
                        }
                    }

                    if is_unwanted_meta {
                        // Strip this node off all its attributes
                        while attrs_mut.len() > 0 {
                            attrs_mut.remove(0);
                        }
                    }
                }
                "link" => {
                    // Remove integrity attributes, keep value of the last one
                    let mut integrity: String = str!();
                    let mut i = 0;
                    while i < attrs_mut.len() {
                        let attr_name: &str = &attrs_mut[i].name.local;
                        if attr_name.eq_ignore_ascii_case("integrity") {
                            integrity = str!(attrs_mut.remove(i).value.trim());
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
                            // Find and remove href attribute(s), keep value of the last found one
                            let mut link_href: String = str!();
                            let mut i = 0;
                            while i < attrs_mut.len() {
                                let attr_name: &str = &attrs_mut[i].name.local;
                                if attr_name.eq_ignore_ascii_case("href") {
                                    link_href = str!(attrs_mut.remove(i).value.trim());
                                } else {
                                    i += 1;
                                }
                            }

                            if !opt_no_images && !link_href.is_empty() {
                                let link_href_full_url =
                                    resolve_url(&url, link_href).unwrap_or_default();
                                let link_href_url_fragment =
                                    get_url_fragment(link_href_full_url.clone());
                                match retrieve_asset(
                                    cache,
                                    client,
                                    &url,
                                    &link_href_full_url,
                                    opt_silent,
                                ) {
                                    Ok((
                                        link_href_data,
                                        link_href_final_url,
                                        link_href_media_type,
                                    )) => {
                                        // Check integrity
                                        if integrity.is_empty()
                                            || has_proper_integrity(&link_href_data, &integrity)
                                        {
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
                                            attrs_mut.push(Attribute {
                                                name: QualName::new(
                                                    None,
                                                    ns!(),
                                                    local_name!("href"),
                                                ),
                                                value: Tendril::from_slice(assembled_url.as_ref()),
                                            });
                                        }
                                    }
                                    Err(_) => {
                                        // Keep remote reference if unable to retrieve the asset
                                        if is_http_url(link_href_full_url.clone()) {
                                            let assembled_url: String = url_with_fragment(
                                                link_href_full_url.as_str(),
                                                link_href_url_fragment.as_str(),
                                            );
                                            attrs_mut.push(Attribute {
                                                name: QualName::new(
                                                    None,
                                                    ns!(),
                                                    local_name!("href"),
                                                ),
                                                value: Tendril::from_slice(assembled_url.as_ref()),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        LinkType::Stylesheet => {
                            // Find and remove href attribute(s), keep value of the last found one
                            let mut link_href: String = str!();
                            let mut i = 0;
                            while i < attrs_mut.len() {
                                let attr_name: &str = &attrs_mut[i].name.local;
                                if attr_name.eq_ignore_ascii_case("href") {
                                    link_href = str!(attrs_mut.remove(i).value.trim());
                                } else {
                                    i += 1;
                                }
                            }

                            if !opt_no_css && !link_href.is_empty() {
                                let link_href_full_url =
                                    resolve_url(&url, link_href).unwrap_or_default();
                                match retrieve_asset(
                                    cache,
                                    client,
                                    &url,
                                    &link_href_full_url,
                                    opt_silent,
                                ) {
                                    Ok((
                                        link_href_data,
                                        link_href_final_url,
                                        _link_href_media_type,
                                    )) => {
                                        // Check integrity
                                        if integrity.is_empty()
                                            || has_proper_integrity(&link_href_data, &integrity)
                                        {
                                            let css: String = embed_css(
                                                cache,
                                                client,
                                                &link_href_final_url,
                                                &String::from_utf8_lossy(&link_href_data),
                                                opt_no_fonts,
                                                opt_no_images,
                                                opt_silent,
                                            );
                                            let link_href_data_url = data_to_data_url(
                                                "text/css",
                                                css.as_bytes(),
                                                &link_href_final_url,
                                            );
                                            // Add new data URL href attribute
                                            attrs_mut.push(Attribute {
                                                name: QualName::new(
                                                    None,
                                                    ns!(),
                                                    local_name!("href"),
                                                ),
                                                value: Tendril::from_slice(
                                                    link_href_data_url.as_ref(),
                                                ),
                                            });
                                        }
                                    }
                                    Err(_) => {
                                        // Keep remote reference if unable to retrieve the asset
                                        if is_http_url(link_href_full_url.clone()) {
                                            attrs_mut.push(Attribute {
                                                name: QualName::new(
                                                    None,
                                                    ns!(),
                                                    local_name!("href"),
                                                ),
                                                value: Tendril::from_slice(
                                                    link_href_full_url.as_ref(),
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        LinkType::Preload | LinkType::DnsPrefetch => {
                            // Since all resources are embedded as data URL, preloading and prefetching are unnecessary
                            for _ in 0..attrs_mut.len() {
                                attrs_mut.remove(0);
                            }
                        }
                        LinkType::Unknown => {
                            for attr in attrs_mut.iter_mut() {
                                let attr_name: &str = &attr.name.local;
                                if attr_name.eq_ignore_ascii_case("href") {
                                    let href_full_url =
                                        resolve_url(&url, attr.value.trim()).unwrap_or_default();
                                    attr.value.clear();
                                    attr.value.push_slice(&href_full_url.as_str());
                                }
                            }
                        }
                    }
                }
                "body" => {
                    // Find and remove background attribute(s), keep value of the last found one
                    let mut background: String = str!();
                    let mut i = 0;
                    while i < attrs_mut.len() {
                        let attr_name: &str = &attrs_mut[i].name.local;
                        if attr_name.eq_ignore_ascii_case("background") {
                            background = str!(attrs_mut.remove(i).value.trim());
                        } else {
                            i += 1;
                        }
                    }

                    if !opt_no_images && !background.is_empty() {
                        let background_full_url = resolve_url(&url, background).unwrap_or_default();
                        let background_url_fragment = get_url_fragment(background_full_url.clone());
                        match retrieve_asset(cache, client, &url, &background_full_url, opt_silent)
                        {
                            Ok((background_data, background_final_url, background_media_type)) => {
                                let background_data_url = data_to_data_url(
                                    &background_media_type,
                                    &background_data,
                                    &background_final_url,
                                );
                                // Add new data URL background attribute
                                let assembled_url: String = url_with_fragment(
                                    background_data_url.as_str(),
                                    background_url_fragment.as_str(),
                                );
                                attrs_mut.push(Attribute {
                                    name: QualName::new(None, ns!(), local_name!("background")),
                                    value: Tendril::from_slice(assembled_url.as_ref()),
                                });
                            }
                            Err(_) => {
                                // Keep remote reference if unable to retrieve the asset
                                if is_http_url(background_full_url.clone()) {
                                    let assembled_url: String = url_with_fragment(
                                        background_full_url.as_str(),
                                        background_url_fragment.as_str(),
                                    );
                                    attrs_mut.push(Attribute {
                                        name: QualName::new(None, ns!(), local_name!("background")),
                                        value: Tendril::from_slice(assembled_url.as_ref()),
                                    });
                                }
                            }
                        }
                    }
                }
                "img" => {
                    // Find source attribute(s)
                    let mut img_data_src: String = str!();
                    let mut img_src: String = str!();
                    let mut img_srcset: String = str!();
                    let mut i = 0;
                    while i < attrs_mut.len() {
                        let attr_name: &str = &attrs_mut[i].name.local;
                        if attr_name.eq_ignore_ascii_case("data-src") {
                            img_data_src = str!(attrs_mut.remove(i).value.trim());
                        } else if attr_name.eq_ignore_ascii_case("src") {
                            img_src = str!(attrs_mut.remove(i).value.trim());
                        } else if attr_name.eq_ignore_ascii_case("srcset") {
                            img_srcset = str!(attrs_mut.remove(i).value.trim());
                        } else {
                            i += 1;
                        }
                    }

                    if opt_no_images {
                        // Add empty image src attribute
                        attrs_mut.push(Attribute {
                            name: QualName::new(None, ns!(), local_name!("src")),
                            value: Tendril::from_slice(empty_image!()),
                        });
                    } else {
                        if img_src.is_empty() && img_data_src.is_empty() {
                            // Add empty src attribute
                            attrs_mut.push(Attribute {
                                name: QualName::new(None, ns!(), local_name!("src")),
                                value: Tendril::from_slice(""),
                            });
                        } else {
                            // Add data URL src attribute
                            let img_full_url = resolve_url(
                                &url,
                                if !img_data_src.is_empty() {
                                    img_data_src
                                } else {
                                    img_src
                                },
                            )
                            .unwrap_or_default();
                            let img_url_fragment = get_url_fragment(img_full_url.clone());
                            match retrieve_asset(cache, client, &url, &img_full_url, opt_silent) {
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
                                    attrs_mut.push(Attribute {
                                        name: QualName::new(None, ns!(), local_name!("src")),
                                        value: Tendril::from_slice(assembled_url.as_ref()),
                                    });
                                }
                                Err(_) => {
                                    // Keep remote reference if unable to retrieve the asset
                                    if is_http_url(img_full_url.clone()) {
                                        let assembled_url: String = url_with_fragment(
                                            img_full_url.as_str(),
                                            img_url_fragment.as_str(),
                                        );
                                        attrs_mut.push(Attribute {
                                            name: QualName::new(None, ns!(), local_name!("src")),
                                            value: Tendril::from_slice(assembled_url.as_ref()),
                                        });
                                    }
                                }
                            }
                        }
                    }

                    if !img_srcset.is_empty() {
                        attrs_mut.push(Attribute {
                            name: QualName::new(None, ns!(), local_name!("srcset")),
                            value: Tendril::from_slice(
                                embed_srcset(
                                    cache,
                                    client,
                                    &url,
                                    &img_srcset,
                                    opt_no_images,
                                    opt_silent,
                                )
                                .as_ref(),
                            ),
                        });
                    }
                }
                "svg" => {
                    if opt_no_images {
                        node.children.borrow_mut().clear();
                    }
                }
                "input" => {
                    // Determine input type
                    let mut is_image_input: bool = false;
                    for attr in attrs_mut.iter_mut() {
                        let attr_name: &str = &attr.name.local;
                        if attr_name.eq_ignore_ascii_case("type") {
                            is_image_input = attr.value.to_string().eq_ignore_ascii_case("image");
                        }
                    }

                    if is_image_input {
                        let mut input_image_src: String = str!();
                        let mut i = 0;
                        while i < attrs_mut.len() {
                            let attr_name: &str = &attrs_mut[i].name.local;
                            if attr_name.eq_ignore_ascii_case("src") {
                                input_image_src = str!(attrs_mut.remove(i).value.trim());
                            } else {
                                i += 1;
                            }
                        }

                        if opt_no_images || input_image_src.is_empty() {
                            attrs_mut.push(Attribute {
                                name: QualName::new(None, ns!(), local_name!("src")),
                                value: Tendril::from_slice(if input_image_src.is_empty() {
                                    ""
                                } else {
                                    empty_image!()
                                }),
                            });
                        } else {
                            let input_image_full_url =
                                resolve_url(&url, input_image_src).unwrap_or_default();
                            let input_image_url_fragment =
                                get_url_fragment(input_image_full_url.clone());
                            match retrieve_asset(
                                cache,
                                client,
                                &url,
                                &input_image_full_url,
                                opt_silent,
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
                                    attrs_mut.push(Attribute {
                                        name: QualName::new(None, ns!(), local_name!("src")),
                                        value: Tendril::from_slice(assembled_url.as_ref()),
                                    });
                                }
                                Err(_) => {
                                    // Keep remote reference if unable to retrieve the asset
                                    if is_http_url(input_image_full_url.clone()) {
                                        let assembled_url: String = url_with_fragment(
                                            input_image_full_url.as_str(),
                                            input_image_url_fragment.as_str(),
                                        );
                                        attrs_mut.push(Attribute {
                                            name: QualName::new(None, ns!(), local_name!("src")),
                                            value: Tendril::from_slice(assembled_url.as_ref()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                "image" => {
                    // Find and remove (xlink:)href attribute(s), keep value of the last one
                    let mut image_href: String = str!();
                    let mut i = 0;
                    while i < attrs_mut.len() {
                        let attr_name: &str = &attrs_mut[i].name.local;
                        if attr_name.eq_ignore_ascii_case("xlink:href")
                            || attr_name.eq_ignore_ascii_case("href")
                        {
                            image_href = str!(attrs_mut.remove(i).value.trim());
                        } else {
                            i += 1;
                        }
                    }

                    if !opt_no_images && !image_href.is_empty() {
                        let image_full_url = resolve_url(&url, image_href).unwrap_or_default();
                        let image_url_fragment = get_url_fragment(image_full_url.clone());
                        match retrieve_asset(cache, client, &url, &image_full_url, opt_silent) {
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
                                attrs_mut.push(Attribute {
                                    name: QualName::new(None, ns!(), local_name!("href")),
                                    value: Tendril::from_slice(assembled_url.as_ref()),
                                });
                            }
                            Err(_) => {
                                // Keep remote reference if unable to retrieve the asset
                                if is_http_url(image_full_url.clone()) {
                                    let assembled_url: String = url_with_fragment(
                                        image_full_url.as_str(),
                                        image_url_fragment.as_str(),
                                    );
                                    attrs_mut.push(Attribute {
                                        name: QualName::new(None, ns!(), local_name!("href")),
                                        value: Tendril::from_slice(assembled_url.as_ref()),
                                    });
                                }
                            }
                        }
                    }
                }
                "source" => {
                    for attr in attrs_mut.iter_mut() {
                        let attr_name: &str = &attr.name.local;

                        if attr_name.eq_ignore_ascii_case("src") {
                            let src_full_url = resolve_url(&url, attr.value.trim())
                                .unwrap_or_else(|_| attr.value.to_string());
                            attr.value.clear();
                            attr.value.push_slice(src_full_url.as_str());
                        } else if attr_name.eq_ignore_ascii_case("srcset") {
                            if get_node_name(&get_parent_node(&node)) == Some("picture") {
                                if opt_no_images {
                                    attr.value.clear();
                                    attr.value.push_slice(empty_image!());
                                } else {
                                    let srcset_full_url =
                                        resolve_url(&url, attr.value.trim()).unwrap_or_default();
                                    let srcset_url_fragment =
                                        get_url_fragment(srcset_full_url.clone());
                                    match retrieve_asset(
                                        cache,
                                        client,
                                        &url,
                                        &srcset_full_url,
                                        opt_silent,
                                    ) {
                                        Ok((srcset_data, srcset_final_url, srcset_media_type)) => {
                                            let srcset_data_url = data_to_data_url(
                                                &srcset_media_type,
                                                &srcset_data,
                                                &srcset_final_url,
                                            );
                                            attr.value.clear();
                                            let assembled_url: String = url_with_fragment(
                                                srcset_data_url.as_str(),
                                                srcset_url_fragment.as_str(),
                                            );
                                            attr.value.push_slice(assembled_url.as_str());
                                        }
                                        Err(_) => {
                                            // Keep remote reference if unable to retrieve the asset
                                            if is_http_url(srcset_full_url.clone()) {
                                                attr.value.clear();
                                                let assembled_url: String = url_with_fragment(
                                                    srcset_full_url.as_str(),
                                                    srcset_url_fragment.as_str(),
                                                );
                                                attr.value.push_slice(assembled_url.as_str());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "a" | "area" => {
                    for attr in attrs_mut.iter_mut() {
                        let attr_name: &str = &attr.name.local;
                        if attr_name.eq_ignore_ascii_case("href") {
                            let attr_value = attr.value.trim();

                            if opt_no_js && attr_value.starts_with("javascript:") {
                                attr.value.clear();
                                // Replace with empty JS call to preserve original behavior
                                attr.value.push_slice("javascript:;");
                                continue;
                            }

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
                    // Remove integrity and src attributes, keep values of the last ones
                    let mut script_integrity: String = str!();
                    let mut script_src: String = str!();
                    let mut i = 0;
                    while i < attrs_mut.len() {
                        let attr_name: &str = &attrs_mut[i].name.local;
                        if attr_name.eq_ignore_ascii_case("integrity") {
                            script_integrity = str!(attrs_mut.remove(i).value.trim());
                        } else if attr_name.eq_ignore_ascii_case("src") {
                            script_src = str!(attrs_mut.remove(i).value.trim());
                        } else {
                            i += 1;
                        }
                    }

                    if opt_no_js {
                        // Empty inner content (src is already gone)
                        node.children.borrow_mut().clear();
                    } else if !script_src.is_empty() {
                        let script_full_url = resolve_url(&url, script_src).unwrap_or_default();
                        match retrieve_asset(cache, client, &url, &script_full_url, opt_silent) {
                            Ok((script_data, script_final_url, _script_media_type)) => {
                                // Only embed if we're able to validate integrity
                                if script_integrity.is_empty()
                                    || has_proper_integrity(&script_data, &script_integrity)
                                {
                                    let script_data_url = data_to_data_url(
                                        "application/javascript",
                                        &script_data,
                                        &script_final_url,
                                    );
                                    // Add new data URL src attribute
                                    attrs_mut.push(Attribute {
                                        name: QualName::new(None, ns!(), local_name!("src")),
                                        value: Tendril::from_slice(script_data_url.as_ref()),
                                    });
                                }
                            }
                            Err(_) => {
                                // Keep remote reference if unable to retrieve the asset
                                if is_http_url(script_full_url.clone()) {
                                    attrs_mut.push(Attribute {
                                        name: QualName::new(None, ns!(), local_name!("src")),
                                        value: Tendril::from_slice(script_full_url.as_ref()),
                                    });
                                }
                            }
                        };
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
                                let replacement = embed_css(
                                    cache,
                                    client,
                                    &url,
                                    tendril.as_ref(),
                                    opt_no_fonts,
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
                        let attr_name: &str = &attr.name.local;
                        if attr_name.eq_ignore_ascii_case("action") {
                            let form_action = attr.value.trim();
                            // Modify action property to ensure it's a full URL
                            if !is_http_url(form_action) {
                                let form_action_full_url =
                                    resolve_url(&url, form_action).unwrap_or_default();
                                attr.value.clear();
                                attr.value.push_slice(form_action_full_url.as_str());
                            }
                        }
                    }
                }
                "frame" | "iframe" => {
                    for attr in attrs_mut.iter_mut() {
                        let attr_name: &str = &attr.name.local;
                        if attr_name.eq_ignore_ascii_case("src") {
                            if opt_no_frames {
                                // Empty the src attribute
                                attr.value.clear();
                                continue;
                            }

                            let frame_src = attr.value.trim();

                            // Ignore (i)frames with empty source — they cause infinite loops
                            if frame_src.is_empty() {
                                continue;
                            }

                            let frame_full_url = resolve_url(&url, frame_src).unwrap_or_default();
                            let frame_url_fragment = get_url_fragment(frame_full_url.clone());
                            match retrieve_asset(cache, client, &url, &frame_full_url, opt_silent) {
                                Ok((frame_data, frame_final_url, frame_media_type)) => {
                                    let frame_dom =
                                        html_to_dom(&String::from_utf8_lossy(&frame_data));
                                    walk_and_embed_assets(
                                        cache,
                                        client,
                                        &frame_final_url,
                                        &frame_dom.document,
                                        opt_no_css,
                                        opt_no_fonts,
                                        opt_no_frames,
                                        opt_no_js,
                                        opt_no_images,
                                        opt_silent,
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
                                    attr.value.clear();
                                    let assembled_url: String = url_with_fragment(
                                        frame_data_url.as_str(),
                                        frame_url_fragment.as_str(),
                                    );
                                    attr.value.push_slice(assembled_url.as_str());
                                }
                                Err(_) => {
                                    // Keep remote reference if unable to retrieve the asset
                                    if is_http_url(frame_full_url.clone()) {
                                        attr.value.clear();
                                        let assembled_url: String = url_with_fragment(
                                            frame_full_url.as_str(),
                                            frame_url_fragment.as_str(),
                                        );
                                        attr.value.push_slice(assembled_url.as_str());
                                    }
                                }
                            }
                        }
                    }
                }
                "video" => {
                    for attr in attrs_mut.iter_mut() {
                        let attr_name: &str = &attr.name.local;
                        if attr_name.eq_ignore_ascii_case("poster") {
                            let video_poster_url = attr.value.trim();

                            // Skip posters with empty source
                            if video_poster_url.is_empty() {
                                continue;
                            }

                            if opt_no_images {
                                attr.value.clear();
                                continue;
                            }

                            let video_poster_full_url =
                                resolve_url(&url, video_poster_url).unwrap_or_default();
                            let video_poster_url_fragment =
                                get_url_fragment(video_poster_full_url.clone());
                            match retrieve_asset(
                                cache,
                                client,
                                &url,
                                &video_poster_full_url,
                                opt_silent,
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
                                    attr.value.clear();
                                    let assembled_url: String = url_with_fragment(
                                        video_poster_data_url.as_str(),
                                        video_poster_url_fragment.as_str(),
                                    );
                                    attr.value.push_slice(assembled_url.as_str());
                                }
                                Err(_) => {
                                    // Keep remote reference if unable to retrieve the asset
                                    if is_http_url(video_poster_full_url.clone()) {
                                        attr.value.clear();
                                        let assembled_url: String = url_with_fragment(
                                            video_poster_full_url.as_str(),
                                            video_poster_url_fragment.as_str(),
                                        );
                                        attr.value.push_slice(assembled_url.as_str());
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            // Process style attributes
            if opt_no_css {
                // Get rid of style attributes
                let mut i = 0;
                while i < attrs_mut.len() {
                    let attr_name: &str = &attrs_mut[i].name.local;
                    if attr_name.eq_ignore_ascii_case("style") {
                        attrs_mut.remove(i);
                    } else {
                        i += 1;
                    }
                }
            } else {
                // Otherwise, parse any links found in the attributes
                for attribute in attrs_mut
                    .iter_mut()
                    .filter(|a| a.name.local.as_ref().eq_ignore_ascii_case("style"))
                {
                    let replacement = embed_css(
                        cache,
                        client,
                        &url,
                        attribute.value.as_ref(),
                        opt_no_fonts,
                        opt_no_images,
                        opt_silent,
                    );
                    // let replacement = str!();
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
                    opt_no_fonts,
                    opt_no_frames,
                    opt_no_js,
                    opt_no_images,
                    opt_silent,
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

pub fn metadata_tag(url: &str) -> String {
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
