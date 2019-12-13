#[macro_use]
extern crate clap;
extern crate monolith;
extern crate reqwest;

mod args;

use args::AppArgs;
use monolith::html::{html_to_dom, stringify_document, walk_and_embed_assets};
use monolith::http::retrieve_asset;
use monolith::utils::is_valid_url;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;

fn main() {
    let app_args = AppArgs::get();
    let cache = &mut HashMap::new();
    if is_valid_url(app_args.url_target.as_str()) {
        // Initialize client
        let mut header_map = HeaderMap::new();
        match HeaderValue::from_str(&app_args.user_agent) {
            Ok(header) => header_map.insert(USER_AGENT, header),
            Err(err) => {
                eprintln!("Invalid user agent! {}", err);
                return;
            }
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(app_args.insecure)
            .default_headers(header_map)
            .build()
            .expect("Failed to initialize HTTP client");

        let (data, final_url) = retrieve_asset(
            cache,
            &client,
            app_args.url_target.as_str(),
            false,
            "",
            app_args.silent,
        )
        .unwrap();
        let dom = html_to_dom(&data);

        walk_and_embed_assets(
            cache,
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

        println!("{}", html);
    }
}
