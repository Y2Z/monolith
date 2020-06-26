use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Error, Write};
use std::path::Path;
use std::process;
use std::time::Duration;

use monolith::html::{html_to_dom, metadata_tag, stringify_document, walk_and_embed_assets};
use monolith::url::{data_url_to_data, is_data_url, is_file_url, is_http_url};
use monolith::utils::retrieve_asset;

mod args;
mod macros;

#[macro_use]
extern crate clap;
use crate::args::AppArgs;

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
    let original_target: &str = &app_args.target;
    let target_url: &str;
    let base_url;
    let dom;

    // Pre-process the input
    let cwd_normalized: String =
        str!(env::current_dir().unwrap().to_str().unwrap()).replace("\\", "/");
    let path = Path::new(original_target);
    let mut target: String = str!(original_target.clone()).replace("\\", "/");
    let path_is_relative: bool = path.is_relative();

    // Determine exact target URL
    if target.clone().len() == 0 {
        eprintln!("No target specified");
        process::exit(1);
    } else if is_http_url(target.clone()) || is_data_url(target.clone()) {
        target_url = target.as_str();
    } else if is_file_url(target.clone()) {
        target_url = target.as_str();
    } else if path.exists() {
        if !path.is_file() {
            eprintln!("Local target is not a file: {}", original_target);
            process::exit(1);
        }
        target.insert_str(0, if cfg!(windows) { "file:///" } else { "file://" });
        if path_is_relative {
            target.insert_str(if cfg!(windows) { 8 } else { 7 }, &cwd_normalized);
            target.insert_str(
                if cfg!(windows) { 8 } else { 7 } + &cwd_normalized.len(),
                "/",
            );
        }
        target_url = target.as_str();
    } else {
        target.insert_str(0, "http://");
        target_url = target.as_str();
    }

    // Define output
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

    // Retrieve target document
    if is_file_url(target_url) || is_http_url(target_url) {
        match retrieve_asset(&mut cache, &client, target_url, target_url, app_args.silent) {
            Ok((data, final_url, _media_type)) => {
                base_url = final_url;
                dom = html_to_dom(&String::from_utf8_lossy(&data));
            }
            Err(_) => {
                eprintln!("Could not retrieve target document");
                process::exit(1);
            }
        }
    } else if is_data_url(target_url) {
        let (media_type, data): (String, Vec<u8>) = data_url_to_data(target_url);
        if !media_type.eq_ignore_ascii_case("text/html") {
            eprintln!("Unsupported data URL media type");
            process::exit(1);
        }
        base_url = str!(target_url);
        dom = html_to_dom(&String::from_utf8_lossy(&data));
    } else {
        process::exit(1);
    }

    // Embed remote assets
    walk_and_embed_assets(
        &mut cache,
        &client,
        &base_url,
        &dom.document,
        app_args.no_css,
        app_args.no_fonts,
        app_args.no_frames,
        app_args.no_js,
        app_args.no_images,
        app_args.silent,
    );

    // Serialize DOM tree
    let mut result: String = stringify_document(
        &dom.document,
        app_args.no_css,
        app_args.no_fonts,
        app_args.no_frames,
        app_args.no_js,
        app_args.no_images,
        app_args.isolate,
    );

    // Add metadata tag
    if !app_args.no_metadata {
        let metadata_comment = metadata_tag(&base_url);
        result.insert_str(0, &metadata_comment);
        if metadata_comment.len() > 0 {
            result.insert_str(metadata_comment.len(), "\n");
        }
    }

    // Write result into stdout or file
    output
        .writeln_str(&result)
        .expect("Could not write HTML output");
}
