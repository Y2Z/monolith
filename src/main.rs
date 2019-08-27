#[macro_use]
extern crate clap;
extern crate monolith;

use clap::{App, Arg};
use monolith::html::{html_to_dom, print_dom, walk_and_embed_assets};
use monolith::http::{is_valid_url, retrieve_asset};

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
        .args_from_usage("-i, --no-images 'Removes images'")
        .args_from_usage("-j, --no-js 'Excludes JavaScript'")
        .args_from_usage("-s, --silent 'Suppress verbosity'")
        .args_from_usage("-u, --user-agent=[Iceweasel] 'Custom User-Agent string'")
        .get_matches();

    // Process the command
    let arg_target = command.value_of("url").unwrap();
    let opt_no_images = command.is_present("no-images");
    let opt_no_js = command.is_present("no-js");
    let opt_silent = command.is_present("silent");
    let opt_user_agent = command.value_of("user-agent").unwrap_or(DEFAULT_USER_AGENT);

    if is_valid_url(arg_target) {
        let data = retrieve_asset(&arg_target, false, "", opt_user_agent, opt_silent).unwrap();
        let dom = html_to_dom(&data);

        walk_and_embed_assets(
            &arg_target,
            &dom.document,
            opt_no_js,
            opt_no_images,
            opt_user_agent,
            opt_silent,
        );

        print_dom(&dom.document);
        println!(); // Ensure newline at end of output
    }
}
