#[macro_use]
extern crate clap;

mod args;
mod macros;

use crate::args::AppArgs;
use monolith::html::{html_to_dom, stringify_document, walk_and_embed_assets};
use monolith::http::retrieve_asset;
use monolith::utils::is_valid_url;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Proxy;
use std::collections::HashMap;
use std::env;
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

fn create_http_client(args: &AppArgs) -> Result<Client, reqwest::Error> {
    let mut header_map = HeaderMap::new();
    header_map.insert(
        USER_AGENT,
        HeaderValue::from_str(&args.user_agent).expect("Invalid User-Agent header specified"),
    );

    let mut builder = Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(args.insecure)
        .default_headers(header_map);

    if let Ok(var) = env::var("https_proxy").or_else(|_| env::var("HTTPS_PROXY")) {
        if !var.is_empty() {
            let proxy = Proxy::https(&var)
                .expect("Could not set HTTPS proxy. Please check $https_proxy env var");
            builder = builder.proxy(proxy);
        }
    }

    if let Ok(var) = env::var("http_proxy").or_else(|_| env::var("HTTP_PROXY")) {
        if !var.is_empty() {
            let proxy = Proxy::http(&var)
                .expect("Could not set HTTP proxy. Please check $http_proxy env var");
            builder = builder.proxy(proxy);
        }
    }

    builder.build()
}

fn main() {
    let app_args = AppArgs::get();

    if !is_valid_url(app_args.url_target.as_str()) {
        eprintln!(
            "Only HTTP and HTTPS URLs are allowed but got: {}",
            &app_args.url_target
        );
        process::exit(1);
    }

    let mut output = Output::new(&app_args.output).expect("Could not prepare output");
    let mut cache = HashMap::new();
    let client = create_http_client(&app_args).expect("Failed to initialize HTTP client");

    // Retrieve root document
    let (data, final_url) = retrieve_asset(
        &mut cache,
        &client,
        app_args.url_target.as_str(),
        false,
        "",
        app_args.silent,
    )
    .expect("Could not retrieve assets in HTML");
    let dom = html_to_dom(&data);

    walk_and_embed_assets(
        &mut cache,
        &client,
        &final_url,
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
