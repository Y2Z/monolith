//! Source maps (https://tc39.es/ecma426/) are for developer tools, pages don't use them: saved
//! pages don't come with them, so references to them would lead nowhere, and they go.

/// Whether a comment points at a source map: `//# sourceMappingURL=…`, `/*# sourceMappingURL=… */`,
/// or the older `@` forms of those.
pub fn is_source_map_comment(comment: &str) -> bool {
    let comment = comment.trim();
    let body = if let Some(body) = comment.strip_prefix("//") {
        body
    } else if let Some(body) = comment
        .strip_prefix("/*")
        .and_then(|body| body.strip_suffix("*/"))
        .filter(|body| !body.contains("*/"))
    {
        body
    } else {
        return false;
    };
    let Some(body) = body.strip_prefix(['#', '@']) else {
        return false;
    };

    // What's in it is the URL, nothing else (https://tc39.es/ecma426/#sec-linking-inline)
    body.trim_start()
        .strip_prefix("sourceMappingURL=")
        .map(str::trim)
        .is_some_and(|url| !url.is_empty() && !url.contains(char::is_whitespace))
}

/// Scripts and stylesheets without what points at their source maps. Only the last line gets
/// looked at, that's where those go, and anything before it could be anything (strings too).
/// Text that isn't ASCII-compatible (UTF-16) stays the way it is.
pub fn remove_source_map_reference(data: &[u8]) -> &[u8] {
    if data.starts_with(&[0xFF, 0xFE]) || data.starts_with(&[0xFE, 0xFF]) {
        return data;
    }

    let end = data.len()
        - data
            .iter()
            .rev()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
    let line_start = data[..end]
        .iter()
        .rposition(|byte| *byte == b'\n' || *byte == b'\r')
        .map_or(0, |position| position + 1);

    match std::str::from_utf8(&data[line_start..end]) {
        Ok(line) if is_source_map_comment(line) => &data[..line_start],
        _ => data,
    }
}

/// Same as remove_source_map_reference(), for what's been decoded already.
pub fn remove_source_map_reference_from_text(text: &str) -> &str {
    let kept = remove_source_map_reference(text.as_bytes()).len();
    // What's kept ends where a line does
    &text[..kept]
}
