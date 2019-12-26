use crate::html::{
    get_node_name, get_parent_node, html_to_dom, is_icon, stringify_document, walk_and_embed_assets,
};
use html5ever::rcdom::{Handle, NodeData};
use html5ever::serialize::{serialize, SerializeOpts};
use std::collections::HashMap;

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
            _ => (),
        };
    }

    test_walk(&dom.document, &mut count);

    assert_eq!(count, 7);
}

#[test]
fn test_walk_and_embed_assets() {
    let cache = &mut HashMap::new();

    let html = "<div><P></P></div>";
    let dom = html_to_dom(&html);
    let url = "http://localhost";

    let opt_no_css: bool = false;
    let opt_no_frames: bool = false;
    let opt_no_js: bool = false;
    let opt_no_images: bool = false;
    let opt_silent = true;

    let client = reqwest::Client::new();

    walk_and_embed_assets(
        cache,
        &client,
        &url,
        &dom.document,
        opt_no_css,
        opt_no_js,
        opt_no_images,
        opt_silent,
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
fn test_walk_and_embed_assets_ensure_no_recursive_iframe() {
    let html = "<div><P></P><iframe src=\"\"></iframe></div>";
    let dom = html_to_dom(&html);
    let url = "http://localhost";
    let cache = &mut HashMap::new();

    let opt_no_css: bool = false;
    let opt_no_frames: bool = false;
    let opt_no_js: bool = false;
    let opt_no_images: bool = false;
    let opt_silent = true;

    let client = reqwest::Client::new();

    walk_and_embed_assets(
        cache,
        &client,
        &url,
        &dom.document,
        opt_no_css,
        opt_no_js,
        opt_no_images,
        opt_silent,
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
    let cache = &mut HashMap::new();

    let opt_no_css: bool = true;
    let opt_no_frames: bool = false;
    let opt_no_js: bool = false;
    let opt_no_images: bool = false;
    let opt_silent = true;
    let client = reqwest::Client::new();

    walk_and_embed_assets(
        cache,
        &client,
        &url,
        &dom.document,
        opt_no_css,
        opt_no_js,
        opt_no_images,
        opt_silent,
        opt_no_frames,
    );

    let mut buf: Vec<u8> = Vec::new();
    serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

    assert_eq!(
        buf.iter().map(|&c| c as char).collect::<String>(),
        "<html>\
         <head>\
         <link rel=\"stylesheet\" href=\"\">\
         <style></style>\
         </head>\
         <body>\
         <div></div>\
         </body>\
         </html>"
    );
}

#[test]
fn test_walk_and_embed_assets_no_images() {
    let html = "<link rel=\"icon\" href=\"favicon.ico\">\
                <div><img src=\"http://localhost/assets/mono_lisa.png\" /></div>";
    let dom = html_to_dom(&html);
    let url = "http://localhost";
    let cache = &mut HashMap::new();

    let opt_no_css: bool = false;
    let opt_no_frames: bool = false;
    let opt_no_js: bool = false;
    let opt_no_images: bool = true;
    let opt_silent = true;

    let client = reqwest::Client::new();

    walk_and_embed_assets(
        cache,
        &client,
        &url,
        &dom.document,
        opt_no_css,
        opt_no_js,
        opt_no_images,
        opt_silent,
        opt_no_frames,
    );

    let mut buf: Vec<u8> = Vec::new();
    serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

    assert_eq!(
        buf.iter().map(|&c| c as char).collect::<String>(),
        "<html>\
         <head>\
         <link rel=\"icon\" href=\"\">\
         </head>\
         <body>\
         <div>\
         <img src=\"data:image/png;base64,\
         iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0\
         lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\">\
         </div>\
         </body>\
         </html>"
    );
}

#[test]
fn test_walk_and_embed_assets_no_frames() {
    let html = "<iframe src=\"http://trackbook.com\"></iframe>";
    let dom = html_to_dom(&html);
    let url = "http://localhost";
    let cache = &mut HashMap::new();

    let opt_no_css: bool = false;
    let opt_no_frames: bool = true;
    let opt_no_js: bool = false;
    let opt_no_images: bool = false;
    let opt_silent = true;
    let client = reqwest::Client::new();

    walk_and_embed_assets(
        cache,
        &client,
        &url,
        &dom.document,
        opt_no_css,
        opt_no_js,
        opt_no_images,
        opt_silent,
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
    let html = "<div onClick=\"void(0)\">\
                <script src=\"http://localhost/assets/some.js\"></script>\
                <script>alert(1)</script>\
                </div>";
    let dom = html_to_dom(&html);
    let url = "http://localhost";
    let cache = &mut HashMap::new();

    let opt_no_css: bool = false;
    let opt_no_frames: bool = false;
    let opt_no_js: bool = true;
    let opt_no_images: bool = false;
    let opt_silent = true;

    let client = reqwest::Client::new();

    walk_and_embed_assets(
        cache,
        &client,
        &url,
        &dom.document,
        opt_no_css,
        opt_no_js,
        opt_no_images,
        opt_silent,
        opt_no_frames,
    );

    let mut buf: Vec<u8> = Vec::new();
    serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

    assert_eq!(
        buf.iter().map(|&c| c as char).collect::<String>(),
        "<html><head></head><body><div><script src=\"\"></script>\
         <script></script></div></body></html>"
    );
}

#[test]
fn test_walk_and_embed_with_no_integrity() {
    let html = "<title>No integrity</title>\
                <link integrity=\"sha384-...\" rel=\"something\"/>\
                <script integrity=\"sha384-...\" src=\"some.js\"></script>";
    let dom = html_to_dom(&html);
    let url = "http://localhost";
    let cache = &mut HashMap::new();
    let client = reqwest::Client::new();
    let opt_no_css: bool = true;
    let opt_no_frames: bool = true;
    let opt_no_js: bool = true;
    let opt_no_images: bool = true;
    let opt_silent = true;

    walk_and_embed_assets(
        cache,
        &client,
        &url,
        &dom.document,
        opt_no_css,
        opt_no_js,
        opt_no_images,
        opt_silent,
        opt_no_frames,
    );

    let mut buf: Vec<u8> = Vec::new();
    serialize(&mut buf, &dom.document, SerializeOpts::default()).unwrap();

    assert_eq!(
        buf.iter().map(|&c| c as char).collect::<String>(),
        "<html>\
         <head><title>No integrity</title><link rel=\"something\"><script src=\"\"></script></head>\
         <body></body>\
         </html>"
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
    let html = "<title>Isolated document</title>\
                <link rel=\"something\" href=\"some.css\" />\
                <meta http-equiv=\"Content-Security-Policy\" content=\"default-src https:\">\
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
                <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-inline' data:;\"></meta>\
                <title>Isolated document</title>\
                <link rel=\"something\" href=\"some.css\">\
                <meta http-equiv=\"Content-Security-Policy\" content=\"default-src https:\">\
            </head>\
            <body>\
                <div>\
                    <script src=\"some.js\"></script>\
                </div>\
            </body>\
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
    let html = "<!doctype html>\
                <title>Frameless document</title>\
                <link rel=\"something\"/>\
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

#[test]
fn test_stringify_document_isolate_no_frames_no_js_no_css_no_images() {
    let html = "<!doctype html>\
                <title>no-frame no-css no-js no-image isolated document</title>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"default-src https:\">\
                <link rel=\"stylesheet\" href=\"some.css\">\
                <div>\
                <script src=\"some.js\"></script>\
                <img style=\"width: 100%;\" src=\"some.png\" />\
                <iframe src=\"some.html\"></iframe>\
                </div>";
    let dom = html_to_dom(&html);

    let opt_isolate: bool = true;
    let opt_no_css: bool = true;
    let opt_no_frames: bool = true;
    let opt_no_js: bool = true;
    let opt_no_images: bool = true;

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
                    <meta http-equiv=\"Content-Security-Policy\" content=\"default-src \'unsafe-inline\' data:; style-src \'none\'; frame-src \'none\';child-src \'none\'; script-src \'none\'; img-src data:;\"></meta>\
                    <title>no-frame no-css no-js no-image isolated document</title>\
                    <meta http-equiv=\"Content-Security-Policy\" content=\"default-src https:\">\
                    <link rel=\"stylesheet\" href=\"some.css\">\
                </head>\
                <body>\
                    <div>\
                        <script src=\"some.js\"></script>\
                        <img style=\"width: 100%;\" src=\"some.png\">\
                        <iframe src=\"some.html\"></iframe>\
                    </div>\
                </body>\
            </html>"
    );
}
