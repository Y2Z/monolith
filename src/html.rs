use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{SecondsFormat, Utc};
use encoding_rs::{CoderResult, Encoder, Encoding, UTF_8};
use html5ever::interface::{Attribute, QualName};
use html5ever::serialize::{SerializeOpts, serialize};
use html5ever::tendril::{StrTendril, TendrilSink, format_tendril};
use html5ever::tree_builder::{TreeSink, create_element};
use html5ever::{LocalName, ns};
use html5ever::{parse_document, parse_fragment};
use markup5ever_rcdom::{Handle, Node, NodeData, RcDom, SerializableHandle};
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::cell::RefCell;
use std::default::Default;
use std::io::{self, Write};
use std::rc::Rc;

use crate::core::{
    MonolithOptions, MonolithOutputFormat, declared_encoding, parse_media_type, print_error_message,
};
use crate::cors::{PageOrigin, RequestMode};
use crate::csp::{Destination, Inline, Policies, Policy, Request, Restrictions};
use crate::css::{embed_css, embed_stylesheet};
use crate::js::{attr_is_event_handler, escape_script_end_tag};
use crate::replay::rewrite_imports;
use crate::session::Session;
use crate::source_maps::{remove_source_map_reference, remove_source_map_reference_from_text};
use crate::url::{
    EMPTY_IMAGE_DATA_URL, Url, clean_url, create_data_url, create_srcset_data_url,
    is_url_and_has_protocol, parse_data_url, resolve_url,
};

/// Attributes that make elements load something, other than src (which is handled separately)
const WIPED_ATTRIBUTES: [&str; 6] = ["data", "href", "integrity", "poster", "srcdoc", "srcset"];
/// Wiping these leaves them in place, there's nothing to unwrap
const ELEMENTS_WIPED_IN_PLACE: [&str; 10] = [
    "audio", "embed", "frame", "iframe", "image", "img", "object", "source", "track", "video",
];
/// Same as above, except their contents would end up on the page as text if they got unwrapped
const ELEMENTS_WITH_RAW_CONTENTS: [&str; 3] = ["noscript", "script", "style"];

const FAVICON_VALUES: &[&str] = &["icon", "shortcut icon"];

/// SVG contents that an element has brought in, by the URL it was requested by and where it
/// ended up, along with the fragment that was taken from it (if any).
struct InlinedSvg {
    requested_url: Url,
    final_url: Url,
    fragment: Option<String>,
}

#[derive(PartialEq, Eq)]
pub enum LinkType {
    Alternate,
    AppleTouchIcon,
    DnsPrefetch,
    Favicon,
    Preload,
    Stylesheet,
    Manifest,
}

pub struct SrcSetItem<'a> {
    pub path: &'a str,
    pub descriptor: &'a str, // Width or pixel density descriptor
}

pub fn add_favicon(document: &Handle, favicon_data_url: String) -> RcDom {
    let mut buf: Vec<u8> = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(document.clone()),
        SerializeOpts::default(),
    )
    .expect("unable to serialize DOM into buffer");

    let dom = html_to_dom(&buf, "utf-8".to_string());
    for head in find_nodes(&dom.document, vec!["html", "head"]).iter() {
        let favicon_node = create_element(
            &dom,
            QualName::new(None, ns!(html), LocalName::from("link")),
            vec![
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("rel")),
                    value: format_tendril!("icon"),
                },
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("href")),
                    value: format_tendril!("{}", favicon_data_url),
                },
            ],
        );

        // Insert favicon LINK tag into HEAD
        head.children.borrow_mut().push(favicon_node.clone());
    }

    dom
}

pub fn check_integrity(data: &[u8], integrity: &str) -> bool {
    if let Some(expected) = integrity.strip_prefix("sha256-") {
        BASE64_STANDARD.encode(Sha256::digest(data)) == expected
    } else if let Some(expected) = integrity.strip_prefix("sha384-") {
        BASE64_STANDARD.encode(Sha384::digest(data)) == expected
    } else if let Some(expected) = integrity.strip_prefix("sha512-") {
        BASE64_STANDARD.encode(Sha512::digest(data)) == expected
    } else {
        false
    }
}

/// Whether elements of any of the given names get removed or wiped no matter where they are,
/// which makes it possible to also prevent them from loading anything if added by scripts
fn targets_any_element(options: &MonolithOptions, names: &[&str]) -> bool {
    parse_selectors(&options.remove_elements)
        .iter()
        .chain(parse_selectors(&options.wipe_elements).iter())
        .any(|selector| {
            matches!(selector.as_slice(), [step] if names.iter().any(|name| step_matches(step, name)))
        })
}

/// Whether any selector goes after elements of the given names, in any part of the document.
/// A wildcard only counts on its own, "main *" isn't going after everything there is.
pub fn targets_element_anywhere(options: &MonolithOptions, names: &[&str]) -> bool {
    parse_selectors(&options.remove_elements)
        .iter()
        .chain(parse_selectors(&options.wipe_elements).iter())
        .any(|selector| match selector.as_slice() {
            [step] if step == "*" => true,
            [.., last_step] => names
                .iter()
                .any(|name| last_step.eq_ignore_ascii_case(name)),
            [] => false,
        })
}

pub fn compose_csp(options: &MonolithOptions) -> String {
    restrictions(options)
        .as_policy()
        .map(|policy| policy.serialize())
        .unwrap_or_default()
}

/// Names of META elements that say where documents have come from, when, and what has saved them.
pub const METADATA_NAMES: [&str; 3] = ["generator", "dcterms.source", "dcterms.created"];

/// What Monolith keeps documents it saves from doing, on top of what their policies do.
pub fn restrictions(options: &MonolithOptions) -> Restrictions {
    Restrictions {
        // Documents that replay responses have no business reaching out to anything, even if not
        // all of their URLs could be taken care of, and their modules get put together on the spot
        isolate: options.isolate || options.replay,
        allows_blobs: options.replay,
        no_scripts: options.remove_js,
        no_styles: options.remove_styles,
        no_fonts: options.remove_fonts,
        no_images: options.remove_images,
        no_frames: targets_any_element(options, &["frame", "iframe"]),
        no_objects: targets_any_element(options, &["embed", "object"]),
        custom_base: options.base_url.is_some(),
    }
}

/// Whether the element is a META element of HEAD that has a Content Security Policy in it (only
/// those count, see csp.rs).
fn is_csp_meta(node: &Handle) -> bool {
    let weak_parent = node.parent.take();
    let parent: Option<Handle> = weak_parent.as_ref().and_then(|weak| weak.upgrade());
    node.parent.set(weak_parent);

    get_node_name(node) == Some("meta")
        && parent.is_some_and(|parent| get_node_name(&parent) == Some("head"))
        && get_node_attr(node, "http-equiv")
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("content-security-policy"))
}

/// Policies of META elements of the document's HEAD, in order.
pub fn csp_meta_contents(document: &Handle) -> Vec<String> {
    find_nodes(document, vec!["html", "head", "meta"])
        .iter()
        .filter(|node| is_csp_meta(node))
        .filter_map(|node| get_node_attr(node, "content"))
        .collect()
}

/// Policies the document is going to have: those it came with, and those of its META elements.
pub fn document_policies(document: &Handle, url: &Url, header: Option<&str>) -> Policies {
    let mut policies = Policies::of(url, header);
    for content in csp_meta_contents(document) {
        policies.add_meta(&content);
    }
    policies
}

/// Puts given policies into META elements at the very start of the document's HEAD, instead of
/// whatever policies it had there.
pub fn set_content_security_policies(dom: &RcDom, policies: &[String]) {
    let Some(head) = get_child_node_by_name(&dom.document, "html")
        .and_then(|html| get_child_node_by_name(&html, "head"))
    else {
        return;
    };
    head.children.borrow_mut().retain(|node| !is_csp_meta(node));

    let metas: Vec<Handle> = policies
        .iter()
        .map(|policy| {
            let meta = create_element(
                dom,
                QualName::new(None, ns!(html), LocalName::from("meta")),
                vec![
                    Attribute {
                        name: QualName::new(None, ns!(), LocalName::from("http-equiv")),
                        value: format_tendril!("Content-Security-Policy"),
                    },
                    Attribute {
                        name: QualName::new(None, ns!(), LocalName::from("content")),
                        value: format_tendril!("{}", policy),
                    },
                ],
            );
            meta.parent.set(Some(Rc::downgrade(&head)));
            meta
        })
        .collect();
    // Policies only apply to what comes after them
    head.children.borrow_mut().splice(0..0, metas);
}

/// Keeps hashes of what's in elements named by nonce, the way they're about to be saved
/// (stylesheets may have become STYLE elements, see inline_stylesheets()).
pub fn hash_named_elements(session: &mut Session, node: &Handle) {
    if let Some(name) = get_node_name(node)
        && matches!(name, "script" | "style")
        && get_node_attr(node, "nonce").as_deref() == Some(session.saved_content.nonce.as_str())
        && get_node_attr(node, "src").is_none()
    {
        let inline = match name {
            "script" => Inline::Script,
            _ => Inline::Style,
        };
        session
            .saved_content
            .add_inline(inline, &node_text_content(node));
    }
    for child in node.children.borrow().iter() {
        hash_named_elements(session, child);
    }
}

/// Policies the saved document gets: those of the page rewritten to allow what it's been made of,
/// with what Monolith keeps documents from doing added to the first one.
pub fn saved_policies(session: &Session) -> Vec<String> {
    let mut policies = session.content_security.rewritten(&session.saved_content);
    let restrictions = restrictions(&session.options);
    match policies.first_mut() {
        Some(policy) => restrictions.apply(policy, &session.saved_content),
        None => policies.extend(restrictions.as_policy()),
    }
    policies.iter().map(|policy| policy.serialize()).collect()
}

fn remove_named_meta_elements(node: &Handle, name: &str) {
    node.children.borrow_mut().retain(|child| {
        get_node_name(child) != Some("meta")
            || !get_node_attr(child, "name").is_some_and(|value| value.eq_ignore_ascii_case(name))
    });
    for child in node.children.borrow().iter() {
        remove_named_meta_elements(child, name);
    }
}

/// Says what has saved the document, when, and where it came from, the way META elements do it:
/// whatever the document had said about that before gets replaced. Only addresses of websites get
/// given away, and never along with credentials.
pub fn set_metadata(dom: &RcDom, source: Option<&Url>) {
    let source: Option<Url> = source
        .filter(|url| url.scheme() == "http" || url.scheme() == "https")
        .map(|url| {
            let mut url = clean_url(url.clone());
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url
        });
    let values = [
        Some(format!(
            "{} {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )),
        source.map(|url| url.to_string()),
        Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)),
    ];
    let Some(head) = find_nodes(&dom.document, vec!["html", "head"])
        .into_iter()
        .next()
    else {
        return;
    };

    // What there's nothing to replace with stays, saving a saved page keeps where it came from
    for (name, value) in METADATA_NAMES.iter().zip(values) {
        let Some(value) = value else {
            continue;
        };
        remove_named_meta_elements(&dom.document, name);
        let meta = create_element(
            dom,
            QualName::new(None, ns!(html), LocalName::from("meta")),
            vec![
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("name")),
                    value: format_tendril!("{}", name),
                },
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("content")),
                    value: format_tendril!("{}", value),
                },
            ],
        );
        meta.parent.set(Some(Rc::downgrade(&head)));
        head.children.borrow_mut().push(meta);
    }
}

pub fn embed_srcset(session: &mut Session, document_url: &Url, srcset: &str) -> String {
    embed_srcset_as(session, document_url, srcset, RequestMode::NoCors)
}

/// Same as embed_srcset(), for images requested in given mode (see cors.rs).
pub fn embed_srcset_as(
    session: &mut Session,
    document_url: &Url,
    srcset: &str,
    mode: RequestMode,
) -> String {
    let srcset_items: Vec<SrcSetItem> = parse_srcset(srcset);

    // Embed assets
    let mut result: String = "".to_string();
    let mut i: usize = srcset_items.len();
    for srcset_item in srcset_items {
        if session.options.remove_images {
            result.push_str(EMPTY_IMAGE_DATA_URL);
        } else {
            let image_full_url: Url = resolve_url(document_url, srcset_item.path);
            match session.retrieve_for(
                document_url,
                &image_full_url,
                mode,
                &Request::to(Destination::Image),
            ) {
                // What the page isn't allowed to use shows nothing
                Ok(None) => result.push_str(EMPTY_IMAGE_DATA_URL),
                Ok(Some((image_data, image_final_url, image_media_type, image_charset))) => {
                    if session.options.output_format != MonolithOutputFormat::MHTML {
                        // Append retrieved asset as a data URL
                        result.push_str(&create_srcset_data_url(
                            &image_media_type,
                            &image_charset,
                            image_data,
                            &image_final_url,
                            image_full_url.fragment(),
                        ));
                    } else {
                        // MHTML
                        result.push_str(image_full_url.as_ref());
                    }
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

        if !srcset_item.descriptor.is_empty() {
            result.push(' ');
            result.push_str(srcset_item.descriptor);
        }

        if i > 1 {
            result.push_str(", ");
        }

        i -= 1;
    }

    result
}

/// Attributes of stylesheet links that mean something for STYLE elements too, and ones that
/// belong to any element.
fn is_kept_on_style_element(name: &str) -> bool {
    matches!(
        name,
        "media" | "title" | "nonce" | "blocking" | "id" | "class" | "lang" | "dir"
    ) || name.starts_with("data-")
}

/// Turns stylesheets that ended up as data URLs into STYLE elements: there's no telling how long
/// a stylesheet can be, and browsers limit how long URLs are.
///
/// Links that something can tell apart from STYLE elements stay the way they are: ones that wait
/// for their load or error events, alternate and disabled ones, and ones holding something that
/// would end STYLE elements early.
pub fn inline_stylesheets(dom: &RcDom, node: &Handle) {
    let children: Vec<Handle> = node.children.borrow().clone();

    for (index, child) in children.iter().enumerate() {
        if let Some(style) = stylesheet_as_style_element(dom, child) {
            style.parent.set(Some(Rc::downgrade(node)));
            node.children.borrow_mut()[index] = style;
        } else {
            inline_stylesheets(dom, child);
        }
    }
}

fn stylesheet_as_style_element(dom: &RcDom, node: &Handle) -> Option<Handle> {
    let NodeData::Element { name, attrs, .. } = &node.data else {
        return None;
    };
    if name.ns != ns!(html) || &*name.local != "link" {
        return None;
    }
    let attrs = attrs.borrow();
    let attribute = |wanted: &str| {
        attrs
            .iter()
            .find(|attr| &*attr.name.local == wanted)
            .map(|attr| attr.value.to_string())
    };
    let rel = attribute("rel")?.to_lowercase();
    let rels: Vec<&str> = rel.split_whitespace().collect();
    if rels != ["stylesheet"]
        || attribute("disabled").is_some()
        || attrs
            .iter()
            .any(|attr| attr_is_event_handler(&attr.name.local))
    {
        return None;
    }
    let href = Url::parse(&attribute("href")?).ok()?;
    if href.scheme() != "data" {
        return None;
    }
    let (media_type, charset, data) = parse_data_url(&href);
    if !media_type.eq_ignore_ascii_case("text/css") {
        return None;
    }
    let css = match Encoding::for_label(charset.as_bytes()) {
        Some(encoding) => encoding.decode_with_bom_removal(&data).0.into_owned(),
        None => String::from_utf8_lossy(&data).into_owned(),
    };
    if css.to_lowercase().contains("</style") {
        return None;
    }

    let kept: Vec<Attribute> = attrs
        .iter()
        .filter(|attr| is_kept_on_style_element(&attr.name.local))
        .cloned()
        .collect();
    let style = create_element(
        dom,
        QualName::new(None, ns!(html), LocalName::from("style")),
        kept,
    );
    let text = create_text_node(&css);
    text.parent.set(Some(Rc::downgrade(&style)));
    style.children.borrow_mut().push(text);

    Some(style)
}

pub fn create_html_element(dom: &RcDom, name: &str) -> Handle {
    create_element(
        dom,
        QualName::new(None, ns!(html), LocalName::from(name)),
        vec![],
    )
}

pub fn create_text_node(text: &str) -> Handle {
    Node::new(NodeData::Text {
        contents: RefCell::new(StrTendril::from(text)),
    })
}

pub fn find_nodes(node: &Handle, mut path: Vec<&str>) -> Vec<Handle> {
    let mut result = vec![];

    while !path.is_empty() {
        match node.data {
            NodeData::Document | NodeData::Element { .. } => {
                // Dig deeper
                for child in node.children.borrow().iter() {
                    if get_node_name(child)
                        .unwrap_or_default()
                        .eq_ignore_ascii_case(path[0])
                    {
                        if path.len() == 1 {
                            result.push(child.clone());
                        } else {
                            result.append(&mut find_nodes(child, path[1..].to_vec()));
                        }
                    }
                }
            }
            _ => {}
        }

        path.remove(0);
    }

    result
}

pub fn get_base_url(handle: &Handle) -> Option<String> {
    if let Some(base_node) = find_nodes(handle, vec!["html", "head", "base"]).first() {
        // Only the first base tag matters (we ignore the rest, if there's any)
        return get_node_attr(base_node, "href");
    }

    None
}

pub fn get_charset(node: &Handle) -> Option<String> {
    find_nodes(node, vec!["html", "head", "meta"])
        .iter()
        .find_map(get_meta_charset)
}

/// Returns the charset a META node declares, either via <meta charset="..." />
/// or <meta http-equiv="content-type" content="text/html; charset=..." />
fn get_meta_charset(meta_node: &Handle) -> Option<String> {
    if let Some(charset) = get_node_attr(meta_node, "charset") {
        return Some(charset);
    }

    if get_node_attr(meta_node, "http-equiv")
        .unwrap_or_default()
        .eq_ignore_ascii_case("content-type")
    {
        // A content type without a charset doesn't declare anything
        let (_media_type, charset, _is_base64) =
            parse_media_type(&get_node_attr(meta_node, "content")?);
        return charset;
    }

    None
}

// TODO: get rid of this function (replace with find_nodes)
pub fn get_child_node_by_name(parent: &Handle, node_name: &str) -> Option<Handle> {
    let children = parent.children.borrow();
    let matching_children = children.iter().find(|child| match child.data {
        NodeData::Element { ref name, .. } => &*name.local == node_name,
        _ => false,
    });
    matching_children.cloned()
}

pub fn get_node_attr(node: &Handle, attr_name: &str) -> Option<String> {
    match &node.data {
        NodeData::Element { attrs, .. } => {
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
        NodeData::Element { name, .. } => Some(name.local.as_ref()),
        _ => None,
    }
}

pub fn get_parent_node(child: &Handle) -> Handle {
    let weak_parent = child.parent.take();
    let parent: Handle = weak_parent
        .as_ref()
        .and_then(|weak| weak.upgrade())
        .unwrap();
    child.parent.set(weak_parent);
    parent
}

/// How what SOURCE elements point at gets requested: the way their AUDIO or VIDEO element says,
/// or the IMG element of their PICTURE element (it comes after them), see cors.rs.
pub fn source_request_mode(node: &Handle) -> RequestMode {
    let Some(parent) = node
        .parent
        .take()
        .inspect(|parent| node.parent.set(Some(parent.clone())))
        .and_then(|parent| parent.upgrade())
    else {
        return RequestMode::NoCors;
    };
    let crossorigin = match get_node_name(&parent) {
        Some("audio" | "video") => get_node_attr(&parent, "crossorigin"),
        Some("picture") => parent
            .children
            .borrow()
            .iter()
            .find(|child| get_node_name(child) == Some("img"))
            .and_then(|img| get_node_attr(img, "crossorigin")),
        _ => None,
    };

    RequestMode::of_attribute(crossorigin.as_deref())
}

pub fn get_robots(handle: &Handle) -> Option<String> {
    for meta_node in find_nodes(handle, vec!["html", "head", "meta"]).iter() {
        // Only the first base tag matters (we ignore the rest, if there's any)
        if get_node_attr(meta_node, "name")
            .unwrap_or_default()
            .eq_ignore_ascii_case("robots")
        {
            return get_node_attr(meta_node, "content");
        }
    }

    None
}

pub fn get_title(node: &Handle) -> Option<String> {
    for title_node in find_nodes(node, vec!["html", "head", "title"]).iter() {
        for child_node in title_node.children.borrow().iter() {
            if let NodeData::Text { ref contents } = child_node.data {
                return Some(contents.borrow().to_string());
            }
        }
    }

    None
}

pub fn has_favicon(handle: &Handle) -> bool {
    let mut found_favicon: bool = false;

    for link_node in find_nodes(handle, vec!["html", "head", "link"]).iter() {
        if let Some(attr_value) = get_node_attr(link_node, "rel")
            && is_favicon(attr_value.trim())
        {
            found_favicon = true;
            break;
        }
    }

    found_favicon
}

pub fn html_to_dom(data: &[u8], document_encoding: String) -> RcDom {
    let s: String;

    if let Some(encoding) = Encoding::for_label(document_encoding.as_bytes()) {
        let (string, _, _) = encoding.decode(data);
        s = string.to_string();
    } else {
        s = String::from_utf8_lossy(data).to_string();
    }

    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut s.as_bytes())
        .unwrap();
    move_template_contents(&dom.document);

    dom
}

/// Parsers keep what's inside of TEMPLATE elements away from the tree, where nothing can find it
/// and it doesn't get serialized, which would've left all of them empty.
fn move_template_contents(node: &Handle) {
    if let NodeData::Element {
        ref template_contents,
        ..
    } = node.data
        && let Some(contents) = template_contents.borrow().as_ref()
    {
        let children: Vec<Handle> = contents.children.borrow_mut().drain(..).collect();
        for child in children.iter() {
            child.parent.set(Some(Rc::downgrade(node)));
        }
        node.children.borrow_mut().extend(children);
    }

    for child in node.children.borrow().iter() {
        move_template_contents(child);
    }
}

pub fn is_favicon(attr_value: &str) -> bool {
    FAVICON_VALUES.contains(&attr_value.to_lowercase().as_str())
}

fn is_noscript(node: &Handle) -> bool {
    matches!(&node.data, NodeData::Element { name, .. } if name.ns == ns!(html) && &*name.local == "noscript")
}

/// Parses HTML as a fragment, as if it were the inner HTML of a `context` element.
/// Resulting tree is always: Document -> <html> -> [parsed nodes]
fn parse_fragment_dom(context: &QualName, html: StrTendril) -> RcDom {
    parse_fragment(
        RcDom::default(),
        Default::default(),
        context.clone(),
        vec![],
        true,
    )
    .one(html)
}

/// Same as parse_fragment_dom(), but returns the parsed nodes detached from the throwaway DOM
/// (callers are responsible for setting their parent pointers)
fn parse_html_fragment(context: &QualName, html: StrTendril) -> Vec<Handle> {
    let fragment: RcDom = parse_fragment_dom(context, html);
    let root: Option<Handle> = fragment.document.children.borrow().first().cloned();
    root.map(|html_node| html_node.children.take())
        .unwrap_or_default()
}

pub fn parse_link_type(link_attr_rel_value: &str) -> Vec<LinkType> {
    let mut types: Vec<LinkType> = vec![];

    for link_attr_rel_type in link_attr_rel_value.split_whitespace() {
        if link_attr_rel_type.eq_ignore_ascii_case("alternate") {
            types.push(LinkType::Alternate);
        } else if link_attr_rel_type.eq_ignore_ascii_case("dns-prefetch") {
            types.push(LinkType::DnsPrefetch);
        } else if link_attr_rel_type.eq_ignore_ascii_case("preload") {
            types.push(LinkType::Preload);
        } else if link_attr_rel_type.eq_ignore_ascii_case("stylesheet") {
            types.push(LinkType::Stylesheet);
        } else if is_favicon(link_attr_rel_type) {
            types.push(LinkType::Favicon);
        } else if link_attr_rel_type.eq_ignore_ascii_case("apple-touch-icon") {
            types.push(LinkType::AppleTouchIcon);
        } else if link_attr_rel_type.eq_ignore_ascii_case("manifest") {
            types.push(LinkType::Manifest);
        }
    }

    types
}

/// Parses srcset following the WHATWG algorithm:
/// https://html.spec.whatwg.org/multipage/images.html#parsing-a-srcset-attribute
/// Descriptors are kept verbatim, not validated (browsers drop invalid candidates themselves)
pub fn parse_srcset(srcset: &str) -> Vec<SrcSetItem<'_>> {
    let bytes = srcset.as_bytes();
    let mut srcset_items: Vec<SrcSetItem> = vec![];
    let mut pos: usize = 0;

    loop {
        // Skip whitespace and commas between candidates
        while pos < bytes.len() && (bytes[pos].is_ascii_whitespace() || bytes[pos] == b',') {
            pos += 1;
        }
        if pos == bytes.len() {
            break;
        }

        // URL is a run of non-whitespace characters (may contain commas, e.g. data URLs)
        let path_start = pos;
        while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let mut path_end = pos;
        let mut descriptor_start = pos;
        let mut descriptor_end = pos;

        if bytes[path_end - 1] == b',' {
            // Trailing commas end the candidate, meaning it has no descriptors
            // (can't underflow: the URL never starts with a comma)
            while bytes[path_end - 1] == b',' {
                path_end -= 1;
            }
        } else {
            // Descriptors run until a comma that isn't inside parentheses
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            descriptor_start = pos;
            descriptor_end = pos;
            let mut in_parens = false;
            while pos < bytes.len() {
                let b = bytes[pos];
                pos += 1;
                if in_parens {
                    in_parens = b != b')';
                } else if b == b',' {
                    break;
                } else if b == b'(' {
                    in_parens = true;
                }
                if !b.is_ascii_whitespace() {
                    descriptor_end = pos;
                }
            }
        }

        srcset_items.push(SrcSetItem {
            path: &srcset[path_start..path_end],
            descriptor: &srcset[descriptor_start..descriptor_end],
        });
    }

    srcset_items
}

pub fn set_base_url(document: &Handle, base_href_value: String) -> RcDom {
    let mut buf: Vec<u8> = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(document.clone()),
        SerializeOpts::default(),
    )
    .expect("unable to serialize DOM into buffer");
    let dom = html_to_dom(&buf, "utf-8".to_string());

    if let Some(html_node) = get_child_node_by_name(&dom.document, "html")
        && let Some(head_node) = get_child_node_by_name(&html_node, "head")
    {
        // Check if BASE node already exists in the DOM tree
        if let Some(base_node) = get_child_node_by_name(&head_node, "base") {
            set_node_attr(&base_node, "href", Some(base_href_value));
        } else {
            let base_node = create_element(
                &dom,
                QualName::new(None, ns!(html), LocalName::from("base")),
                vec![Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("href")),
                    value: format_tendril!("{}", base_href_value),
                }],
            );

            // Insert newly created BASE node into HEAD
            head_node.children.borrow_mut().push(base_node.clone());
        }
    }

    dom
}

pub fn set_charset(dom: RcDom, charset: String) -> RcDom {
    let head_node: Handle = match find_nodes(&dom.document, vec!["html", "head"]).first() {
        Some(head_node) => head_node.clone(),
        None => return dom,
    };

    // Reuse the META node that declares the charset (if any), the way browsers would pick it
    let existing_meta_node: Option<Handle> =
        find_nodes(&dom.document, vec!["html", "head", "meta"])
            .into_iter()
            .find(|meta_node| get_meta_charset(meta_node).is_some());

    let meta_charset_node: Handle = match existing_meta_node {
        Some(meta_node) => {
            if get_node_attr(&meta_node, "charset").is_some() {
                set_node_attr(&meta_node, "charset", Some(charset));
            } else {
                set_node_attr(
                    &meta_node,
                    "content",
                    Some(format!("text/html;charset={}", charset)),
                );
            }
            meta_node
        }
        // The HTML namespace makes it get serialized as a void element (without "</meta>")
        None => create_element(
            &dom,
            QualName::new(None, ns!(html), LocalName::from("meta")),
            vec![Attribute {
                name: QualName::new(None, ns!(), LocalName::from("charset")),
                value: format_tendril!("{}", charset),
            }],
        ),
    };

    // Browsers only look for the charset within the first 1024 bytes of the document,
    // and embedded assets can easily push it past that, so it has to be HEAD's first element
    let mut children = head_node.children.borrow_mut();
    let is_first_element: bool = children
        .iter()
        .find(|child| matches!(child.data, NodeData::Element { .. }))
        .is_some_and(|first_element| Rc::ptr_eq(first_element, &meta_charset_node));
    if !is_first_element {
        children.retain(|child| !Rc::ptr_eq(child, &meta_charset_node));
        meta_charset_node
            .parent
            .set(Some(Rc::downgrade(&head_node)));
        children.insert(0, meta_charset_node);
    }
    drop(children);

    dom
}

/// Gets rid of all attributes except for the given ones
pub fn retain_node_attrs(node: &Handle, attr_names: &[&str]) {
    if let NodeData::Element { attrs, .. } = &node.data {
        attrs
            .borrow_mut()
            .retain(|attr| attr_names.contains(&attr.name.local.as_ref()));
    }
}

pub fn set_node_attr(node: &Handle, attr_name: &str, attr_value: Option<String>) {
    if let NodeData::Element { attrs, .. } = &node.data {
        let attrs_mut = &mut attrs.borrow_mut();
        let mut i = 0;
        let mut found_existing_attr: bool = false;

        while i < attrs_mut.len() {
            if &attrs_mut[i].name.local == attr_name {
                found_existing_attr = true;

                if let Some(attr_value) = attr_value.clone() {
                    let _ = &attrs_mut[i].value.clear();
                    let _ = &attrs_mut[i].value.push_slice(attr_value.as_str());
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
    };
}

pub fn set_robots(dom: RcDom, content_value: &str) -> RcDom {
    for meta_node in find_nodes(&dom.document, vec!["html", "head", "meta"]).iter() {
        if get_node_attr(meta_node, "name")
            .unwrap_or_default()
            .eq_ignore_ascii_case("robots")
        {
            set_node_attr(meta_node, "content", Some(content_value.to_string()));
            return dom;
        }
    }

    // Manually append robots META node to HEAD
    {
        let meta_charset_node: Handle = create_element(
            &dom,
            QualName::new(None, ns!(html), LocalName::from("meta")),
            vec![
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("name")),
                    value: format_tendril!("robots"),
                },
                Attribute {
                    name: QualName::new(None, ns!(), LocalName::from("content")),
                    value: format_tendril!("{}", content_value),
                },
            ],
        );

        // Insert newly created META charset node into HEAD
        if let Some(head_node) = find_nodes(&dom.document, vec!["html", "head"]).first() {
            head_node
                .children
                .borrow_mut()
                .push(meta_charset_node.clone());
        }
    }

    dom
}

/// Lets the given function modify contents of a NOSCRIPT element, which is otherwise just text
pub fn process_noscript_contents(node: &Handle, mut process: impl FnMut(&RcDom)) {
    let context = QualName::new(None, ns!(html), LocalName::from("body"));

    for child_node in node.children.borrow().iter() {
        if let NodeData::Text { ref contents } = child_node.data {
            // Get contents of NOSCRIPT node
            let mut noscript_contents = contents.borrow_mut();
            // Parse contents of NOSCRIPT node as a DOM fragment
            let fragment: RcDom = parse_fragment_dom(&context, noscript_contents.clone());
            process(&fragment);
            // Serialize the fragment's root; default SerializeOpts use
            // TraversalScope::ChildrenOnly, so the <html> wrapper itself
            // isn't written out
            let mut buf: Vec<u8> = Vec::new();
            if let Some(root) = fragment.document.children.borrow().first() {
                serialize(
                    &mut buf,
                    &SerializableHandle::from(root.clone()),
                    SerializeOpts::default(),
                )
                .expect("Unable to serialize DOM into buffer");
            }
            // Replace original contents with the processed HTML
            noscript_contents.clear();
            noscript_contents.push_slice(&String::from_utf8_lossy(&buf));
        }
    }
}

/// Removes or wipes elements of the document, including whatever is inside of NOSCRIPT elements.
///
/// Removing an element makes it disappear along with its contents. Wiping an element that can
/// load something leaves the element in place without its source, other elements get replaced
/// with their contents (e.g. links turn into plain text). Removing takes priority over wiping.
///
/// Each selector is a space-separated path of element names, each element in it being
/// a descendant of the previous one (e.g. "main div iframe"), "*" stands for any element.
/// Multiple selectors can be given within one string, separated by commas.
/// Selectors that don't make sense simply don't match anything.
pub fn strip_elements(document: &Handle, remove: &[String], wipe: &[String]) {
    let remove: Vec<Vec<String>> = parse_selectors(remove);
    let wipe: Vec<Vec<String>> = parse_selectors(wipe);

    if !remove.is_empty() || !wipe.is_empty() {
        strip_children(document, &remove, &wipe, &mut vec![]);
    }
}

fn parse_selectors(selectors: &[String]) -> Vec<Vec<String>> {
    selectors
        .iter()
        .flat_map(|selector_list| selector_list.split(','))
        .map(|selector| selector.split_whitespace().map(str::to_string).collect())
        .filter(|path: &Vec<String>| !path.is_empty())
        .collect()
}

fn step_matches(step: &str, name: &str) -> bool {
    step == "*" || step.eq_ignore_ascii_case(name)
}

/// The closest ancestor that fits is always the best pick, so there's no need to backtrack
fn selector_matches(selector: &[String], name: &str, ancestors: &[Handle]) -> bool {
    let Some((last_step, other_steps)) = selector.split_last() else {
        return false;
    };
    if !step_matches(last_step, name) {
        return false;
    }

    let mut ancestor_names = ancestors
        .iter()
        .rev()
        .filter_map(|node| get_node_name(node));
    other_steps
        .iter()
        .rev()
        .all(|step| ancestor_names.any(|ancestor_name| step_matches(step, ancestor_name)))
}

fn any_selector_matches(selectors: &[Vec<String>], name: &str, ancestors: &[Handle]) -> bool {
    selectors
        .iter()
        .any(|selector| selector_matches(selector, name, ancestors))
}

/// Gets rid of everything that makes the element load anything
fn wipe_element_in_place(node: &Handle, name: &str) {
    if name == "img" && get_node_attr(node, "src").is_some() {
        set_node_attr(node, "src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
    } else {
        set_node_attr(node, "src", None);
    }
    for attr_name in WIPED_ATTRIBUTES {
        set_node_attr(node, attr_name, None);
    }

    if ELEMENTS_WITH_RAW_CONTENTS.contains(&name) {
        node.children.borrow_mut().clear();
    }
}

/// Ancestors are tracked here since get_parent_node() is of no use for parsed NOSCRIPT
/// contents: they have no connection to the document they came from.
fn strip_children(
    parent: &Handle,
    remove: &[Vec<String>],
    wipe: &[Vec<String>],
    ancestors: &mut Vec<Handle>,
) {
    let mut children: Vec<Handle> = Vec::new();

    for child in parent.children.take() {
        let Some(name) = get_node_name(&child) else {
            children.push(child);
            continue;
        };
        let name: String = name.to_ascii_lowercase();

        if any_selector_matches(remove, &name, ancestors) {
            continue;
        }
        let is_wiped: bool = any_selector_matches(wipe, &name, ancestors);

        ancestors.push(child.clone());
        if name == "noscript" {
            process_noscript_contents(&child, |fragment| {
                // The root is just a wrapper that doesn't exist in the document
                if let Some(root) = fragment.document.children.borrow().first() {
                    strip_children(root, remove, wipe, ancestors);
                }
            });
        }
        strip_children(&child, remove, wipe, ancestors);
        ancestors.pop();

        if !is_wiped {
            children.push(child);
        } else if ELEMENTS_WIPED_IN_PLACE.contains(&name.as_str())
            || ELEMENTS_WITH_RAW_CONTENTS.contains(&name.as_str())
        {
            wipe_element_in_place(&child, &name);
            children.push(child);
        } else {
            for grandchild in child.children.take() {
                grandchild.parent.set(Some(Rc::downgrade(parent)));
                children.push(grandchild);
            }
        }
    }

    *parent.children.borrow_mut() = children;
}

/// Replaces every NOSCRIPT element with its contents parsed as real DOM nodes
pub fn unwrap_noscripts(node: &Handle) {
    fn has_noscript_child(node: &Handle) -> bool {
        node.children.borrow().iter().any(is_noscript)
    }

    // (3) Only nodes that actually have NOSCRIPT children get their children rebuilt.
    // It's a loop rather than an `if` because unwrapping can surface another NOSCRIPT
    // at this same level (e.g. text like `<noscript>` inside the original NOSCRIPT);
    // each pass consumes the text it parses, so this always terminates.
    if has_noscript_child(node) {
        // NOSCRIPT contents get parsed in the context of the NOSCRIPT's parent,
        // which is what a browser with JS disabled would effectively do
        let context: QualName = match &node.data {
            NodeData::Element { name, .. } => name.clone(),
            _ => QualName::new(None, ns!(html), LocalName::from("body")),
        };

        while has_noscript_child(node) {
            let old_children: Vec<Handle> = node.children.take();
            let mut new_children: Vec<Handle> = Vec::with_capacity(old_children.len());

            for child in old_children {
                if !is_noscript(&child) {
                    new_children.push(child);
                    continue;
                }

                // With scripting enabled, NOSCRIPT holds a raw text node;
                // anything that's already a real node gets moved over as-is
                for grandchild in child.children.take() {
                    let text: Option<StrTendril> = match &grandchild.data {
                        NodeData::Text { contents } => Some(contents.borrow().clone()),
                        _ => None,
                    };
                    match text {
                        Some(html) => new_children.extend(parse_html_fragment(&context, html)),
                        None => new_children.push(grandchild),
                    }
                }
            }

            for child in new_children.iter() {
                child.parent.set(Some(Rc::downgrade(node)));
            }
            *node.children.borrow_mut() = new_children;
        }
    }

    // Recurse into children (including freshly unwrapped ones)
    for child in node.children.borrow().iter() {
        unwrap_noscripts(child);
    }
    if let NodeData::Element {
        ref template_contents,
        ..
    } = node.data
        && let Some(ref contents) = *template_contents.borrow()
    {
        unwrap_noscripts(contents);
    }
}

pub fn serialize_document(
    dom: RcDom,
    document_encoding: String,
    options: &MonolithOptions,
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    serialize_document_into(dom, document_encoding, options, None, &mut buf)
        .expect("Unable to serialize DOM into buffer");

    buf
}

/// Writes the document out as it gets serialized, in the given encoding, with given markup
/// (if any) right after the opening tag of HEAD. Provides the last byte that has been written.
pub fn serialize_document_into(
    dom: RcDom,
    document_encoding: String,
    options: &MonolithOptions,
    after_head: Option<&[u8]>,
    out: &mut dyn Write,
) -> io::Result<Option<u8>> {
    if options.unwrap_noscript {
        unwrap_noscripts(&dom.document);
    }

    // UTF-8 is what the serializer makes, it only needs to be changed into something else
    let encoder = Some(document_encoding)
        .filter(|label| !label.is_empty())
        .and_then(|label| Encoding::for_label(label.as_bytes()))
        .map(|encoding| encoding.output_encoding())
        .filter(|encoding| *encoding != UTF_8)
        .map(|encoding| encoding.new_encoder());
    let mut injector = HeadInjector {
        markup: after_head,
        out,
        last_byte: None,
        tail: vec![],
        is_in_head_tag: false,
    };
    let mut encoder = EncodingWriter {
        encoder,
        out: &mut injector,
        pending: vec![],
    };

    let serializable: SerializableHandle = dom.document.into();
    serialize(&mut encoder, &serializable, SerializeOpts::default())?;
    encoder.finish()?;
    injector.finish()
}

/// Changes UTF-8 into another encoding as it gets written, what the encoding can't have
/// becomes numeric character references.
struct EncodingWriter<'a> {
    /// None for keeping it UTF-8
    encoder: Option<Encoder>,
    out: &'a mut dyn Write,
    /// Beginning of a character that is yet to be written in full
    pending: Vec<u8>,
}

impl EncodingWriter<'_> {
    fn encode(&mut self, last: bool) -> io::Result<()> {
        let Some(encoder) = self.encoder.as_mut() else {
            return Ok(());
        };
        let complete = match std::str::from_utf8(&self.pending) {
            Ok(text) => text.len(),
            Err(error) if last => {
                // Can't happen to what the serializer makes, but nothing should get lost
                return Err(io::Error::new(io::ErrorKind::InvalidData, error));
            }
            Err(error) => error.valid_up_to(),
        };
        let mut text = std::str::from_utf8(&self.pending[..complete]).unwrap();
        let mut buffer = [0u8; 8192];

        loop {
            let (result, read, written, _) = encoder.encode_from_utf8(text, &mut buffer, last);
            self.out.write_all(&buffer[..written])?;
            text = &text[read..];
            if result == CoderResult::InputEmpty {
                break;
            }
        }
        self.pending.drain(..complete);

        Ok(())
    }

    fn finish(mut self) -> io::Result<()> {
        self.encode(true)
    }
}

impl Write for EncodingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if self.encoder.is_none() {
            return self.out.write_all(buf);
        }
        self.pending.extend_from_slice(buf);
        self.encode(false)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

/// Puts markup right after the opening tag of HEAD, as the document goes through it.
struct HeadInjector<'a> {
    /// None once it's in, or if there's nothing to put in
    markup: Option<&'a [u8]>,
    out: &'a mut dyn Write,
    last_byte: Option<u8>,
    /// What could turn out to be the beginning of "<head"
    tail: Vec<u8>,
    /// Whether the opening tag of HEAD has begun, but not yet ended
    is_in_head_tag: bool,
}

const HEAD_TAG_START: &[u8] = b"<head";

impl HeadInjector<'_> {
    fn pass(&mut self, data: &[u8]) -> io::Result<()> {
        if let Some(byte) = data.last() {
            self.last_byte = Some(*byte);
            self.out.write_all(data)?;
        }
        Ok(())
    }

    fn finish(mut self) -> io::Result<Option<u8>> {
        let tail = std::mem::take(&mut self.tail);
        self.pass(&tail)?;
        self.out.flush()?;
        Ok(self.last_byte)
    }
}

impl Write for HeadInjector<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let Some(markup) = self.markup else {
            return self.pass(buf);
        };
        let mut data: Vec<u8> = std::mem::take(&mut self.tail);
        data.extend_from_slice(buf);
        let mut data: &[u8] = &data;

        if !self.is_in_head_tag {
            match data
                .windows(HEAD_TAG_START.len())
                .position(|part| part.eq_ignore_ascii_case(HEAD_TAG_START))
            {
                Some(start) => {
                    let end = start + HEAD_TAG_START.len();
                    self.pass(&data[..end])?;
                    data = &data[end..];
                    self.is_in_head_tag = true;
                }
                None => {
                    // What's at the end could be where it begins
                    let kept = data.len().min(HEAD_TAG_START.len() - 1);
                    let (passed, kept) = data.split_at(data.len() - kept);
                    self.pass(passed)?;
                    self.tail = kept.to_vec();
                    return Ok(());
                }
            }
        }

        match data.iter().position(|byte| *byte == b'>') {
            Some(end) => {
                self.pass(&data[..=end])?;
                self.pass(markup)?;
                self.markup = None;
                self.pass(&data[end + 1..])
            }
            None => self.pass(data),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

/// Whether USE elements get to show what's at given URL: browsers only fetch what's of the page's
/// own origin for them (in "same-origin" mode, CORS doesn't help), and nothing out of data URLs.
fn is_usable_by_use(session: &Session, url: &Url) -> bool {
    let is_usable = url.scheme() != "data"
        && session
            .page_origin
            .as_ref()
            .is_none_or(|origin| origin.is_same_origin(url));
    if !is_usable && !session.options.silent {
        print_error_message(&format!(
            "{} (browsers don't use SVGs of {})",
            clean_url(url.clone()),
            match url.scheme() {
                "data" => "data URLs",
                _ => "other origins",
            }
        ));
    }
    is_usable
}

/// Whether SCRIPT elements have code in them (or point at some), rather than data.
fn is_executable_script(node: &Handle) -> bool {
    let script_type = get_node_attr(node, "type").unwrap_or_default();
    let script_type = script_type.trim().to_ascii_lowercase();
    script_type.is_empty()
        || ["module", "importmap", "speculationrules"].contains(&script_type.as_str())
        || ["javascript", "ecmascript", "jscript", "livescript"]
            .iter()
            .any(|kind| script_type.contains(kind))
}

/// Text of an element, the way policies see it.
fn node_text_content(node: &Handle) -> String {
    node.children
        .borrow()
        .iter()
        .filter_map(|child| match &child.data {
            NodeData::Text { contents } => Some(contents.borrow().to_string()),
            _ => None,
        })
        .collect()
}

/// Names the element by the nonce that the saved document's policies allow, if it has any, and
/// has anything to run or apply.
fn name_by_nonce(session: &mut Session, node: &Handle, destination: Destination) {
    let has_source = ["src", "href"]
        .iter()
        .any(|attr_name| get_node_attr(node, attr_name).is_some_and(|value| !value.is_empty()));
    let content = node_text_content(node);
    if session.names_elements && (has_source || !content.trim().is_empty()) {
        set_node_attr(node, "nonce", Some(session.saved_content.nonce.clone()));
        session.saved_content.named.insert(destination);
        if !has_source {
            let inline = match destination {
                Destination::Script => Inline::Script,
                _ => Inline::Style,
            };
            session.saved_content.add_inline(inline, &content);
        }
    }
}

/// What a document that's in another one changes about the session, for when it's done.
pub struct OuterDocument {
    policies: Policies,
    names_elements: bool,
    remove_js: bool,
}

/// Makes given policies the ones of the document that's about to be processed.
pub fn enter_document(session: &mut Session, dom: &RcDom, policies: Policies) -> OuterDocument {
    // Elements get named by what the policies (those of META elements too) have
    let mut every_policy: Vec<Policy> = policies.list().to_vec();
    every_policy.extend(
        csp_meta_contents(&dom.document)
            .iter()
            .filter_map(|content| Policy::parse_meta(content)),
    );
    session.saved_content.add_nonces(&every_policy);
    let outer = OuterDocument {
        policies: std::mem::replace(&mut session.content_security, policies),
        names_elements: session.names_elements,
        remove_js: session.options.remove_js,
    };
    // Documents have the policies of what they're in, once saved (see csp.rs)
    session.names_elements = session.names_elements
        || !session.content_security.is_empty()
        || !csp_meta_contents(&dom.document).is_empty();
    // Sandboxes can be without scripts
    if session.content_security.forbids_scripts() {
        session.options.remove_js = true;
    }
    outer
}

/// Puts policies of the document that's been processed into it (those after the first n, the
/// rest are of what it's in), and makes the session what it was before.
pub fn leave_document(session: &mut Session, dom: &RcDom, n: usize, outer: OuterDocument) {
    let policies: Vec<String> = session
        .content_security
        .rewritten_from(n, &session.saved_content)
        .iter()
        .map(|policy| policy.serialize())
        .collect();
    set_content_security_policies(dom, &policies);

    session.content_security = outer.policies;
    session.names_elements = outer.names_elements;
    session.options.remove_js = outer.remove_js;
}

/// Where the document's BASE element points, if it does, and its policies allow it.
pub fn base_url_allowed(session: &Session, dom: &RcDom, document_url: &Url) -> Option<Url> {
    let base_url = resolve_url(document_url, &get_base_url(&dom.document)?);
    // What META elements before it say counts
    let mut policies = session.content_security.clone();
    for node in find_nodes(&dom.document, vec!["html", "head"])
        .iter()
        .flat_map(|head| head.children.borrow().clone())
    {
        if get_node_name(&node) == Some("base") && get_node_attr(&node, "href").is_some() {
            break;
        }
        if is_csp_meta(&node)
            && let Some(content) = get_node_attr(&node, "content")
        {
            policies.add_meta(&content);
        }
    }
    if !policies.allows_base(&base_url) {
        if !session.options.silent {
            print_error_message(&format!(
                "{} (base URL blocked by Content Security Policy)",
                base_url
            ));
        }
        return None;
    }
    Some(base_url)
}

/// Documents that frames have in their srcdoc attributes are the same as any other, they just
/// don't come from anywhere: they're of the same origin as what they're in, and their URLs get
/// resolved the same way, unless they say otherwise with BASE elements of their own.
fn embed_srcdoc(session: &mut Session, document_url: &Url, srcdoc: &str) -> String {
    let frame_dom = html_to_dom(srcdoc.as_bytes(), "utf-8".to_string());
    strip_elements(
        &frame_dom.document,
        &session.options.remove_elements,
        &session.options.wipe_elements,
    );
    // They have the policies of what they're in, and those of their own
    let inherited = session.content_security.len();
    let policies = session.content_security.inherited_by(document_url);
    let outer = enter_document(session, &frame_dom, policies);
    let frame_url = match base_url_allowed(session, &frame_dom, document_url) {
        Some(base_url) => base_url,
        None => document_url.clone(),
    };

    let environment_encoding = session.environment_encoding.replace(UTF_8);
    // What's in it isn't anything that could be referencing itself, so it goes past the check
    // that the document itself would get stopped by (it has the URL of what it's in)
    for child_node in frame_dom.document.children.borrow().iter() {
        walk(session, &frame_url, child_node);
    }
    session.environment_encoding = environment_encoding;
    leave_document(session, &frame_dom, inherited, outer);

    let mut frame_data: Vec<u8> = Vec::new();
    let serializable: SerializableHandle = frame_dom.document.into();
    serialize(&mut frame_data, &serializable, SerializeOpts::default()).unwrap();

    String::from_utf8_lossy(&frame_data).into_owned()
}

pub fn retrieve_and_embed_asset(
    session: &mut Session,
    document_url: &Url,
    node: &Handle,
    attr_name: &str,
    attr_value: &str,
) {
    let resolved_url: Url = resolve_url(document_url, attr_value);
    let node_name: &str = get_node_name(node).unwrap();
    let crossorigin: Option<String> = get_node_attr(node, "crossorigin");
    let mode: RequestMode = match (node_name, attr_name) {
        ("script", _) => RequestMode::of_script(
            get_node_attr(node, "type").as_deref(),
            crossorigin.as_deref(),
        ),
        // Posters aren't what crossorigin is about
        ("video", "poster") => RequestMode::NoCors,
        ("img" | "audio" | "video" | "link", _) => {
            RequestMode::of_attribute(crossorigin.as_deref())
        }
        ("source", _) => source_request_mode(node),
        _ => RequestMode::NoCors,
    };
    let is_stylesheet_link = node_name == "link"
        && parse_link_type(&get_node_attr(node, "rel").unwrap_or(String::from("")))
            .contains(&LinkType::Stylesheet);
    let is_frame = node_name == "frame" || node_name == "iframe";
    let destination: Destination = match (node_name, attr_name) {
        ("script", _) => Destination::Script,
        ("link", _) if is_stylesheet_link => Destination::Style,
        ("link", _)
            if parse_link_type(&get_node_attr(node, "rel").unwrap_or_default())
                .contains(&LinkType::Manifest) =>
        {
            Destination::Manifest
        }
        ("frame" | "iframe", _) => Destination::Frame,
        ("object" | "embed" | "applet", _) => Destination::Object,
        ("audio" | "source" | "track", _) | ("video", "src") => Destination::Media,
        _ => Destination::Image,
    };
    let nonce: Option<String> = matches!(destination, Destination::Script | Destination::Style)
        .then(|| get_node_attr(node, "nonce"))
        .flatten();
    let integrity: Option<String> = (destination == Destination::Script)
        .then(|| get_node_attr(node, "integrity"))
        .flatten();
    let request = Request {
        destination,
        nonce: nonce.as_deref(),
        integrity: integrity.as_deref(),
    };

    // Something that's being expanded already is referencing itself, directly or through other
    // assets; leave an empty asset in its place instead of expanding it forever
    if is_stylesheet_link && session.is_expanding(&resolved_url) {
        let empty_stylesheet = create_data_url("text/css", "utf-8", "", &resolved_url, None);
        set_node_attr(node, attr_name, Some(empty_stylesheet));
        return;
    }
    if is_frame && session.is_expanding(&resolved_url) {
        let empty_document = create_data_url(
            "text/html",
            "utf-8",
            "",
            &resolved_url,
            resolved_url.fragment(),
        );
        set_node_attr(node, attr_name, Some(empty_document));
        return;
    }

    // Integrity of what comes from other origins can't be checked without CORS, browsers don't
    // use what they can't check
    let has_integrity = matches!(node_name, "link" | "script")
        && get_node_attr(node, "integrity").is_some_and(|integrity| !integrity.is_empty());
    let is_cross_origin = session
        .page_origin
        .as_ref()
        .is_some_and(|origin| !origin.is_same_origin(&resolved_url));
    if has_integrity
        && is_cross_origin
        && mode == RequestMode::NoCors
        && resolved_url.scheme() != "data"
        && session.checks_cors()
    {
        if !session.options.silent {
            print_error_message(&format!(
                "{} (integrity can't be checked without CORS)",
                resolved_url
            ));
        }
        set_node_attr(node, "integrity", None);
        set_node_attr(node, attr_name, None);
        return;
    }

    match session.retrieve_for(&document_url.clone(), &resolved_url, mode, &request) {
        // What the page isn't allowed to use doesn't get to be in it
        Ok(None) => {
            set_node_attr(node, "integrity", None);
            set_node_attr(node, attr_name, None);
        }
        Ok(Some((data, final_url, media_type, charset))) => {
            // Check integrity if it's a LINK or SCRIPT element
            let mut ok_to_include: bool = true;
            if node_name == "link" || node_name == "script" {
                // Check integrity
                if let Some(node_integrity_attr_value) = get_node_attr(node, "integrity") {
                    if !node_integrity_attr_value.is_empty() {
                        ok_to_include = check_integrity(
                            session.original_data(&final_url).unwrap_or(&data),
                            &node_integrity_attr_value,
                        );
                    }

                    // Wipe the integrity attribute
                    set_node_attr(node, "integrity", None);
                }
            }

            if ok_to_include {
                if is_stylesheet_link {
                    // Stylesheet LINK elements require special treatment
                    let alias = session.enter_expansion_alias(&resolved_url, &final_url);
                    let css: String = embed_stylesheet(session, &final_url, &data, &charset);
                    if let Some(key) = alias {
                        session.leave_expansion_alias(&key);
                    }

                    // Create and embed data URL (decoded stylesheets are always stored as UTF-8)
                    let css_data_url = create_data_url(&media_type, "utf-8", css, &final_url, None);
                    set_node_attr(node, attr_name, Some(css_data_url));
                } else if is_frame {
                    // The frame's own URL can be where the request redirected to, even if it was
                    // requested by another one
                    if session.is_expanding(&final_url) {
                        let empty_document = create_data_url(
                            &media_type,
                            "utf-8",
                            "",
                            &final_url,
                            resolved_url.fragment(),
                        );
                        set_node_attr(node, attr_name, Some(empty_document));
                    } else {
                        // (I)FRAMEs are also quite different from conventional resources
                        let mut frame_dom = html_to_dom(&data, charset.clone());
                        let mut frame_encoding: Option<&'static Encoding> =
                            declared_encoding(&charset);

                        // Without a charset from the protocol, look for one declared inside the document
                        if frame_encoding.is_none()
                            && let Some(frame_charset) = get_charset(&frame_dom.document)
                            && let Some(encoding) = declared_encoding(&frame_charset)
                        {
                            frame_dom = html_to_dom(&data, encoding.name().to_string());
                            frame_encoding = Some(encoding);
                        }

                        // A byte order mark beats any declaration when it comes to decoding
                        if let Some((bom_encoding, _)) = Encoding::for_bom(&data) {
                            frame_encoding = Some(bom_encoding);
                        }

                        let alias = session.enter_expansion_alias(&resolved_url, &final_url);
                        let environment_encoding =
                            std::mem::replace(&mut session.environment_encoding, frame_encoding);
                        strip_elements(
                            &frame_dom.document,
                            &session.options.remove_elements,
                            &session.options.wipe_elements,
                        );
                        // Frames have policies of their own, documents of data URLs have those of
                        // what they're in too (see csp.rs)
                        let (frame_policies, inherited) = match final_url.scheme() == "data" {
                            true => (
                                session.content_security.inherited_by(&final_url),
                                session.content_security.len(),
                            ),
                            false => (
                                Policies::of(
                                    &final_url,
                                    session.received_policy(&final_url).as_deref(),
                                ),
                                0,
                            ),
                        };
                        let outer = enter_document(session, &frame_dom, frame_policies);
                        // Frames are documents of their own, with origins of their own
                        let page_origin = match session.page_origin.is_some() {
                            true => std::mem::replace(
                                &mut session.page_origin,
                                match session.content_security.has_opaque_origin() {
                                    true => Some(PageOrigin::Opaque),
                                    false => PageOrigin::of(&final_url),
                                },
                            ),
                            false => None,
                        };
                        walk(session, &final_url, &frame_dom.document);
                        session.environment_encoding = environment_encoding;
                        if page_origin.is_some() {
                            session.page_origin = page_origin;
                        }
                        if let Some(key) = alias {
                            session.leave_expansion_alias(&key);
                        }
                        leave_document(session, &frame_dom, inherited, outer);

                        let mut frame_data: Vec<u8> = Vec::new();
                        let serializable: SerializableHandle = frame_dom.document.into();
                        serialize(&mut frame_data, &serializable, SerializeOpts::default())
                            .unwrap();

                        // Create and embed data URL (serialized frames are always UTF-8)
                        let frame_data_url = create_data_url(
                            &media_type,
                            "utf-8",
                            frame_data,
                            &final_url,
                            resolved_url.fragment(),
                        );
                        set_node_attr(node, attr_name, Some(frame_data_url));
                    }
                } else {
                    // Every other type of element gets processed here

                    // Parse media type for SCRIPT elements
                    if node_name == "script" {
                        // Source maps don't come along (see source_maps.rs)
                        let data: &[u8] = remove_source_map_reference(&data);
                        let mut script_media_type =
                            get_node_attr(node, "type").unwrap_or(String::from("text/javascript"));

                        // JavaScript modules are a special case that uses `module` instead of a
                        // MIME type.
                        if script_media_type == "module" {
                            script_media_type = String::from("text/javascript");
                        } else if script_media_type == "importmap" {
                            // JavaScript import maps are another special case that uses `importmap`
                            // instead of a MIME type.
                            // An import map is a JSON object.
                            // There is ongoing work to register the MIME type
                            // `application/importmap+json` for it, but this has not been accepted
                            // yet.
                            script_media_type = String::from("application/json");
                        }
                        // TODO: There is also `speculationrules` but this is not yet standard.

                        // Browsers ignore both of these on scripts that have no source
                        let is_timed = get_node_attr(node, "async").is_some()
                            || get_node_attr(node, "defer").is_some();
                        // Scripts that have no source never get to load, or fail to
                        let is_awaited = get_node_attr(node, "onload").is_some()
                            || get_node_attr(node, "onerror").is_some();

                        if !is_timed
                            && !is_awaited
                            && (script_media_type == "text/javascript"
                                || script_media_type == "application/javascript")
                        {
                            // Embed javascript code instead of using data URLs
                            let script_dom: RcDom =
                                parse_document(RcDom::default(), Default::default())
                                    .one("<script>;</script>");
                            for script_node in
                                find_nodes(&script_dom.document, vec!["html", "head", "script"])
                                    .iter()
                            {
                                let text_node = &script_node.children.borrow()[0];

                                if let NodeData::Text { ref contents } = text_node.data {
                                    let mut tendril = contents.borrow_mut();
                                    tendril.clear();
                                    let code = match Encoding::for_label(charset.as_bytes()) {
                                        Some(encoding) => {
                                            encoding.decode_with_bom_removal(data).0.into_owned()
                                        }
                                        None => String::from_utf8_lossy(data).into_owned(),
                                    };
                                    tendril.push_slice(&escape_script_end_tag(&code));
                                }

                                node.children.borrow_mut().push(text_node.clone());
                                set_node_attr(node, attr_name, None);
                            }
                        } else {
                            // Create and embed data URL
                            let data_url = create_data_url(
                                &script_media_type,
                                &charset,
                                data,
                                &final_url,
                                resolved_url.fragment(),
                            );
                            set_node_attr(node, attr_name, Some(data_url));
                        }
                    } else {
                        // Create and embed data URL
                        let data_url = create_data_url(
                            &media_type,
                            &charset,
                            data,
                            &final_url,
                            resolved_url.fragment(),
                        );
                        set_node_attr(node, attr_name, Some(data_url));
                    }
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

pub fn walk(session: &mut Session, document_url: &Url, node: &Handle) {
    match node.data {
        NodeData::Document => {
            // A document that's being walked already is referencing itself, directly or through
            // frames; its contents are on their way into the result, so there's nothing more to add
            let Some(expansion_key) = session.enter_expansion(document_url) else {
                return;
            };

            // Dig deeper
            for child_node in node.children.borrow().iter() {
                walk(session, document_url, child_node);
            }

            session.leave_expansion(&expansion_key);
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let mut expansions_to_leave: Vec<(String, Option<String>)> = Vec::new();
            // External SVG contents brought into this element are relative to where that SVG is
            let mut child_document_url: Option<Url> = None;

            match name.local.as_ref() {
                "meta" => {
                    // Policies apply from where they are on (see csp.rs)
                    if is_csp_meta(node)
                        && let Some(content) = get_node_attr(node, "content")
                    {
                        session.content_security.add_meta(&content);
                    }
                    if let Some(meta_attr_http_equiv_value) = get_node_attr(node, "http-equiv") {
                        let meta_attr_http_equiv_value: &str = &meta_attr_http_equiv_value;
                        if meta_attr_http_equiv_value.eq_ignore_ascii_case("refresh")
                            || meta_attr_http_equiv_value.eq_ignore_ascii_case("location")
                        {
                            // Remove http-equiv attributes from META nodes if they're able to control the page
                            set_node_attr(node, "http-equiv", None);
                        }
                    }
                }
                "link" => {
                    let link_node_types: Vec<LinkType> =
                        parse_link_type(&get_node_attr(node, "rel").unwrap_or(String::from("")));

                    // Hints are of no use without a network, and would've reached out to one
                    let is_hint = get_node_attr(node, "rel")
                        .unwrap_or_default()
                        .split_whitespace()
                        .any(|rel| {
                            [
                                "dns-prefetch",
                                "modulepreload",
                                "preconnect",
                                "prefetch",
                                "preload",
                            ]
                            .iter()
                            .any(|hint| rel.eq_ignore_ascii_case(hint))
                        });

                    if session.options.replay && is_hint {
                        set_node_attr(node, "href", None);
                    } else if link_node_types.contains(&LinkType::Favicon)
                        || link_node_types.contains(&LinkType::AppleTouchIcon)
                    {
                        // Find and resolve LINK's href attribute
                        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                            if !session.options.remove_images && !link_attr_href_value.is_empty() {
                                retrieve_and_embed_asset(
                                    session,
                                    document_url,
                                    node,
                                    "href",
                                    &link_attr_href_value,
                                );
                            } else {
                                set_node_attr(node, "href", None);
                            }
                        }
                    } else if link_node_types.contains(&LinkType::Stylesheet) {
                        // Resolve LINK's href attribute
                        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                            if session.options.remove_styles {
                                set_node_attr(node, "href", None);
                                // Wipe integrity attribute
                                set_node_attr(node, "integrity", None);
                            } else if !link_attr_href_value.is_empty() {
                                retrieve_and_embed_asset(
                                    session,
                                    document_url,
                                    node,
                                    "href",
                                    &link_attr_href_value,
                                );
                                name_by_nonce(session, node, Destination::Style);
                            }
                        }
                    } else if link_node_types.contains(&LinkType::Manifest) {
                        // Resolve LINK's href attribute
                        if let Some(link_attr_href_value) = get_node_attr(node, "href")
                            && !link_attr_href_value.is_empty()
                        {
                            retrieve_and_embed_asset(
                                session,
                                document_url,
                                node,
                                "href",
                                &link_attr_href_value,
                            );
                        }
                    } else if link_node_types.contains(&LinkType::Preload)
                        || link_node_types.contains(&LinkType::DnsPrefetch)
                    {
                        // Since all resources are embedded as data URLs, preloading and prefetching are not necessary
                        set_node_attr(node, "rel", None);
                    } else {
                        // Make sure that all other LINKs' href attributes are full URLs
                        if let Some(link_attr_href_value) = get_node_attr(node, "href") {
                            let href_full_url: Url =
                                resolve_url(document_url, &link_attr_href_value);
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

                        if !session.options.remove_images && !body_attr_background_value.is_empty()
                        {
                            retrieve_and_embed_asset(
                                session,
                                document_url,
                                node,
                                "background",
                                &body_attr_background_value,
                            );
                        }
                    }
                }
                "img" => {
                    // Images that scripts load on their own are what browsers are there for (-B),
                    // attributes that only scripts know the meaning of get left alone
                    let img_src: Option<String> = get_node_attr(node, "src");

                    if session.options.remove_images {
                        // Put empty image into src attribute
                        if img_src.is_some() {
                            set_node_attr(node, "src", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                        }
                    } else if img_src.clone().unwrap_or_default().is_empty() {
                        // Add empty src attribute
                        set_node_attr(node, "src", Some("".to_string()));
                    } else {
                        // Add data URL src attribute
                        let img_src: String = img_src.unwrap_or_default();

                        if session.options.output_format != MonolithOutputFormat::MHTML {
                            retrieve_and_embed_asset(session, document_url, node, "src", &img_src);
                        } else {
                            let img_src_full_url: Url = resolve_url(document_url, &img_src);
                            if session
                                .is_allowed(&img_src_full_url, &Request::to(Destination::Image))
                            {
                                set_node_attr(node, "src", Some(img_src_full_url.to_string()));
                                session.log_asset_url_as(
                                    &img_src_full_url,
                                    RequestMode::of_attribute(
                                        get_node_attr(node, "crossorigin").as_deref(),
                                    ),
                                );
                            } else {
                                set_node_attr(node, "src", None);
                            }
                        }
                    }

                    // Resolve srcset attribute
                    if let Some(img_srcset) = get_node_attr(node, "srcset")
                        && !img_srcset.is_empty()
                    {
                        let mode = RequestMode::of_attribute(
                            get_node_attr(node, "crossorigin").as_deref(),
                        );
                        let resolved_srcset: String =
                            embed_srcset_as(session, document_url, &img_srcset, mode);
                        set_node_attr(node, "srcset", Some(resolved_srcset));
                    }
                }
                "input" => {
                    if let Some(input_attr_type_value) = get_node_attr(node, "type")
                        && input_attr_type_value.eq_ignore_ascii_case("image")
                        && let Some(input_attr_src_value) = get_node_attr(node, "src")
                    {
                        if session.options.remove_images || input_attr_src_value.is_empty() {
                            let value = if input_attr_src_value.is_empty() {
                                ""
                            } else {
                                EMPTY_IMAGE_DATA_URL
                            };
                            set_node_attr(node, "src", Some(value.to_string()));
                        } else {
                            retrieve_and_embed_asset(
                                session,
                                document_url,
                                node,
                                "src",
                                &input_attr_src_value,
                            );
                        }
                    }
                }
                "svg" => {
                    if session.options.remove_images {
                        // Remove all children
                        node.children.borrow_mut().clear();
                    }
                }
                "image" => {
                    let attr_names: [&str; 2] = ["href", "xlink:href"];

                    for attr_name in attr_names.into_iter() {
                        if let Some(image_attr_href_value) = get_node_attr(node, attr_name) {
                            if session.options.remove_images {
                                set_node_attr(node, attr_name, None);
                            } else {
                                retrieve_and_embed_asset(
                                    session,
                                    document_url,
                                    node,
                                    attr_name,
                                    &image_attr_href_value,
                                );
                            }
                        }
                    }
                }
                "use" => {
                    let attr_names: [&str; 2] = ["href", "xlink:href"];
                    // USE elements tend to point at the same symbol with both attributes, the
                    // contents only have to be brought in once
                    let mut inlined_svg: Option<InlinedSvg> = None;

                    for attr_name in attr_names.into_iter() {
                        if let Some(use_attr_href_value) = get_node_attr(node, attr_name) {
                            if session.options.remove_images {
                                set_node_attr(node, attr_name, None);
                            } else if use_attr_href_value.clone().starts_with('#') {
                                // Relative symbol that resolves to its own URL; keep as-is.
                                set_node_attr(node, attr_name, Some(use_attr_href_value));
                            } else {
                                let image_asset_url: Url =
                                    resolve_url(document_url, &use_attr_href_value);
                                let image_asset_url_without_fragment: Url =
                                    clean_url(image_asset_url.clone());

                                // The other attribute has already brought in this very SVG,
                                // possibly by another URL that leads to the same one
                                if let Some(inlined) = &inlined_svg
                                    && inlined.fragment.as_deref() == image_asset_url.fragment()
                                    && (inlined.requested_url == image_asset_url_without_fragment
                                        || inlined.final_url == image_asset_url_without_fragment)
                                {
                                    if let Some(fragment) = &inlined.fragment {
                                        set_node_attr(
                                            node,
                                            attr_name,
                                            Some(format!("#{}", fragment)),
                                        );
                                    }
                                    continue;
                                }

                                if session.is_expanding(&image_asset_url) {
                                    set_node_attr(node, attr_name, None);
                                    continue;
                                }

                                // Browsers only use what's of the page's own origin, they don't
                                // even ask for anything else (see cors.rs)
                                if !is_usable_by_use(session, &image_asset_url) {
                                    set_node_attr(node, attr_name, None);
                                    continue;
                                }

                                match session.retrieve_asset_for(
                                    document_url,
                                    &image_asset_url,
                                    &Request::to(Destination::Image),
                                ) {
                                    // Not even after having been redirected elsewhere
                                    Ok((_, final_url, _, _))
                                        if !is_usable_by_use(session, &final_url) =>
                                    {
                                        set_node_attr(node, attr_name, None);
                                    }
                                    Ok((data, final_url, media_type, charset)) => {
                                        if media_type == "image/svg+xml" {
                                            let final_url_without_fragment: Url =
                                                clean_url(final_url.clone());

                                            // This URL may be another name for what the other
                                            // attribute has already brought in
                                            if let Some(inlined) = &inlined_svg
                                                && inlined.fragment.as_deref()
                                                    == image_asset_url.fragment()
                                                && inlined.final_url == final_url_without_fragment
                                            {
                                                if let Some(fragment) = &inlined.fragment {
                                                    set_node_attr(
                                                        node,
                                                        attr_name,
                                                        Some(format!("#{}", fragment)),
                                                    );
                                                }
                                                continue;
                                            }

                                            // Keep the SVG being expanded until its contents have
                                            // been walked, so that it can't use itself
                                            if session.is_expanding(&final_url) {
                                                set_node_attr(node, attr_name, None);
                                                continue;
                                            }

                                            let alias = session.enter_expansion_alias(
                                                &image_asset_url,
                                                &final_url,
                                            );
                                            let Some(expansion_key) =
                                                session.enter_expansion(&final_url)
                                            else {
                                                if let Some(key) = alias {
                                                    session.leave_expansion_alias(&key);
                                                }
                                                set_node_attr(node, attr_name, None);
                                                continue;
                                            };
                                            expansions_to_leave.push((expansion_key, alias));

                                            // Parse SVG
                                            let svg_dom: RcDom = parse_document(
                                                RcDom::default(),
                                                Default::default(),
                                            )
                                            .from_utf8()
                                            .read_from(&mut data.as_slice())
                                            .unwrap();

                                            if image_asset_url.fragment().is_some() {
                                                // Take only that one #fragment symbol from SVG and replace this image|use with that node
                                                let single_symbol_node = create_element(
                                                    &svg_dom,
                                                    QualName::new(
                                                        None,
                                                        ns!(),
                                                        LocalName::from("symbol"),
                                                    ),
                                                    vec![],
                                                );
                                                for symbol_node in find_nodes(
                                                    &svg_dom.document,
                                                    vec!["html", "body", "svg", "defs", "symbol"],
                                                )
                                                .iter()
                                                {
                                                    if get_node_attr(symbol_node, "id")
                                                        .unwrap_or_default()
                                                        == image_asset_url.fragment().unwrap()
                                                    {
                                                        svg_dom.reparent_children(
                                                            symbol_node,
                                                            &single_symbol_node,
                                                        );
                                                        set_node_attr(
                                                            &single_symbol_node,
                                                            "id",
                                                            Some(
                                                                image_asset_url
                                                                    .fragment()
                                                                    .unwrap()
                                                                    .to_string(),
                                                            ),
                                                        );

                                                        set_node_attr(
                                                            node,
                                                            attr_name,
                                                            Some(format!(
                                                                "#{}",
                                                                image_asset_url.fragment().unwrap()
                                                            )),
                                                        );

                                                        break;
                                                    }
                                                }

                                                node.children
                                                    .borrow_mut()
                                                    .push(single_symbol_node.clone());
                                            } else {
                                                // Replace this image|use with whole DOM of that SVG file
                                                if let Some(svg_node) = find_nodes(
                                                    &svg_dom.document,
                                                    vec!["html", "body", "svg"],
                                                )
                                                .first()
                                                {
                                                    svg_dom.reparent_children(svg_node, node);
                                                }
                                                // TODO: decide if we resort to using data URL here or stick with embedding the DOM
                                            }

                                            // What got brought in is relative to where it came from
                                            child_document_url = Some(final_url.clone());
                                            inlined_svg = Some(InlinedSvg {
                                                requested_url: image_asset_url_without_fragment,
                                                final_url: final_url_without_fragment,
                                                fragment: image_asset_url
                                                    .fragment()
                                                    .map(str::to_string),
                                            });
                                        } else {
                                            // It's likely a raster image; embed it as data URL
                                            let image_asset_data: String = create_data_url(
                                                &media_type,
                                                &charset,
                                                data,
                                                &final_url,
                                                None,
                                            );
                                            set_node_attr(node, attr_name, Some(image_asset_data));
                                        }
                                    }
                                    Err(_) => {
                                        set_node_attr(
                                            node,
                                            attr_name,
                                            Some(image_asset_url.to_string()),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                "source" => {
                    let parent_node = get_parent_node(node);
                    let parent_node_name: &str = get_node_name(&parent_node).unwrap_or_default();

                    if let Some(source_src) = get_node_attr(node, "src") {
                        if session.options.output_format != MonolithOutputFormat::MHTML {
                            retrieve_and_embed_asset(
                                session,
                                document_url,
                                node,
                                "src",
                                &source_src,
                            );
                        } else {
                            let source_src_full_url: Url = resolve_url(document_url, &source_src);
                            if session
                                .is_allowed(&source_src_full_url, &Request::to(Destination::Media))
                            {
                                set_node_attr(node, "src", Some(source_src_full_url.to_string()));
                                session.log_asset_url_as(
                                    &source_src_full_url,
                                    source_request_mode(node),
                                );
                            } else {
                                set_node_attr(node, "src", None);
                            }
                        }
                    }

                    if let Some(source_attr_srcset_value) = get_node_attr(node, "srcset")
                        && parent_node_name == "picture"
                        && !source_attr_srcset_value.is_empty()
                    {
                        if session.options.remove_images {
                            set_node_attr(node, "srcset", Some(EMPTY_IMAGE_DATA_URL.to_string()));
                        } else {
                            let resolved_srcset: String = embed_srcset_as(
                                session,
                                document_url,
                                &source_attr_srcset_value,
                                source_request_mode(node),
                            );
                            set_node_attr(node, "srcset", Some(resolved_srcset));
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
                            if session.options.remove_js {
                                // Replace with empty JS call to preserve original behavior
                                set_node_attr(node, "href", Some("javascript:;".to_string()));
                            }
                        } else if !anchor_attr_href_value.starts_with('#') {
                            let href_full_url: Url =
                                resolve_url(document_url, &anchor_attr_href_value);
                            if let Some(link) = session.link_to(&href_full_url) {
                                // Leads to some other document that gets saved
                                set_node_attr(node, "href", Some(link));
                            } else if !is_url_and_has_protocol(&anchor_attr_href_value) {
                                // Don't touch mailto: links or hrefs which begin with a hash sign
                                set_node_attr(node, "href", Some(href_full_url.to_string()));
                            }
                        }
                    }
                }
                "script" => {
                    if session.options.remove_js {
                        // TODO: remove this whole node from DOM instead of messing with it
                        // Empty contents
                        node.children.borrow_mut().clear();
                        // Remove src attribute
                        set_node_attr(node, "src", None);
                        // Wipe integrity attribute
                        set_node_attr(node, "integrity", None);
                    } else {
                        // Read values of integrity and src attributes
                        let script_src: &str = &get_node_attr(node, "src").unwrap_or_default();
                        let is_executable = is_executable_script(node);
                        let nonce: Option<String> = get_node_attr(node, "nonce");
                        let integrity: Option<String> = get_node_attr(node, "integrity");
                        let request = Request {
                            destination: Destination::Script,
                            nonce: nonce.as_deref(),
                            integrity: integrity.as_deref(),
                        };

                        let is_module = get_node_attr(node, "type")
                            .is_some_and(|value| value.eq_ignore_ascii_case("module"));

                        if !script_src.is_empty()
                            && !session.is_allowed(&resolve_url(document_url, script_src), &request)
                        {
                            // What policies don't allow doesn't get to run (see csp.rs)
                            set_node_attr(node, "src", None);
                            set_node_attr(node, "integrity", None);
                        } else if !script_src.is_empty() && session.options.replay && is_module {
                            // Everything that imports it has to end up with the same instance
                            // of it, so it can't become a separate copy of itself in here
                            session.saved_content.blob.insert(Destination::Script);
                            let script_url = resolve_url(document_url, script_src);
                            let integrity = get_node_attr(node, "integrity").unwrap_or_default();
                            let is_intact = integrity.is_empty()
                                || session
                                    .original_data(&script_url)
                                    .is_some_and(|data| check_integrity(data, &integrity));
                            let import = script_url.as_str().replace('"', "%22");
                            set_node_attr(
                                node,
                                "src",
                                is_intact.then(|| {
                                    format!(
                                        "data:text/javascript;charset=utf-8,import%20%22{}%22",
                                        import.replace('%', "%25").replace('#', "%23")
                                    )
                                }),
                            );
                            set_node_attr(node, "integrity", None);
                        } else if !script_src.is_empty() {
                            if session.options.output_format != MonolithOutputFormat::MHTML {
                                retrieve_and_embed_asset(
                                    session,
                                    document_url,
                                    node,
                                    "src",
                                    script_src,
                                );
                            } else {
                                let script_src_full_url: Url =
                                    resolve_url(document_url, script_src);
                                set_node_attr(node, "src", Some(script_src_full_url.to_string()));
                                session.log_asset_url_as(
                                    &script_src_full_url,
                                    RequestMode::of_script(
                                        get_node_attr(node, "type").as_deref(),
                                        get_node_attr(node, "crossorigin").as_deref(),
                                    ),
                                );
                            }
                        } else if is_executable
                            && !session.content_security.allows_inline(
                                Inline::Script,
                                nonce.as_deref(),
                                &node_text_content(node),
                            )
                        {
                            if !session.options.silent {
                                print_error_message(&format!(
                                    "{} (inline script blocked by Content Security Policy)",
                                    document_url
                                ));
                            }
                            node.children.borrow_mut().clear();
                        } else {
                            for child_node in node.children.borrow_mut().iter_mut() {
                                if let NodeData::Text { ref contents } = child_node.data {
                                    let mut tendril = contents.borrow_mut();
                                    // Source maps don't come along (see source_maps.rs)
                                    let code =
                                        remove_source_map_reference_from_text(tendril.as_ref())
                                            .to_string();
                                    let code = match session.options.replay {
                                        true => rewrite_imports(&code, document_url).code,
                                        false => code,
                                    };
                                    tendril.clear();
                                    tendril.push_slice(&code);
                                }
                            }
                        }
                        if is_executable {
                            name_by_nonce(session, node, Destination::Script);
                        }
                    }
                }
                "style" => {
                    // What browsers have made out of constructed stylesheets isn't up to policies
                    let is_adopted = session.adopted_style_marker.is_some()
                        && get_node_attr(node, "data-monolith-adopted")
                            == session.adopted_style_marker;
                    set_node_attr(node, "data-monolith-adopted", None);
                    if session.options.remove_styles {
                        // Empty inner content of STYLE tags
                        node.children.borrow_mut().clear();
                    } else if !is_adopted
                        && !session.content_security.allows_inline(
                            Inline::Style,
                            get_node_attr(node, "nonce").as_deref(),
                            &node_text_content(node),
                        )
                    {
                        if !session.options.silent {
                            print_error_message(&format!(
                                "{} (inline style blocked by Content Security Policy)",
                                document_url
                            ));
                        }
                        node.children.borrow_mut().clear();
                    } else {
                        for child_node in node.children.borrow_mut().iter_mut() {
                            if let NodeData::Text { ref contents } = child_node.data {
                                let mut tendril = contents.borrow_mut();
                                let replacement =
                                    embed_css(session, document_url, tendril.as_ref());
                                tendril.clear();
                                tendril.push_slice(&replacement);
                            }
                        }
                        name_by_nonce(session, node, Destination::Style);
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
                "iframe" if get_node_attr(node, "srcdoc").is_some() => {
                    let srcdoc = get_node_attr(node, "srcdoc").unwrap_or_default();
                    set_node_attr(
                        node,
                        "srcdoc",
                        Some(embed_srcdoc(session, document_url, &srcdoc)),
                    );
                    // Browsers go with what's in srcdoc, src never gets loaded
                    set_node_attr(node, "src", None);

                    // Sandbox iframe if JS is disabled
                    if session.options.remove_js {
                        set_node_attr(node, "sandbox", Some("allow-forms".to_string()));
                    }
                }
                "frame" | "iframe" => {
                    if let Some(frame_attr_src_value) = get_node_attr(node, "src") {
                        if session.options.output_format != MonolithOutputFormat::MHTML {
                            // Ignore (i)frames with empty source (they cause infinite loops)
                            if !frame_attr_src_value.trim().is_empty() {
                                retrieve_and_embed_asset(
                                    session,
                                    document_url,
                                    node,
                                    "src",
                                    &frame_attr_src_value,
                                );
                            }
                        } else {
                            let frame_src_full_url: Url =
                                resolve_url(document_url, &frame_attr_src_value);
                            if session
                                .is_allowed(&frame_src_full_url, &Request::to(Destination::Frame))
                            {
                                set_node_attr(node, "src", Some(frame_src_full_url.to_string()));
                                session.log_asset_url(&frame_src_full_url);
                            } else {
                                set_node_attr(node, "src", None);
                            }
                        }

                        // Sandbox iframe if JS is disabled
                        if name.local.as_ref() == "iframe" && session.options.remove_js {
                            set_node_attr(node, "sandbox", Some("allow-forms".to_string()));
                        }
                    }
                }
                "audio" => {
                    // Embed audio source
                    if let Some(audio_attr_src_value) = get_node_attr(node, "src") {
                        if session.options.output_format != MonolithOutputFormat::MHTML {
                            retrieve_and_embed_asset(
                                session,
                                document_url,
                                node,
                                "src",
                                &audio_attr_src_value,
                            );
                        } else {
                            let audio_src_full_url: Url =
                                resolve_url(document_url, &audio_attr_src_value);
                            if session
                                .is_allowed(&audio_src_full_url, &Request::to(Destination::Media))
                            {
                                set_node_attr(node, "src", Some(audio_src_full_url.to_string()));
                                session.log_asset_url_as(
                                    &audio_src_full_url,
                                    RequestMode::of_attribute(
                                        get_node_attr(node, "crossorigin").as_deref(),
                                    ),
                                );
                            } else {
                                set_node_attr(node, "src", None);
                            }
                        }
                    }
                }
                "video" => {
                    // Embed video source
                    if let Some(video_attr_src_value) = get_node_attr(node, "src") {
                        if session.options.output_format != MonolithOutputFormat::MHTML {
                            retrieve_and_embed_asset(
                                session,
                                document_url,
                                node,
                                "src",
                                &video_attr_src_value,
                            );
                        } else {
                            let video_src_full_url: Url =
                                resolve_url(document_url, &video_attr_src_value);
                            if session
                                .is_allowed(&video_src_full_url, &Request::to(Destination::Media))
                            {
                                set_node_attr(node, "src", Some(video_src_full_url.to_string()));
                                session.log_asset_url_as(
                                    &video_src_full_url,
                                    RequestMode::of_attribute(
                                        get_node_attr(node, "crossorigin").as_deref(),
                                    ),
                                );
                            } else {
                                set_node_attr(node, "src", None);
                            }
                        }
                    }

                    // Embed poster images
                    if let Some(video_attr_poster_value) = get_node_attr(node, "poster") {
                        // Skip posters with empty source
                        if !video_attr_poster_value.is_empty() {
                            if session.options.remove_images {
                                set_node_attr(
                                    node,
                                    "poster",
                                    Some(EMPTY_IMAGE_DATA_URL.to_string()),
                                );
                            } else {
                                if session.options.output_format != MonolithOutputFormat::MHTML {
                                    retrieve_and_embed_asset(
                                        session,
                                        document_url,
                                        node,
                                        "poster",
                                        &video_attr_poster_value,
                                    );
                                } else {
                                    let video_poster_full_url: Url =
                                        resolve_url(document_url, &video_attr_poster_value);
                                    if session.is_allowed(
                                        &video_poster_full_url,
                                        &Request::to(Destination::Image),
                                    ) {
                                        set_node_attr(
                                            node,
                                            "poster",
                                            Some(video_poster_full_url.to_string()),
                                        );
                                        session.log_asset_url(&video_poster_full_url);
                                    } else {
                                        set_node_attr(node, "poster", None);
                                    }
                                }
                            }
                        }
                    }
                }
                "noscript" => {
                    // Embed assets of NOSCRIPT node contents
                    process_noscript_contents(node, |fragment| {
                        // This fragment is part of the current document, not a document of its
                        // own, so it doesn't need its own expansion scope
                        for child_node in fragment.document.children.borrow().iter() {
                            walk(session, document_url, child_node);
                        }
                    });
                }
                _ => {}
            }

            // Process style attributes
            if session.options.remove_styles {
                // Get rid of style attributes
                set_node_attr(node, "style", None);
            } else {
                // Embed URLs found within the style attribute of this node
                if let Some(node_attr_style_value) = get_node_attr(node, "style") {
                    if session.content_security.allows_inline(
                        Inline::StyleAttribute,
                        None,
                        &node_attr_style_value,
                    ) {
                        let embedded_style =
                            embed_css(session, document_url, &node_attr_style_value);
                        if session.names_elements {
                            session
                                .saved_content
                                .add_inline(Inline::StyleAttribute, &embedded_style);
                        }
                        set_node_attr(node, "style", Some(embedded_style));
                    } else {
                        // Browsers ignore what policies don't allow
                        set_node_attr(node, "style", None);
                    }
                }
            }

            // Event handlers have to be allowed by policies, the saved document has to allow
            // those that are (see csp.rs)
            if !session.options.remove_js {
                let mut attrs_mut = attrs.borrow_mut();
                let content_security = &session.content_security;
                attrs_mut.retain(|attr| {
                    !attr_is_event_handler(&attr.name.local)
                        || content_security.allows_inline(
                            Inline::ScriptAttribute,
                            None,
                            &attr.value,
                        )
                });
                if session.names_elements {
                    for attr in attrs_mut.iter() {
                        if attr_is_event_handler(&attr.name.local) {
                            session
                                .saved_content
                                .add_inline(Inline::ScriptAttribute, &attr.value);
                        }
                    }
                }
            }

            // Strip all JS from document
            if session.options.remove_js {
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
            let child_document_url: &Url = child_document_url.as_ref().unwrap_or(document_url);
            for child_node in node.children.borrow().iter() {
                walk(session, child_document_url, child_node);
            }

            for (expansion_key, alias) in expansions_to_leave {
                session.leave_expansion(&expansion_key);
                if let Some(key) = alias {
                    session.leave_expansion_alias(&key);
                }
            }
        }
        _ => {
            // Note: in case of options.remove_js being set to true, there's no need to worry about
            //       getting rid of comments that may contain scripts, e.g. <!--[if IE]><script>...
            //       since that's not part of W3C standard and therefore gets ignored
            //       by browsers other than IE [5, 9]
        }
    }
}
