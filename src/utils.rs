extern crate base64;
extern crate mime_sniffer;

use self::base64::encode;
use self::mime_sniffer::MimeTypeSniffer;

pub fn data_to_dataurl(mime: &str, data: &[u8]) -> String {
    let mimetype = if mime == "" {
        detect_mimetype(data)
    } else {
        mime.to_string()
    };
    format!("data:{};base64,{}", mimetype, encode(data))
}

fn detect_mimetype(data: &[u8]) -> String {
    data.sniff_mime_type().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_to_dataurl() {
        let mime = "application/javascript";
        let data = "var word = 'hello';\nalert(word);\n";
        let datauri = data_to_dataurl(mime, data.as_bytes());
        assert_eq!(
            &datauri,
            "data:application/javascript;base64,dmFyIHdvcmQgPSAnaGVsbG8nOwphbGVydCh3b3JkKTsK"
        );
    }
}
