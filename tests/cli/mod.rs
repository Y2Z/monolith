mod archives;
mod base_url;
mod basic;
mod browser;
mod cors;
mod credentials;
mod csp;
mod cycles;
mod data_url;
mod http_encoding;
mod local_files;
mod mhtml;
mod noscript;
mod source_maps;
mod srcdoc;
mod targets;
mod unusual_encodings;
mod use_elements;

/// Monolith, writing to STDOUT unless told otherwise.
pub fn monolith() -> assert_cmd::Command {
    let mut command = assert_cmd::cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
    command.args(["--output", "-"]);
    command
}

/// Monolith, writing wherever it's told to.
pub fn monolith_writing_files() -> assert_cmd::Command {
    assert_cmd::cargo_bin_cmd!(env!("CARGO_PKG_NAME"))
}

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
