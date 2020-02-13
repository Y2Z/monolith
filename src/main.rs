#[macro_use]
extern crate clap;

mod args;
mod macros;

use crate::args::AppArgs;
use monolith::html::{html_to_dom, stringify_document, walk_and_embed_assets};
use monolith::http::retrieve_asset;
use monolith::utils::{data_url_to_text, is_data_url, is_http_url};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Error, Write};
use std::process;
use std::time::Duration;

enum Output {
    Stdout(io::Stdout),
    File(File),
}

impl Output {
    fn new(file_path: &str) -> Result<Output, Error> {
        if file_path.is_empty() {
            Ok(Output::Stdout(io::stdout()))
        } else {
            Ok(Output::File(File::create(file_path)?))
        }
    }

    fn writeln_str(&mut self, s: &str) -> Result<(), Error> {
        match self {
            Output::Stdout(stdout) => {
                writeln!(stdout, "{}", s)?;
                stdout.flush()
            }
            Output::File(f) => {
                writeln!(f, "{}", s)?;
                f.flush()
            }
        }
    }
}

fn main() {
    let app_args = AppArgs::get();
    let target_url: &str = app_args.url_target.as_str();
    let base_url;
    let dom;

    if !is_http_url(target_url) && !is_data_url(target_url) {
        eprintln!(
            "Only HTTP(S) or data URLs are allowed but got: {}",
            &target_url
        );
        process::exit(1);
    }

    let mut output = Output::new(&app_args.output).expect("Could not prepare output");

    // Initialize client
    let mut cache = HashMap::new();
    let mut header_map = HeaderMap::new();
    header_map.insert(
        USER_AGENT,
        HeaderValue::from_str(&app_args.user_agent).expect("Invalid User-Agent header specified"),
    );

    let timeout: u64 = if app_args.timeout > 0 {
        app_args.timeout
    } else {
        std::u64::MAX / 4
    };
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .danger_accept_invalid_certs(app_args.insecure)
        .default_headers(header_map)
        .build()
        .expect("Failed to initialize HTTP client");

    // Retrieve root document
    if is_http_url(target_url) {
        let (data, final_url) =
            retrieve_asset(&mut cache, &client, target_url, false, "", app_args.silent)
                .expect("Could not retrieve assets in HTML");
        dom = html_to_dom(&data);
        base_url = final_url;
    } else if is_data_url(target_url) {
        base_url = target_url.to_string();
        dom = html_to_dom(&data_url_to_text(target_url));
    } else {
        process::exit(1);
    }

    walk_and_embed_assets(
        &mut cache,
        &client,
        &base_url,
        &dom.document,
        app_args.no_css,
        app_args.no_js,
        app_args.no_images,
        app_args.silent,
        app_args.no_frames,
    );

    let html: String = stringify_document(
        &dom.document,
        app_args.no_css,
        app_args.no_frames,
        app_args.no_js,
        app_args.no_images,
        app_args.isolate,
    );

    output
        .writeln_str(&html)
        .expect("Could not write HTML output");
}
