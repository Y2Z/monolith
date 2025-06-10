use std::{fs, path::PathBuf};

use html5ever::serialize::{serialize, SerializeOpts};
use markup5ever_rcdom::SerializableHandle;
use monolith::{core::MonolithOptions, html, session::Session};
use url::Url;

#[test]
fn as_text_test() {
    // Construct the path to the as_text directory
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/as_text");
    let html_path = dir.join("source.html");

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
    let mut buf: Vec<u8> = Vec::new();
    serialize(
        &mut buf,
        &SerializableHandle::from(dom.document.clone()),
        SerializeOpts::default(),
    )
    .unwrap();

    assert_eq!(
        buf.iter().map(|&c| c as char).collect::<String>(),
        // include_str!("as_text/source-result.html")
        include_str!("as_text/source-result-old.html")
    );
}
