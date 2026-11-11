use markup5ever_rcdom::{Handle, Node, NodeData, RcDom};
use std::collections::HashMap;
use std::rc::Rc;

use crate::html::{
    create_html_element, create_text_node, find_nodes, get_node_attr, get_node_name,
    get_parent_node, is_favicon, retain_node_attrs, set_node_attr,
};

/// Anything shorter than this isn't considered to be an article
const MIN_ARTICLE_LENGTH: usize = 250;
/// Anything shorter than this isn't considered to be a paragraph
const MIN_PARAGRAPH_LENGTH: usize = 25;
/// Ancestors of a paragraph that get credit for it
const SCORED_ANCESTORS: usize = 3;
/// Ancestors scoring this much of what the best element got take its place
const PROMOTION_RATIO: f64 = 0.75;
const SIBLING_MIN_SCORE: f64 = 10.0;
const SIBLING_SCORE_RATIO: f64 = 0.2;
const SIBLING_MIN_PARAGRAPH_LENGTH: usize = 80;
const SIBLING_MAX_LINK_DENSITY: f64 = 0.25;
/// Short blocks made up mostly of links are lists of related articles and such
const CLUTTER_MAX_LENGTH: usize = 500;
const CLUTTER_MIN_LINK_DENSITY: f64 = 0.5;

/// Elements that never make it into the simplified document
const REMOVED_ELEMENTS: [&str; 22] = [
    "aside", "button", "canvas", "dialog", "embed", "footer", "form", "frame", "frameset",
    "iframe", "input", "link", "menu", "meta", "nav", "noscript", "object", "script", "select",
    "style", "template", "textarea",
];
const REMOVED_ROLES: [&str; 8] = [
    "alert",
    "alertdialog",
    "banner",
    "complementary",
    "contentinfo",
    "dialog",
    "menu",
    "navigation",
];
/// Words in class names and IDs that suggest the element isn't a part of the article
const UNLIKELY_WORDS: [&str; 33] = [
    "ad",
    "advert",
    "advertisement",
    "banner",
    "breadcrumb",
    "citation",
    "comment",
    "cookie",
    "disqus",
    "footer",
    "header",
    "masthead",
    "menu",
    "modal",
    "nav",
    "navbar",
    "navigation",
    "newsletter",
    "pagination",
    "popup",
    "promo",
    "related",
    "share",
    "sharing",
    "sidebar",
    "social",
    "sponsor",
    "sponsored",
    "subscribe",
    "subscription",
    "tag",
    "toolbar",
    "widget",
];
/// Words in class names and IDs that suggest the element is (a part of) the article
const LIKELY_WORDS: [&str; 10] = [
    "article", "blog", "body", "content", "entry", "main", "markdown", "post", "prose", "story",
];
/// Characters that give away utility classes, like the ones of Tailwind, e.g. "md:p-4", "w-1/2",
/// or "[--top:var(--header-h)]": those describe looks, and say nothing about what the element is
const UTILITY_CLASS_CHARS: [char; 8] = ['!', '(', '.', '/', ':', '=', '@', '['];
/// Elements that contain a paragraph's worth of text on their own
const TEXT_BLOCKS: [&str; 4] = ["blockquote", "p", "pre", "td"];
/// Elements that stop a DIV from being treated as a paragraph
const BLOCK_ELEMENTS: [&str; 12] = [
    "article",
    "blockquote",
    "div",
    "dl",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "ul",
    "h1",
    "h2",
];
const MEDIA_ELEMENTS: [&str; 6] = ["audio", "img", "picture", "pre", "svg", "video"];
/// Elements that are of no use once they end up with nothing in them
const REMOVED_IF_EMPTY: [&str; 9] = ["div", "h1", "h2", "h3", "li", "ol", "p", "section", "ul"];
const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
const CLUTTER_CANDIDATES: [&str; 5] = ["div", "ol", "section", "table", "ul"];
/// What's left of widgets that were supposed to get brought to life by JavaScript
const PLACEHOLDER_TEXTS: [&str; 3] = ["loading", "loading...", "loading\u{2026}"];
/// Labels of blocks with links to other articles, which outlive their links getting removed
const LEFTOVER_LABELS: [&str; 9] = [
    "from your site articles",
    "keep reading",
    "more from this author",
    "read more",
    "read next",
    "recommended",
    "related articles",
    "related articles around the web",
    "you may also like",
];
/// Words in class names and IDs of what's right below the title, summing up the article
const SUBTITLE_WORDS: [&str; 7] = [
    "dek",
    "deck",
    "standfirst",
    "subhead",
    "subheadline",
    "subheading",
    "subtitle",
];
const BYLINE_WORDS: [&str; 2] = ["author", "byline"];
const MAX_BYLINE_LENGTH: usize = 100;
const MAX_SUBTITLE_LENGTH: usize = 300;
/// How many elements away from the title a subtitle can be
const MAX_SUBTITLE_DISTANCE: usize = 12;
/// What usually separates the title of a page from the name of the website
const TITLE_SEPARATORS: [&str; 5] = [" - ", " | ", " \u{b7} ", " \u{2013} ", " \u{2014} "];
const KEPT_ATTRIBUTES: [&str; 12] = [
    "alt", "cite", "colspan", "datetime", "dir", "href", "id", "lang", "rowspan", "src", "srcset",
    "title",
];

/// Code with lines shorter than this fits on paper at its usual size
const MIN_MEASURED_COLUMNS: usize = 60;

const STYLES: &str = "\
:root { color-scheme: light dark; }
body { font: 1.125rem/1.6 system-ui, sans-serif; margin: 2rem auto; max-width: 40rem; padding: 0 1rem 10rem; }
h1, h2, h3, h4 { line-height: 1.25; }
img, video { height: auto; max-width: 100%; }
video, figure img { display: block; width: 100%; }
figure { margin: 1rem 0; }
pre { overflow-x: auto; }
pre, code { font-size: 0.9em; }
blockquote { border-left: 0.25rem solid currentcolor; margin-left: 0; opacity: 0.8; padding-left: 1rem; }
table { border-collapse: collapse; display: block; overflow-x: auto; }
td, th { border: 1px solid currentcolor; padding: 0.25rem 0.5rem; }
@media print {
  pre { font-size: min(0.9em, calc(min(100vw - 2rem, 40rem) / (var(--columns, 1) * 0.62))); overflow: visible; }
  pre code { font-size: inherit; }
  details::details-content { content-visibility: visible; display: block; height: auto; }
}
";

/// Turns the document into a simplified version of itself, the way reader modes of browsers do:
/// just the article and its title, without anything else that surrounds it on the page.
/// Returns false if hardly any text is left, which is what pages that rely on JavaScript end up with.
pub fn simplify_document(dom: &RcDom, add_styles: bool) -> bool {
    let html_nodes = find_nodes(&dom.document, vec!["html", "body"]);
    let Some(body) = html_nodes.first() else {
        return false; // Framesets don't have any contents to work with
    };
    let head_nodes = find_nodes(&dom.document, vec!["html", "head"]);
    let Some(head) = head_nodes.first() else {
        return false;
    };

    let title: Option<String> = page_title(head, body);
    let subtitle: Option<String> = find_subtitle(body);
    let byline: Option<String> = find_byline(head, body);

    place_streamed_contents(body);
    remove_distractions(body);

    // Pages with nothing that resembles an article simply get whatever is left of them
    if let Some(article) = find_article(body) {
        set_children(body, article);
    }
    for root in find_nodes(&dom.document, vec!["html"]).iter() {
        retain_node_attrs(root, &KEPT_ATTRIBUTES);
    }
    retain_node_attrs(body, &KEPT_ATTRIBUTES);
    tidy_children(body);
    let has_text: bool = text_length(body) >= MIN_ARTICLE_LENGTH;

    let mut has_new_title = false;
    if let Some(title) = title
        && !has_descendant(body, &["h1"])
    {
        let heading = create_html_element(dom, "h1");
        set_children(&heading, vec![create_text_node(&title)]);
        let mut children = vec![heading];
        children.extend(body.children.take());
        set_children(body, children);
        has_new_title = true;
    }
    add_below_title(dom, body, [subtitle, byline], has_new_title);

    if add_styles {
        measure_code(body);
    }
    simplify_head(dom, head, add_styles);

    has_text
}

fn set_children(parent: &Handle, children: Vec<Handle>) {
    for child in children.iter() {
        child.parent.set(Some(Rc::downgrade(parent)));
    }
    *parent.children.borrow_mut() = children;
}

fn is_element(node: &Handle, names: &[&str]) -> bool {
    get_node_name(node).is_some_and(|name| names.iter().any(|n| name.eq_ignore_ascii_case(n)))
}

fn has_descendant(node: &Handle, names: &[&str]) -> bool {
    node.children
        .borrow()
        .iter()
        .any(|child| is_element(child, names) || has_descendant(child, names))
}

fn collect_text(node: &Handle, text: &mut String) {
    if let NodeData::Text { ref contents } = node.data {
        text.push_str(&contents.borrow());
    }
    for child in node.children.borrow().iter() {
        collect_text(child, text);
    }
}

/// Text of the element, with all the whitespace being collapsed
fn inner_text(node: &Handle) -> String {
    let mut text = String::new();
    collect_text(node, &mut text);
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn text_length(node: &Handle) -> usize {
    inner_text(node).chars().count()
}

fn link_text_length(node: &Handle) -> usize {
    if is_element(node, &["a"]) {
        return text_length(node);
    }
    node.children.borrow().iter().map(link_text_length).sum()
}

/// How much of the element's text belongs to links, from 0 to 1
fn link_density(node: &Handle) -> f64 {
    match text_length(node) {
        0 => 0.0,
        length => link_text_length(node) as f64 / length as f64,
    }
}

/// Words that class names and the ID of the element are made of
fn name_words(node: &Handle) -> Vec<String> {
    let class = get_node_attr(node, "class").unwrap_or_default();
    let id = get_node_attr(node, "id").unwrap_or_default();

    format!("{class} {id}")
        .to_lowercase()
        .split_whitespace()
        .filter(|name| !name.contains(UTILITY_CLASS_CHARS))
        .flat_map(|name| name.split(['-', '_']))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// Plurals count as well, e.g. "comments" as "comment"
fn has_any_word(words: &[String], wanted: &[&str]) -> bool {
    words.iter().any(|word| {
        let singular = word.strip_suffix('s').unwrap_or(word);
        wanted.contains(&word.as_str()) || wanted.contains(&singular)
    })
}

fn collect_all_elements(node: &Handle, elements: &mut Vec<Handle>) {
    for child in node.children.borrow().iter() {
        if get_node_name(child).is_some() {
            elements.push(child.clone());
            collect_all_elements(child, elements);
        }
    }
}

fn first_h1(node: &Handle) -> Option<Handle> {
    node.children.borrow().iter().find_map(|child| {
        if is_element(child, &["h1"]) {
            Some(child.clone())
        } else {
            first_h1(child)
        }
    })
}

fn without_site_name(title: &str, site_name: &str) -> String {
    let mut title = title.to_string();
    for separator in TITLE_SEPARATORS {
        if let Some(rest) = title.strip_suffix(&format!("{separator}{site_name}")) {
            title = rest.to_string();
        }
        if let Some(rest) = title.strip_prefix(&format!("{site_name}{separator}")) {
            title = rest.to_string();
        }
    }
    title
}

fn page_title(head: &Handle, body: &Handle) -> Option<String> {
    // Unlike TITLE elements, these don't tend to have the name of the website in them
    if let Some(heading) = first_h1(body) {
        let title = inner_text(&heading);
        return (!title.is_empty()).then_some(title);
    }

    let children = head.children.borrow();
    let title = inner_text(children.iter().find(|n| is_element(n, &["title"]))?);
    let site_name: Option<String> = children
        .iter()
        .find(|node| get_node_attr(node, "property").as_deref() == Some("og:site_name"))
        .and_then(|node| get_node_attr(node, "content"));

    let title = match site_name {
        Some(site_name) => without_site_name(&title, &site_name),
        None => title,
    };
    (!title.is_empty()).then_some(title)
}

/// First element whose class names or ID contain any of the given words, and that has fitting text
fn find_text_by_words(node: &Handle, words: &[&str], max_length: usize) -> Option<String> {
    node.children.borrow().iter().find_map(|child| {
        get_node_name(child)?;
        let names = name_words(child);
        let text = inner_text(child);
        if has_any_word(&names, words)
            && !has_any_word(&names, &["comment"])
            && !text.is_empty()
            && text.chars().count() <= max_length
        {
            Some(text)
        } else {
            find_text_by_words(child, words, max_length)
        }
    })
}

/// Has to come right after the title, there's no telling what it is otherwise
fn find_subtitle(body: &Handle) -> Option<String> {
    let mut elements: Vec<Handle> = Vec::new();
    collect_all_elements(body, &mut elements);

    let title = elements.iter().position(|node| is_element(node, &["h1"]))?;
    elements
        .iter()
        .skip(title + 1)
        .take(MAX_SUBTITLE_DISTANCE)
        .find(|node| has_any_word(&name_words(node), &SUBTITLE_WORDS))
        .map(inner_text)
        .filter(|text| !text.is_empty() && text.chars().count() <= MAX_SUBTITLE_LENGTH)
}

/// What's on the page goes first, META tags tend to have the name of the company in them
fn find_byline(head: &Handle, body: &Handle) -> Option<String> {
    find_text_by_words(body, &BYLINE_WORDS, MAX_BYLINE_LENGTH)
        .or_else(|| {
            head.children
                .borrow()
                .iter()
                .find(|node| {
                    is_element(node, &["meta"])
                        && get_node_attr(node, "name").as_deref() == Some("author")
                })
                .and_then(|node| get_node_attr(node, "content"))
                .filter(|author| !author.trim().is_empty())
        })
        .map(|byline| match byline.to_lowercase().starts_with("by ") {
            true => byline,
            false => format!("By {byline}"),
        })
}

/// Unless the article comes with them already, along with a line that sets all of that apart
/// from the article; titles that are a part of the article may be followed by more of its header
fn add_below_title<const N: usize>(
    dom: &RcDom,
    body: &Handle,
    details: [Option<String>; N],
    has_new_title: bool,
) {
    let Some(heading) = first_h1(body) else {
        return;
    };
    let parent = get_parent_node(&heading);
    let text = inner_text(body);

    let mut children: Vec<Handle> = Vec::new();
    for child in parent.children.take() {
        let is_heading = Rc::ptr_eq(&child, &heading);
        children.push(child);
        if is_heading {
            let mut needs_line = has_new_title;
            for detail in details.iter().flatten() {
                let name_only = detail.strip_prefix("By ").unwrap_or(detail);
                if !text.contains(name_only) {
                    let paragraph = create_html_element(dom, "p");
                    set_children(&paragraph, vec![create_text_node(detail)]);
                    children.push(paragraph);
                    needs_line = true;
                }
            }
            if needs_line {
                children.push(create_html_element(dom, "hr"));
            }
        }
    }
    set_children(&parent, children);
}

fn is_hidden(node: &Handle) -> bool {
    let style = get_node_attr(node, "style")
        .unwrap_or_default()
        .replace(' ', "")
        .to_lowercase();

    get_node_attr(node, "hidden").is_some()
        || get_node_attr(node, "aria-hidden").as_deref() == Some("true")
        || style.contains("display:none")
        || style.contains("visibility:hidden")
}

fn is_distraction(node: &Handle) -> bool {
    if is_element(node, &REMOVED_ELEMENTS) || is_hidden(node) {
        return true;
    }

    let role = get_node_attr(node, "role").unwrap_or_default();
    if REMOVED_ROLES.contains(&role.to_lowercase().as_str()) {
        return true;
    }

    let words = name_words(node);
    !is_element(node, &["article", "body", "main"])
        && has_any_word(&words, &UNLIKELY_WORDS)
        && !has_any_word(&words, &LIKELY_WORDS)
}

fn remove_distractions(node: &Handle) {
    node.children
        .borrow_mut()
        .retain(|child| get_node_name(child).is_none() || !is_distraction(child));

    for child in node.children.borrow().iter() {
        remove_distractions(child);
    }
}

/// Starting point for an element's score, based on what it is
fn initial_score(node: &Handle) -> f64 {
    let name = get_node_name(node).unwrap_or_default().to_ascii_lowercase();
    let by_name: f64 = match name.as_str() {
        "article" => 25.0,
        "main" => 10.0,
        "div" | "section" => 5.0,
        "blockquote" | "pre" | "td" => 3.0,
        "address" | "dl" | "ol" | "ul" => -3.0,
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "th" => -5.0,
        _ => 0.0,
    };

    let words = name_words(node);
    let by_names: f64 = if has_any_word(&words, &LIKELY_WORDS) {
        25.0
    } else if has_any_word(&words, &UNLIKELY_WORDS) {
        -25.0
    } else {
        0.0
    };

    by_name + by_names
}

/// Paragraphs and DIVs that are used as ones, since some websites go without P elements
fn is_paragraph(node: &Handle) -> bool {
    is_element(node, &TEXT_BLOCKS)
        || (is_element(node, &["div"])
            && !node
                .children
                .borrow()
                .iter()
                .any(|child| is_element(child, &BLOCK_ELEMENTS)))
}

fn paragraph_score(text: &str) -> f64 {
    let length = text.chars().count() as f64;
    let commas = text.matches(',').count() as f64;
    1.0 + commas + (length / 100.0).min(3.0)
}

/// Paragraphs give points to the elements around them, the closer the element is the more it gets
fn score_paragraphs(
    node: &Handle,
    ancestors: &mut Vec<Handle>,
    scores: &mut HashMap<*const Node, (Handle, f64)>,
) {
    if is_paragraph(node) {
        let text = inner_text(node);
        if text.chars().count() >= MIN_PARAGRAPH_LENGTH {
            let score = paragraph_score(&text);
            for (distance, ancestor) in ancestors.iter().rev().take(SCORED_ANCESTORS).enumerate() {
                let entry = scores
                    .entry(Rc::as_ptr(ancestor))
                    .or_insert_with(|| (ancestor.clone(), initial_score(ancestor)));
                entry.1 += score / (distance + 1) as f64;
            }
        }
    }

    ancestors.push(node.clone());
    for child in node.children.borrow().iter() {
        if get_node_name(child).is_some() {
            score_paragraphs(child, ancestors, scores);
        }
    }
    ancestors.pop();
}

fn collect_elements(node: &Handle, name: &str, elements: &mut Vec<Handle>) {
    for child in node.children.borrow().iter() {
        if is_element(child, &[name]) {
            elements.push(child.clone());
        }
        collect_elements(child, name, elements);
    }
}

/// Takes the node out of the tree. Dropped nodes wipe all of their descendants, even the ones
/// that are still in use elsewhere, so anything worth keeping has to be taken out beforehand.
fn detach(node: &Handle) {
    get_parent_node(node)
        .children
        .borrow_mut()
        .retain(|sibling| !Rc::ptr_eq(sibling, node));
}

/// React sends parts of pages that took longer to render at the very end, hidden, and has a script
/// move them to where they belong: from <div hidden id="S:0"> to <template id="B:0">.
/// Without running that script, all of that would remain hidden, and get thrown out.
fn place_streamed_contents(body: &Handle) {
    let mut templates: Vec<Handle> = Vec::new();
    collect_elements(body, "template", &mut templates);
    let mut blocks: Vec<Handle> = Vec::new();
    collect_elements(body, "div", &mut blocks);

    for template in templates {
        let Some(number) =
            get_node_attr(&template, "id").and_then(|id| id.strip_prefix("B:").map(str::to_string))
        else {
            continue;
        };
        let Some(block) = blocks
            .iter()
            .find(|block| get_node_attr(block, "id").as_deref() == Some(&format!("S:{number}")))
        else {
            continue;
        };

        let parent = get_parent_node(&template);
        let mut children: Vec<Handle> = Vec::new();
        for sibling in parent.children.take() {
            if Rc::ptr_eq(&sibling, &template) {
                children.extend(block.children.take());
            } else {
                children.push(sibling);
            }
        }
        set_children(&parent, children);
        detach(block);
    }
}

/// Pages that have a single ARTICLE holding most of the text have done all the work for us
fn marked_up_article(body: &Handle) -> Option<Handle> {
    let mut articles: Vec<Handle> = Vec::new();
    collect_elements(body, "article", &mut articles);

    match articles.as_slice() {
        [article]
            if text_length(article) >= MIN_ARTICLE_LENGTH
                && text_length(article) * 2 >= text_length(body) =>
        {
            Some(article.clone())
        }
        _ => None,
    }
}

fn find_article(body: &Handle) -> Option<Vec<Handle>> {
    if let Some(article) = marked_up_article(body) {
        detach(&article);
        return Some(vec![article]);
    }

    let mut scores: HashMap<*const Node, (Handle, f64)> = HashMap::new();
    score_paragraphs(body, &mut vec![], &mut scores);

    // Lots of links make for a poor article, no matter how much text there is
    let final_score = |node: &Handle, score: f64| score * (1.0 - link_density(node));
    let mut final_scores: HashMap<*const Node, f64> = HashMap::new();
    for (key, (node, score)) in scores.iter() {
        final_scores.insert(*key, final_score(node, *score));
    }

    let (mut best, mut best_score) = scores
        .iter()
        .map(|(key, (node, _))| (node.clone(), final_scores[key]))
        .max_by(|a, b| a.1.total_cmp(&b.1))?;

    if Rc::ptr_eq(&best, body) || text_length(&best) < MIN_ARTICLE_LENGTH {
        return None;
    }

    // Articles can get split into multiple containers, which have to be taken as a whole
    loop {
        let parent = get_parent_node(&best);
        if Rc::ptr_eq(&parent, body) {
            break;
        }
        match final_scores.get(&Rc::as_ptr(&parent)) {
            Some(score) if *score >= best_score * PROMOTION_RATIO => {
                best = parent;
                best_score = best_score.max(final_scores[&Rc::as_ptr(&best)]);
            }
            _ => break,
        }
    }

    // Parts of the article may be sitting right next to it
    let threshold = SIBLING_MIN_SCORE.max(best_score * SIBLING_SCORE_RATIO);
    let parent = get_parent_node(&best);
    let article: Vec<Handle> = parent
        .children
        .borrow()
        .iter()
        .filter(|sibling| {
            Rc::ptr_eq(sibling, &best)
                || final_scores
                    .get(&Rc::as_ptr(sibling))
                    .is_some_and(|score| *score >= threshold)
                || (is_element(sibling, &["blockquote", "figure", "p", "pre"])
                    && text_length(sibling) >= SIBLING_MIN_PARAGRAPH_LENGTH
                    && link_density(sibling) < SIBLING_MAX_LINK_DENSITY)
        })
        .cloned()
        .collect();

    for node in article.iter() {
        detach(node);
    }

    Some(article)
}

fn is_placeholder(node: &Handle) -> bool {
    let text = inner_text(node).to_lowercase();
    PLACEHOLDER_TEXTS.contains(&text.as_str()) || LEFTOVER_LABELS.contains(&text.as_str())
}

fn has_no_source(node: &Handle) -> bool {
    get_node_attr(node, "src").is_none() && !has_descendant(node, &["source"])
}

fn heading_text_length(node: &Handle) -> usize {
    let mut headings: Vec<Handle> = Vec::new();
    for name in HEADINGS {
        collect_elements(node, name, &mut headings);
    }
    headings.iter().map(text_length).sum()
}

/// Wrappers of headings, no matter how many links those have in them
fn is_just_a_heading(node: &Handle) -> bool {
    let heading_length = heading_text_length(node);
    heading_length > 0 && text_length(node) <= heading_length
}

/// Sections that only link elsewhere, e.g. "Keep reading" with its link to the rest of the blog
fn is_heading_with_links(node: &Handle) -> bool {
    let heading_length = heading_text_length(node);
    let link_length = link_text_length(node);

    heading_length > 0 && link_length > 0 && text_length(node) <= heading_length + link_length
}

fn is_clutter(node: &Handle) -> bool {
    let has_media = is_element(node, &MEDIA_ELEMENTS) || has_descendant(node, &MEDIA_ELEMENTS);

    (is_element(node, &REMOVED_IF_EMPTY) && !has_media && text_length(node) == 0)
        || (is_element(node, &["audio", "video"]) && has_no_source(node))
        || (is_element(node, &["div", "h2", "h3", "h4", "p", "span"])
            && !has_media
            && is_placeholder(node))
        || (is_element(node, &CLUTTER_CANDIDATES)
            && !is_just_a_heading(node)
            && is_heading_with_links(node))
        // Images don't save these: blocks of links to other articles tend to come with thumbnails
        || (is_element(node, &CLUTTER_CANDIDATES)
            && !has_descendant(node, &["pre"])
            && !is_just_a_heading(node)
            && text_length(node) < CLUTTER_MAX_LENGTH
            && link_density(node) > CLUTTER_MIN_LINK_DENSITY)
}

/// Cleans up what's left of the document, innermost elements go first
fn tidy_children(parent: &Handle) {
    let mut children: Vec<Handle> = Vec::new();

    for child in parent.children.take() {
        if matches!(child.data, NodeData::Comment { .. }) {
            continue;
        }
        if get_node_name(&child).is_some() {
            retain_node_attrs(&child, &KEPT_ATTRIBUTES);
            tidy_children(&child);
            if is_clutter(&child) {
                continue;
            }
        }
        children.push(child);
    }

    *parent.children.borrow_mut() = children;
}

/// Printing can't scroll: this is what lets styles shrink code just enough to fit on paper
fn measure_code(body: &Handle) {
    let mut blocks: Vec<Handle> = Vec::new();
    collect_elements(body, "pre", &mut blocks);

    for block in blocks {
        let mut text = String::new();
        collect_text(&block, &mut text);

        let columns = text.lines().map(|line| line.chars().count()).max();
        if let Some(columns) = columns.filter(|columns| *columns > MIN_MEASURED_COLUMNS) {
            set_node_attr(&block, "style", Some(format!("--columns: {columns}")));
        }
    }
}

fn simplify_head(dom: &RcDom, head: &Handle, add_styles: bool) {
    head.children.borrow_mut().retain(|node| {
        let attr = |name: &str| get_node_attr(node, name).unwrap_or_default().to_lowercase();

        is_element(node, &["title"])
            || (is_element(node, &["link"]) && is_favicon(attr("rel").trim()))
            || (is_element(node, &["meta"])
                && (get_node_attr(node, "charset").is_some()
                    || attr("http-equiv") == "content-type"
                    || attr("name") == "viewport"))
    });

    if add_styles {
        if !head
            .children
            .borrow()
            .iter()
            .any(|node| get_node_attr(node, "name").as_deref() == Some("viewport"))
        {
            let viewport = create_html_element(dom, "meta");
            set_node_attr(&viewport, "name", Some("viewport".to_string()));
            set_node_attr(
                &viewport,
                "content",
                Some("width=device-width, initial-scale=1".to_string()),
            );
            viewport.parent.set(Some(Rc::downgrade(head)));
            head.children.borrow_mut().push(viewport);
        }

        let style = create_html_element(dom, "style");
        set_children(&style, vec![create_text_node(STYLES)]);
        style.parent.set(Some(Rc::downgrade(head)));
        head.children.borrow_mut().push(style);
    }
}
