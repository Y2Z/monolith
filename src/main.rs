use encoding_rs::Encoding;
use html5ever::rcdom::RcDom;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::collections::HashMap;
use std::fs;
use std::io::{self, prelude::*, Error, Write};
use std::path::Path;
use std::process;
use std::time::Duration;
use url::Url;

use monolith::html::{
    add_favicon, create_metadata_tag, get_base_url, get_charset, has_favicon, html_to_dom,
    serialize_document, set_base_url, set_charset, walk_and_embed_assets,
};
use monolith::opts::Options;
use monolith::url::{create_data_url, resolve_url};
use monolith::utils::retrieve_asset;

mod macros;

enum Output {
    Stdout(io::Stdout),
    File(fs::File),
}

impl Output {
    fn new(file_path: &str) -> Result<Output, Error> {
        if file_path.is_empty() || file_path.eq("-") {
            Ok(Output::Stdout(io::stdout()))
        } else {
            Ok(Output::File(fs::File::create(file_path)?))
        }
    }

    fn write(&mut self, bytes: &Vec<u8>) -> Result<(), Error> {
        match self {
            Output::Stdout(stdout) => {
                stdout.write_all(bytes)?;
                // Ensure newline at end of output
                if bytes.last() != Some(&b"\n"[0]) {
                    stdout.write(b"\n")?;
                }
                stdout.flush()
            }
            Output::File(file) => {
                file.write_all(bytes)?;
                // Ensure newline at end of output
                if bytes.last() != Some(&b"\n"[0]) {
                    file.write(b"\n")?;
                }
                file.flush()
            }
        }
    }
}

pub fn read_stdin() -> Vec<u8> {
    let mut buffer: Vec<u8> = vec![];

    match io::stdin().lock().read_to_end(&mut buffer) {
        Ok(_) => buffer,
        Err(_) => buffer,
    }
}

fn main() {
    let options = Options::from_args();
    let mut target: String = str!(&options.target.clone());

    // Check if target was provided
    if target.len() == 0 {
        if !options.silent {
            eprintln!("No target specified");
        }
        process::exit(1);
    }

    // Check if custom charset is valid
    if let Some(custom_charset) = options.charset.clone() {
        if !Encoding::for_label_no_replacement(custom_charset.as_bytes()).is_some() {
            eprintln!("Unknown encoding: {}", &custom_charset);
            process::exit(1);
        }
    }

    let target_url: Url;
    let mut base_url: Url;
    let mut use_stdin: bool = false;

    // Determine exact target URL
    if target.clone() == "-" {
        // Read from pipe (stdin)
        use_stdin = true;
        // Set default target URL to an empty data URL; the user can set it via --base-url
        target_url = Url::parse("data:text/html,").unwrap();
    } else {
        match Url::parse(&target.clone()) {
            Ok(parsed_url) => {
                if parsed_url.scheme() == "data"
                    || parsed_url.scheme() == "file"
                    || (parsed_url.scheme() == "http" || parsed_url.scheme() == "https")
                {
                    target_url = parsed_url;
                } else {
                    if !options.silent {
                        eprintln!("Unsupported target URL type: {}", &parsed_url.scheme());
                    }
                    process::exit(1);
                }
            }
            Err(_err) => {
                // Failed to parse given base URL,
                // perhaps it's a filesystem path?
                let path: &Path = Path::new(&target);

                if path.exists() {
                    if path.is_file() {
                        match Url::from_file_path(fs::canonicalize(&path).unwrap()) {
                            Ok(file_url) => {
                                target_url = file_url;
                            }
                            Err(_err) => {
                                if !options.silent {
                                    eprintln!(
                                        "Could not generate file URL out of given path: {}",
                                        "err"
                                    );
                                }
                                process::exit(1);
                            }
                        }
                    } else {
                        if !options.silent {
                            eprintln!("Local target is not a file: {}", &options.target);
                        }
                        process::exit(1);
                    }
                } else {
                    // Last chance, now we do what browsers do:
                    // prepend "http://" and hope it points to a website
                    target.insert_str(0, "http://");
                    target_url = Url::parse(&target).unwrap();
                }
            }
        }
    }

    // Initialize client
    let mut cache = HashMap::new();
    let mut header_map = HeaderMap::new();
    if let Some(user_agent) = &options.user_agent {
        header_map.insert(
            USER_AGENT,
            HeaderValue::from_str(&user_agent).expect("Invalid User-Agent header specified"),
        );
    }
    let timeout: u64 = if options.timeout > 0 {
        options.timeout
    } else {
        std::u64::MAX / 4 // This is pretty close to infinity
    };
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .danger_accept_invalid_certs(options.insecure)
        .default_headers(header_map)
        .build()
        .expect("Failed to initialize HTTP client");

    // At this stage we assume that the base URL is the same as the target URL
    base_url = target_url.clone();

    let data: Vec<u8>;
    let mut document_encoding: String = str!();
    let mut dom: RcDom;

    // Retrieve target document
    if use_stdin {
        data = read_stdin();
    } else if target_url.scheme() == "file"
        || (target_url.scheme() == "http" || target_url.scheme() == "https")
        || target_url.scheme() == "data"
    {
        match retrieve_asset(&mut cache, &client, &target_url, &target_url, &options, 0) {
            Ok((retrieved_data, final_url, media_type, charset)) => {
                // Make sure the media type is text/html
                if !media_type.eq_ignore_ascii_case("text/html") {
                    if !options.silent {
                        eprintln!("Unsupported document media type");
                    }
                    process::exit(1);
                }

                if options.base_url.clone().unwrap_or(str!()).is_empty() {
                    base_url = final_url;
                }

                data = retrieved_data;
                document_encoding = charset;
            }
            Err(_) => {
                if !options.silent {
                    eprintln!("Could not retrieve target document");
                }
                process::exit(1);
            }
        }
    } else {
        process::exit(1);
    }

    // Initial parse
    dom = html_to_dom(&data, document_encoding.clone());

    // TODO: investigate if charset from filesystem/data URL/HTTP headers
    //       has power over what's specified in HTML

    // Attempt to determine document's charset
    if let Some(charset) = get_charset(&dom.document) {
        if !charset.is_empty() {
            // Check if the charset specified inside HTML is valid
            if let Some(encoding) = Encoding::for_label(charset.as_bytes()) {
                // No point in parsing HTML again with the same encoding as before
                if encoding.name() != "UTF-8" {
                    document_encoding = charset;
                    dom = html_to_dom(&data, document_encoding.clone());
                }
            }
        }
    }

    // Use custom base URL if specified, read and use what's in the DOM otherwise
    let custom_base_url: String = options.base_url.clone().unwrap_or(str!());
    if custom_base_url.is_empty() {
        // No custom base URL is specified,
        // try to see if the document has BASE tag
        if let Some(existing_base_url) = get_base_url(&dom.document) {
            base_url = resolve_url(&target_url, &existing_base_url);
        }
    } else {
        // Custom base URL provided
        match Url::parse(&custom_base_url) {
            Ok(parsed_url) => {
                if parsed_url.scheme() == "file" {
                    // File base URLs can only work with
                    // documents saved from filesystem
                    if target_url.scheme() == "file" {
                        base_url = parsed_url;
                    }
                } else {
                    base_url = parsed_url;
                }
            }
            Err(_) => {
                // Failed to parse given base URL,
                // perhaps it's a filesystem path?
                if target_url.scheme() == "file" {
                    // Relative paths could work for documents saved from filesystem
                    let path: &Path = Path::new(&custom_base_url);
                    if path.exists() {
                        match Url::from_file_path(fs::canonicalize(&path).unwrap()) {
                            Ok(file_url) => {
                                base_url = file_url;
                            }
                            Err(_) => {
                                if !options.silent {
                                    eprintln!(
                                        "Could not map given path to base URL: {}",
                                        custom_base_url
                                    );
                                }
                                process::exit(1);
                            }
                        }
                    }
                }
            }
        }
    }

    // Traverse through the document and embed remote assets
    walk_and_embed_assets(&mut cache, &client, &base_url, &dom.document, &options, 0);

    // Update or add new BASE element to reroute network requests and hash-links
    if let Some(new_base_url) = options.base_url.clone() {
        dom = set_base_url(&dom.document, new_base_url);
    }

    // Request and embed /favicon.ico (unless it's already linked in the document)
    if !options.no_images
        && (target_url.scheme() == "http" || target_url.scheme() == "https")
        && !has_favicon(&dom.document)
    {
        let favicon_ico_url: Url = resolve_url(&base_url, "/favicon.ico");

        match retrieve_asset(
            &mut cache,
            &client,
            &target_url,
            &favicon_ico_url,
            &options,
            0,
        ) {
            Ok((data, final_url, media_type, _charset)) => {
                // TODO: use charset
                let favicon_data_url: Url = create_data_url(&media_type, &data, &final_url);
                dom = add_favicon(&dom.document, favicon_data_url.to_string());
            }
            Err(_) => {
                // Failed to retrieve /favicon.ico
            }
        }
    }

    // Save using specified charset, if given
    if let Some(custom_charset) = options.charset.clone() {
        document_encoding = custom_charset;
        dom = set_charset(dom, document_encoding.clone());
    }

    // Serialize DOM tree
    let mut result: Vec<u8> = serialize_document(dom, document_encoding, &options);

    // Add metadata tag
    if !options.no_metadata {
        let mut metadata_comment: String = create_metadata_tag(&target_url);
        metadata_comment += "\n";
        result.splice(0..0, metadata_comment.as_bytes().to_vec());
    }

    // Define output
    let mut output = Output::new(&options.output).expect("Could not prepare output");

    // Write result into stdout or file
    output.write(&result).expect("Could not write HTML output");
}
