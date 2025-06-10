use std::{fs, path::PathBuf};

use html5ever::serialize::{serialize, SerializeOpts};
use markup5ever_rcdom::SerializableHandle;
use monolith::{core::MonolithOptions, html, session::Session};
use url::Url;

fn as_text(name: &str) {
    // Construct the path to the as_text directory
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/as_text/{name}"));
    let html_path = dir.join("index.html");

    // Read the HTML input
    let html = fs::read_to_string(html_path.clone()).unwrap();

    // Prepare a file:// URL to the source.html for correct relative asset resolution
    let url = Url::from_file_path(html_path).unwrap();

    // Setup Monolith options
    let mut options = MonolithOptions::default();
    options.silent = true;
    let mut session = Session::new(None, None, options);

    // Parse HTML into DOM
    let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

    // Walk DOM to embed referenced CSS
    html::walk(&mut session, &url, &dom.document);

    // Serialize the DOM back to HTML
    let mut buf = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(dom.document.clone()),
        SerializeOpts::default(),
    )
    .unwrap();

    let contents = fs::read_to_string(dir.join("result.html"))
        .unwrap();

    assert_eq!(
        buf.iter().map(|&c| c as char).collect::<String>(),
        contents
    );
}

#[test]
fn as_text_css() {
    as_text("css");
}

#[test]
fn as_text_svg() {
    as_text("svg");
}

#[test]
fn as_text_png() {
    as_text("png");
}