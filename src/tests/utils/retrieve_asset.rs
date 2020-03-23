use crate::utils;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::env;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_read_data_url() {
    let cache = &mut HashMap::new();
    let client = Client::new();

    // If both source and target are data URLs,
    // ensure the result contains target data URL
    let (retrieved_data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        "data:text/html;base64,SoUrCe",
        "data:text/html;base64,TaRgEt",
        true,
        "",
        false,
    )
    .unwrap();
    assert_eq!(&retrieved_data, "data:text/html;base64,TaRgEt");
    assert_eq!(&final_url, "data:text/html;base64,TaRgEt");
}

#[test]
fn passing_read_data_url_ignore_suggested_media_type() {
    let cache = &mut HashMap::new();
    let client = Client::new();

    // Media type parameter should not influence data URLs
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        "data:text/html;base64,SoUrCe",
        "data:text/html;base64,TaRgEt",
        true,
        "image/png",
        false,
    )
    .unwrap();
    assert_eq!(&data, "data:text/html;base64,TaRgEt");
    assert_eq!(&final_url, "data:text/html;base64,TaRgEt");
}

#[test]
fn passing_read_local_file_with_file_url_parent() {
    let cache = &mut HashMap::new();
    let client = Client::new();

    let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

    // Inclusion of local assets from local sources should be allowed
    let cwd = env::current_dir().unwrap();
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        &format!(
            "{file}{cwd}/src/tests/data/local-file.html",
            file = file_url_protocol,
            cwd = cwd.to_str().unwrap()
        ),
        &format!(
            "{file}{cwd}/src/tests/data/local-script.js",
            file = file_url_protocol,
            cwd = cwd.to_str().unwrap()
        ),
        true,
        "application/javascript",
        false,
    )
    .unwrap();
    assert_eq!(&data, "data:application/javascript;base64,ZG9jdW1lbnQuYm9keS5zdHlsZS5iYWNrZ3JvdW5kQ29sb3IgPSAiZ3JlZW4iOwpkb2N1bWVudC5ib2R5LnN0eWxlLmNvbG9yID0gInJlZCI7Cg==");
    assert_eq!(
        &final_url,
        &format!(
            "{file}{cwd}/src/tests/data/local-script.js",
            file = file_url_protocol,
            cwd = cwd.to_str().unwrap()
        )
    );
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn failing_read_local_file_with_data_url_parent() {
    let cache = &mut HashMap::new();
    let client = Client::new();

    // Inclusion of local assets from data URL sources should not be allowed
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        "data:text/html;base64,SoUrCe",
        "file:///etc/passwd",
        true,
        "",
        false,
    )
    .unwrap();
    assert_eq!(&data, "");
    assert_eq!(&final_url, "");
}

#[test]
fn failing_read_local_file_with_https_parent() {
    let cache = &mut HashMap::new();
    let client = Client::new();

    // Inclusion of local assets from remote sources should not be allowed
    let (data, final_url) = utils::retrieve_asset(
        cache,
        &client,
        "https://kernel.org/",
        "file:///etc/passwd",
        true,
        "",
        false,
    )
    .unwrap();
    assert_eq!(&data, "");
    assert_eq!(&final_url, "");
}
