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
use std::collections::HashMap;
use std::fs::{remove_file, File};
use std::io::{Error, Write};
use std::time::Duration;

fn create_file(file_path: &String, content: String) -> Result<(), Error> {
    let file = File::create(file_path.as_str());

    let mut file = match file {
        Ok(file) => file,
        Err(error) => return Err(error),
    };

    if content != str!() {
        file.write_all(content.as_bytes())?;
        file.write_all("\n".as_bytes())?;
        file.sync_all()?;
    } else {
        // Remove the file right away if it had no content
        remove_file(file_path.as_str())?;
    }

    Ok(())
}

fn main() {
    let app_args = AppArgs::get();
    let cache = &mut HashMap::new();

    // Attempt to create output file
    if app_args.output != str!() {
        create_file(&app_args.output, str!()).unwrap();
    }

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
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(app_args.insecure)
            .default_headers(header_map)
            .build()
            .expect("Failed to initialize HTTP client");

        // Retrieve root document
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

        if app_args.output == str!() {
            println!("{}", html);
        } else {
            create_file(&app_args.output, html).unwrap();
        }
    }
}
