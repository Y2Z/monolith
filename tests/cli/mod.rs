mod archives;
mod base_url;
mod basic;
mod browser;
mod data_url;
mod http_encoding;
mod local_files;
mod mhtml;
mod noscript;
mod unusual_encodings;

/// What monolith has output, without what changes every time it saves something (when it's done
/// that, what version it is): the rest can be compared as is.
pub fn without_metadata(output: &[u8]) -> String {
    let mut output = String::from_utf8_lossy(output).into_owned();
    for name in monolith::html::METADATA_NAMES {
        let start = format!("<meta name=\"{}\" content=\"", name);
        while let Some(position) = output.find(&start) {
            let end = position + output[position..].find('>').unwrap() + 1;
            output.replace_range(position..end, "");
        }
    }

    output
}
