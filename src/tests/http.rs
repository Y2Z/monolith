use crate::http::retrieve_asset;

#[test]
fn test_retrieve_asset() {
    let (data, final_url) =
        retrieve_asset("data:text/html;base64,...", true, "", "", true, false).unwrap();
    assert_eq!(&data, "data:text/html;base64,...");
    assert_eq!(&final_url, "data:text/html;base64,...");

    let (data, final_url) = retrieve_asset(
        "data:text/html;base64,...",
        true,
        "image/png",
        "",
        true,
        false,
    )
    .unwrap();
    assert_eq!(&data, "data:text/html;base64,...");
    assert_eq!(&final_url, "data:text/html;base64,...");
}
