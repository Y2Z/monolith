#[macro_use]
extern crate clap;
extern crate monolith;

use clap::{App, Arg};
use monolith::html::{html_to_dom, stringify_document, walk_and_embed_assets};
use monolith::http::retrieve_asset;
use monolith::utils::is_valid_url;

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.14; rv:66.0) Gecko/20100101 Firefox/66.0";

fn main() {
    let command = App::new("monolith")
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(
            Arg::with_name("url")
                .required(true)
                .takes_value(true)
                .index(1)
                .help("URL to download"),
        )
        // .args_from_usage("-a, --include-audio 'Embed audio sources'")
        .args_from_usage("-c, --no-css 'Ignore styles'")
        .args_from_usage("-f, --no-frames 'Exclude iframes'")
        .args_from_usage("-i, --no-images 'Remove images'")
        .args_from_usage("-I, --isolate 'Cut off from the Internet'")
        .args_from_usage("-j, --no-js 'Exclude JavaScript'")
        .args_from_usage("-k, --insecure 'Accept invalid X.509 (TLS) certificates'")
        .args_from_usage("-s, --silent 'Suppress verbosity'")
        .args_from_usage("-u, --user-agent=[Iceweasel] 'Custom User-Agent string'")
        // .args_from_usage("-v, --include-video 'Embed video sources'")
        .get_matches();

    // Process the command
    let arg_target: &str = command.value_of("url").unwrap();
    let opt_no_css: bool = command.is_present("no-css");
    let opt_no_frames: bool = command.is_present("no-frames");
    let opt_no_images: bool = command.is_present("no-images");
    let opt_no_js: bool = command.is_present("no-js");
    let opt_insecure: bool = command.is_present("insecure");
    let opt_isolate: bool = command.is_present("isolate");
    let opt_silent: bool = command.is_present("silent");
    let opt_user_agent: &str = command.value_of("user-agent").unwrap_or(DEFAULT_USER_AGENT);

    if is_valid_url(arg_target) {
        let data = retrieve_asset(
            &arg_target,
            false,
            "",
            opt_user_agent,
            opt_silent,
            opt_insecure,
        )
        .unwrap();
        let dom = html_to_dom(&data);

        walk_and_embed_assets(
            &arg_target,
            &dom.document,
            opt_no_css,
            opt_no_js,
            opt_no_images,
            opt_user_agent,
            opt_silent,
            opt_insecure,
            opt_no_frames,
        );

        let html: String = stringify_document(
            &dom.document,
            opt_no_css,
            opt_no_frames,
            opt_no_js,
            opt_no_images,
            opt_isolate,
        );

        println!("{}", html);
    }
}
