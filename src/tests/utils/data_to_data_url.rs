use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_encode_string_with_specific_media_type() {
    let mime = "application/javascript";
    let data = "var word = 'hello';\nalert(word);\n";
    let data_url = utils::data_to_data_url(mime, data.as_bytes());

    assert_eq!(
        &data_url,
        "data:application/javascript;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
    );
}
