use clap::Parser;
use encoding_rs::Encoding;
use markup5ever_rcdom::RcDom;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::collections::HashMap;
use std::fs;
use std::io::{self, prelude::*, Error, Write};
use std::path::Path;
use std::process;
use std::time::Duration;
use url::Url;

use monolith::cookies::{parse_cookie_file_contents, Cookie};
use monolith::html::{
    add_favicon, create_metadata_tag, get_base_url, get_charset, has_favicon, html_to_dom,
    serialize_document, set_base_url, set_charset, walk_and_embed_assets,
};
use monolith::opts::Options;
use monolith::url::{create_data_url, resolve_url};
use monolith::utils::retrieve_asset;

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
    let options = Options::parse();
    let mut cookies: Vec<Cookie> = vec![];

    // Check if target was provided
    if options.target.len() == 0 {
        if !options.silent {
            eprintln!("No target specified");
        }
        process::exit(1);
    }

    // Check if custom encoding is valid
    if let Some(custom_encoding) = options.encoding.clone() {
        if !Encoding::for_label_no_replacement(custom_encoding.as_bytes()).is_some() {
            eprintln!("Unknown encoding: {}", &custom_encoding);
            process::exit(1);
        }
    }

    let mut use_stdin: bool = false;

    let target_url = match options.target.as_str() {
        "-" => {
            // Read from pipe (stdin)
            use_stdin = true;
            // Set default target URL to an empty data URL; the user can set it via --base-url
            Url::parse("data:text/html,").unwrap()
        }
        target => match Url::parse(&target) {
            Ok(url) => match url.scheme() {
                "data" | "file" | "http" | "https" => url,
                unsupported_scheme => {
                    if !options.silent {
                        eprintln!("Unsupported target URL type: {}", unsupported_scheme);
                    }
                    process::exit(1)
                }
            },
            Err(_) => {
                // Failed to parse given base URL (perhaps it's a filesystem path?)
                let path: &Path = Path::new(&target);
                match path.exists() {
                    true => match path.is_file() {
                        true => {
                            let canonical_path = fs::canonicalize(&path).unwrap();
                            match Url::from_file_path(canonical_path) {
                                Ok(url) => url,
                                Err(_) => {
                                    if !options.silent {
                                        eprintln!(
                                            "Could not generate file URL out of given path: {}",
                                            &target
                                        );
                                    }
                                    process::exit(1);
                                }
                            }
                        }
                        false => {
                            if !options.silent {
                                eprintln!("Local target is not a file: {}", &target);
                            }
                            process::exit(1);
                        }
                    },
                    false => {
                        // It is not a FS path, now we do what browsers do:
                        // prepend "http://" and hope it points to a website
                        Url::parse(&format!("http://{hopefully_url}", hopefully_url = &target))
                            .unwrap()
                    }
                }
            }
        },
    };

    // Read and parse cookie file
    if let Some(opt_cookie_file) = options.cookie_file.clone() {
        match fs::read_to_string(opt_cookie_file) {
            Ok(str) => match parse_cookie_file_contents(&str) {
                Ok(parsed_cookies) => {
                    cookies = parsed_cookies;
                    // for c in &cookies {
                    //     // if !cookie.is_expired() {
                    //         // options.cookies.append(c);
                    //     // }
                    // }
                }
                Err(_) => {
                    eprintln!("Could not parse specified cookie file");
                    process::exit(1);
                }
            },
            Err(_) => {
                eprintln!("Could not read specified cookie file");
                process::exit(1);
            }
        }
    }

    // Initialize client
    let mut cache = HashMap::new();
    let mut header_map = HeaderMap::new();
    header_map.insert(
        USER_AGENT,
        HeaderValue::from_str(&options.user_agent).expect("Invalid User-Agent header specified"),
    );
    let client = if options.timeout > 0 {
        Client::builder().timeout(Duration::from_secs(options.timeout))
    } else {
        // No timeout is default
        Client::builder()
    }
    .danger_accept_invalid_certs(options.insecure)
    .default_headers(header_map)
    .build()
    .expect("Failed to initialize HTTP client");

    // At first we assume that base URL is the same as target URL
    let mut base_url: Url = target_url.clone();

    let data: Vec<u8>;
    let mut document_encoding: String = "".to_string();
    let mut dom: RcDom;

    // Retrieve target document
    if use_stdin {
        data = read_stdin();
    } else if target_url.scheme() == "file"
        || (target_url.scheme() == "http" || target_url.scheme() == "https")
        || target_url.scheme() == "data"
    {
        match retrieve_asset(&mut cache, &client, &target_url, &target_url, &options, &cookies) {
            Ok((retrieved_data, final_url, media_type, charset)) => {
                // Provide output as text without processing it, the way browsers do
                if !media_type.eq_ignore_ascii_case("text/html")
                    && !media_type.eq_ignore_ascii_case("application/xhtml+xml")
                {
                    // Define output
                    let mut output =
                        Output::new(&options.output).expect("Could not prepare output");

                    // Write retrieved data into STDOUT or file
                    output
                        .write(&retrieved_data)
                        .expect("Could not write output");

                    // Nothing else to do past this point
                    process::exit(0);
                }

                if options
                    .base_url
                    .clone()
                    .unwrap_or("".to_string())
                    .is_empty()
                {
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
    //       has say over what's specified in HTML

    // Attempt to determine document's charset
    if let Some(html_charset) = get_charset(&dom.document) {
        if !html_charset.is_empty() {
            // Check if the charset specified inside HTML is valid
            if let Some(encoding) = Encoding::for_label_no_replacement(html_charset.as_bytes()) {
                document_encoding = html_charset;
                dom = html_to_dom(&data, encoding.name().to_string());
            }
        }
    }

    // Use custom base URL if specified, read and use what's in the DOM otherwise
    let custom_base_url: String = options.base_url.clone().unwrap_or("".to_string());
    if custom_base_url.is_empty() {
        // No custom base URL is specified
        // Try to see if document has BASE element
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
                // Failed to parse given base URL, perhaps it's a filesystem path?
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
    walk_and_embed_assets(&mut cache, &client, &base_url, &dom.document, &options, &cookies);

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

        match retrieve_asset(&mut cache, &client, &target_url, &favicon_ico_url, &options, &cookies) {
            Ok((data, final_url, media_type, charset)) => {
                let favicon_data_url: Url =
                    create_data_url(&media_type, &charset, &data, &final_url);
                dom = add_favicon(&dom.document, favicon_data_url.to_string());
            }
            Err(_) => {
                // Failed to retrieve /favicon.ico
            }
        }
    }

    // Save using specified charset, if given
    if let Some(custom_encoding) = options.encoding.clone() {
        document_encoding = custom_encoding;
        dom = set_charset(dom, document_encoding.clone());
    }

    // Serialize DOM tree
    let mut result: Vec<u8> = serialize_document(dom, document_encoding, &options);

    // Prepend metadata comment tag
    if !options.no_metadata {
        let mut metadata_comment: String = create_metadata_tag(&target_url);
        metadata_comment += "\n";
        result.splice(0..0, metadata_comment.as_bytes().to_vec());
    }

    // Define output
    let mut output = Output::new(&options.output).expect("Could not prepare output");

    // Write result into STDOUT or file
    output.write(&result).expect("Could not write output");
}
