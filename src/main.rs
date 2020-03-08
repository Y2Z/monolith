#[macro_use]
extern crate clap;

mod args;
mod macros;

use crate::args::AppArgs;
use monolith::html::{html_to_dom, stringify_document, walk_and_embed_assets};
use monolith::utils::{data_url_to_text, is_data_url, is_file_url, is_http_url, retrieve_asset};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Error, Write};
use std::path::Path;
use std::process;
use std::time::Duration;

enum Output {
    Stdout(io::Stdout),
    File(fs::File),
}

impl Output {
    fn new(file_path: &str) -> Result<Output, Error> {
        if file_path.is_empty() {
            Ok(Output::Stdout(io::stdout()))
        } else {
            Ok(Output::File(fs::File::create(file_path)?))
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
    let mut original_target: String = app_args.url_target.clone();
    let target_url: &str;
    let base_url;
    let dom;

    // Pre-process the input
    let cwd = env::current_dir().unwrap();
    let path = Path::new(original_target.as_str());
    if original_target.clone().len() == 0 {
        eprintln!("No target specified");
        process::exit(1);
    } else if is_http_url(original_target.clone()) || is_data_url(original_target.clone()) {
        target_url = original_target.as_str();
    } else if original_target.starts_with('/') {
        original_target.insert_str(0, "file://");
        target_url = original_target.as_str();
    } else if path.exists() {
        if !path.is_file() {
            eprintln!("Local target is not a file: {}", original_target);
            process::exit(1);
        }
        original_target.insert_str(0, "file://");
        original_target.insert_str(7, cwd.to_str().unwrap());
        original_target.insert_str(7 + cwd.to_str().unwrap().len(), "/");
        target_url = original_target.as_str();
    } else {
        original_target.insert_str(0, "http://");
        target_url = original_target.as_str();
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
    if is_file_url(target_url) || is_http_url(target_url) {
        let (data, final_url) = retrieve_asset(
            &mut cache,
            &client,
            target_url,
            target_url,
            false,
            "",
            app_args.silent,
        )
        .expect("Could not retrieve target document");
        base_url = final_url;
        dom = html_to_dom(&data);
    } else if is_data_url(target_url) {
        let text: String = data_url_to_text(target_url);
        if text.len() == 0 {
            eprintln!("Unsupported data URL input");
            process::exit(1);
        }
        base_url = str!(target_url);
        dom = html_to_dom(&text);
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
