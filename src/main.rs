extern crate clap;
extern crate monolith;

use clap::{Arg, App};
use monolith::http::{is_url, retrieve_asset};
use monolith::html::{walk_and_embed_assets, html_to_dom, print_dom};

fn main() {
    let command = App::new("monolith")
        .version("2.0.1")
        .author("Sunshine <sunshine@uberspace.net>")
        .about("CLI tool to save web pages as single HTML files")
        .arg(Arg::with_name("url")
                 .required(true)
                 .takes_value(true)
                 .index(1)
                 .help("URL to download"))
        .args_from_usage("-j, --nojs 'Remove JavaScript'")
        .get_matches();

    // Process the command
    let arg_target = command.value_of("url").unwrap();
    let opt_no_js = command.is_present("nojs");

    if is_url(arg_target) {
        let data = retrieve_asset(&arg_target, false, "");
        let dom = html_to_dom(&data.unwrap());

        walk_and_embed_assets(&arg_target, &dom.document, opt_no_js);

        print_dom(&dom.document);
    }
}
