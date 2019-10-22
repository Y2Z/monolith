#[macro_use]
extern crate clap;
extern crate monolith;

mod args;

use args::AppArgs;
use monolith::html::{html_to_dom, stringify_document, walk_and_embed_assets};
use monolith::http::retrieve_asset;
use monolith::utils::is_valid_url;
use std::collections::HashMap;

fn main() {
    let app_args = AppArgs::get();
    let cache = &mut HashMap::new();
    if is_valid_url(app_args.url_target.as_str()) {
        let (data, final_url) = retrieve_asset(
            cache,
            app_args.url_target.as_str(),
            false,
            "",
            app_args.user_agent.as_str(),
            app_args.silent,
            app_args.insecure,
        )
        .unwrap();
        let dom = html_to_dom(&data);

        walk_and_embed_assets(
            cache,
            &final_url,
            &dom.document,
            app_args.no_css,
            app_args.no_js,
            app_args.no_images,
            app_args.user_agent.as_str(),
            app_args.silent,
            app_args.insecure,
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
