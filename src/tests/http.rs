use crate::http::retrieve_asset;
use std::collections::HashMap;
#[test]
fn test_retrieve_asset() {
    let cache = &mut HashMap::new();
    let client = reqwest::Client::new();
    let (data, final_url) =
        retrieve_asset(cache, &client, "data:text/html;base64,...", true, "", false).unwrap();
    assert_eq!(&data, "data:text/html;base64,...");
    assert_eq!(&final_url, "data:text/html;base64,...");

    let (data, final_url) = retrieve_asset(
        cache,
        &client,
        "data:text/html;base64,...",
        true,
        "image/png",
        false,
    )
    .unwrap();
    assert_eq!(&data, "data:text/html;base64,...");
    assert_eq!(&final_url, "data:text/html;base64,...");
}
