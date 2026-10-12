use cssparser::{
    ParseError, Parser, SourcePosition, Token, serialize_identifier, serialize_string,
};
use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};

use crate::core::MonolithOutputFormat;
use crate::session::Session;
use crate::url::{EMPTY_IMAGE_DATA_URL, Url, create_data_url, resolve_url};

const CSS_PROPS_WITH_IMAGE_URLS: &[&str] = &[
    // Universal
    "background",
    "background-image",
    "border-image",
    "border-image-source",
    "content",
    "cursor",
    "list-style",
    "list-style-image",
    "mask",
    "mask-image",
    // Specific to @counter-style
    "additive-symbols",
    "negative",
    "pad",
    "prefix",
    "suffix",
    "symbols",
];

pub fn embed_css(session: &mut Session, document_url: &Url, css: &str) -> String {
    let mut parser = Parser::new(css);

    process_css(session, document_url, &mut parser, "", "", "").unwrap()
}

/// Decodes a stylesheet's raw bytes into text, following the order the CSS Syntax spec uses
/// to determine a stylesheet's encoding (https://www.w3.org/TR/css-syntax-3/#input-byte-stream):
///
///   1. byte order mark;
///   2. charset given by the protocol (Content-Type header, data URL);
///   3. `@charset` rule at the very beginning of the stylesheet;
///   4. fallback: UTF-8 if the bytes are valid UTF-8, Windows-1252 otherwise.
///
/// A charset of "US-ASCII" is treated the same as no charset at all, because that's what asset
/// retrieval reports when none was given (RFC 2397 default). For text that really is ASCII it
/// makes no difference, and for text that isn't the fallback mirrors what browsers do (WHATWG
/// maps "us-ascii" onto Windows-1252), while also handling the far more common case of UTF-8
/// stylesheets served without any charset.
///
/// The returned text no longer carries a byte order mark, and its leading `@charset` rule
/// (if any) is rewritten to say UTF-8, since that's how the text will be embedded from now on.
pub fn decode_css(data: &[u8], charset: &str) -> String {
    let (encoding, bom_length) = if let Some((encoding, bom_length)) = Encoding::for_bom(data) {
        (encoding, bom_length)
    } else if let Some(encoding) = protocol_encoding(charset) {
        (encoding, 0)
    } else if let Some(encoding) = charset_rule_encoding(data) {
        (encoding, 0)
    } else if std::str::from_utf8(data).is_ok() {
        (UTF_8, 0)
    } else {
        (WINDOWS_1252, 0)
    };

    let (text, _) = encoding.decode_without_bom_handling(&data[bom_length..]);

    rewrite_charset_rule(&text)
}

/// Decodes a stylesheet (see `decode_css`) and embeds all of its assets.
/// The resulting text is meant to be stored as UTF-8.
pub fn embed_stylesheet(session: &mut Session, url: &Url, data: &[u8], charset: &str) -> String {
    embed_css(session, url, &decode_css(data, charset))
}

const CHARSET_RULE_PREFIX: &str = "@charset \"";
const CHARSET_RULE_SUFFIX: &str = "\";";

fn protocol_encoding(charset: &str) -> Option<&'static Encoding> {
    let charset = charset.trim();

    if charset.is_empty() || charset.eq_ignore_ascii_case("US-ASCII") {
        return None;
    }

    Encoding::for_label_no_replacement(charset.as_bytes())
}

/// Extracts the encoding from a leading `@charset "...";` rule.
/// Per spec, the rule has to match byte for byte (one space, double quotes, no comments).
fn charset_rule_encoding(data: &[u8]) -> Option<&'static Encoding> {
    let label = data.strip_prefix(CHARSET_RULE_PREFIX.as_bytes())?;
    let label_end = label
        .windows(CHARSET_RULE_SUFFIX.len())
        .take(1024) // Labels are short, no need to scan the whole stylesheet
        .position(|window| window == CHARSET_RULE_SUFFIX.as_bytes())?;
    let encoding = Encoding::for_label_no_replacement(&label[..label_end])?;

    // A stylesheet that declares itself UTF-16 but has no BOM is read as UTF-8
    if encoding == UTF_16BE || encoding == UTF_16LE {
        Some(UTF_8)
    } else {
        Some(encoding)
    }
}

fn rewrite_charset_rule(css: &str) -> String {
    if let Some(rest) = css.strip_prefix(CHARSET_RULE_PREFIX) {
        if let Some(label_end) = rest.find(CHARSET_RULE_SUFFIX) {
            if !rest[..label_end].eq_ignore_ascii_case("UTF-8") {
                return format!("{}UTF-8{}", CHARSET_RULE_PREFIX, &rest[label_end..]);
            }
        }
    }

    css.to_string()
}

pub fn format_ident(ident: &str) -> String {
    let mut res: String = "".to_string();
    let _ = serialize_identifier(ident, &mut res);
    res = res.trim_end().to_string();
    res
}

pub fn format_quoted_string(string: &str) -> String {
    let mut res: String = "".to_string();
    let _ = serialize_string(string, &mut res);
    res
}

pub fn is_image_url_prop(prop_name: &str) -> bool {
    CSS_PROPS_WITH_IMAGE_URLS
        .iter()
        .any(|p| prop_name.eq_ignore_ascii_case(p))
}

pub fn process_css<'a>(
    session: &mut Session,
    document_url: &Url,
    parser: &mut Parser,
    rule_name: &str,
    prop_name: &str,
    func_name: &str,
) -> Result<String, ParseError<String>> {
    let mut result: String = "".to_string();

    let mut curr_rule: String = rule_name.to_string();
    let mut curr_prop: String = prop_name.to_string();
    let mut token: &Token;
    let mut token_offset: SourcePosition;

    loop {
        token_offset = parser.position();
        token = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token,
            Err(_) => {
                break;
            }
        };

        match *token {
            Token::Comment(_) => {
                let token_slice = parser.slice_from(token_offset);
                result.push_str(token_slice);
            }
            Token::Semicolon => result.push(';'),
            Token::Colon => result.push(':'),
            Token::Comma => result.push(','),
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                if session.options.no_fonts && curr_rule == "font-face" {
                    continue;
                }

                let closure: &str;
                if token == &Token::ParenthesisBlock {
                    result.push('(');
                    closure = ")";
                } else if token == &Token::SquareBracketBlock {
                    result.push('[');
                    closure = "]";
                } else {
                    result.push('{');
                    closure = "}";
                }

                let block_css: String = parser
                    .parse_nested_block(|parser| {
                        process_css(
                            session,
                            document_url,
                            parser,
                            rule_name,
                            curr_prop.as_str(),
                            func_name,
                        )
                    })
                    .unwrap();
                result.push_str(block_css.as_str());

                result.push_str(closure);
            }
            Token::CloseParenthesis => result.push(')'),
            Token::CloseSquareBracket => result.push(']'),
            Token::CloseCurlyBracket => result.push('}'),
            Token::IncludeMatch => result.push_str("~="),
            Token::DashMatch => result.push_str("|="),
            Token::PrefixMatch => result.push_str("^="),
            Token::SuffixMatch => result.push_str("$="),
            Token::SubstringMatch => result.push_str("*="),
            Token::CDO => result.push_str("<!--"),
            Token::CDC => result.push_str("-->"),
            Token::WhiteSpace(value) => {
                result.push_str(value);
            }
            // div...
            Token::Ident(ref value) => {
                curr_rule = "".to_string();
                curr_prop = value.to_string();
                result.push_str(&format_ident(value));
            }
            // @import, @font-face, @charset, @media...
            Token::AtKeyword(ref value) => {
                curr_rule = value.to_string();
                if session.options.no_fonts && curr_rule == "font-face" {
                    continue;
                }
                result.push('@');
                result.push_str(value);
            }
            Token::Hash(ref value) => {
                result.push('#');
                result.push_str(value);
            }
            Token::QuotedString(ref value) => {
                if curr_rule == "import" {
                    // Reset current at-rule value
                    curr_rule = "".to_string();

                    // Skip empty import values
                    if value.len() == 0 {
                        result.push_str("''");
                        continue;
                    }

                    let import_full_url: Url = resolve_url(document_url, value);
                    match session.retrieve_asset(document_url, &import_full_url) {
                        Ok((
                            import_contents,
                            import_final_url,
                            import_media_type,
                            import_charset,
                        )) => {
                            let mut import_data_url = create_data_url(
                                &import_media_type,
                                "utf-8",
                                embed_stylesheet(
                                    session,
                                    &import_final_url,
                                    &import_contents,
                                    &import_charset,
                                )
                                .as_bytes(),
                                &import_final_url,
                            );
                            import_data_url.set_fragment(import_full_url.fragment());
                            result
                                .push_str(format_quoted_string(import_data_url.as_ref()).as_str());
                        }
                        Err(_) => {
                            // Keep remote reference if unable to retrieve the asset
                            if import_full_url.scheme() == "http"
                                || import_full_url.scheme() == "https"
                            {
                                result.push_str(
                                    format_quoted_string(import_full_url.as_ref()).as_str(),
                                );
                            }
                        }
                    }
                } else if func_name == "url" {
                    // Skip empty url()'s
                    if value.len() == 0 {
                        continue;
                    }

                    if session.options.no_images && is_image_url_prop(curr_prop.as_str()) {
                        result.push_str(format_quoted_string(EMPTY_IMAGE_DATA_URL).as_str());
                    } else {
                        let resolved_url: Url = resolve_url(document_url, value);

                        if session.options.output_format == MonolithOutputFormat::HTML {
                            match session.retrieve_asset(document_url, &resolved_url) {
                                Ok((data, final_url, media_type, charset)) => {
                                    // TODO: if it's @font-face, exclude definitions of non-woff/woff-2 fonts (if woff/woff-2 are present)
                                    let mut data_url =
                                        create_data_url(&media_type, &charset, &data, &final_url);
                                    data_url.set_fragment(resolved_url.fragment());
                                    result
                                        .push_str(format_quoted_string(data_url.as_ref()).as_str());
                                }
                                Err(_) => {
                                    // Keep remote reference if unable to retrieve the asset
                                    if resolved_url.scheme() == "http"
                                        || resolved_url.scheme() == "https"
                                    {
                                        result.push_str(
                                            format_quoted_string(resolved_url.as_ref()).as_str(),
                                        );
                                    }
                                }
                            }
                        } else {
                            result.push_str(format_quoted_string(resolved_url.as_ref()).as_str());
                            session.log_asset_url(&resolved_url);
                        }
                    }
                } else {
                    result.push_str(format_quoted_string(value).as_str());
                }
            }
            Token::Number {
                ref has_sign,
                ref value,
                ..
            } => {
                if *has_sign && *value >= 0. {
                    result.push('+');
                }
                result.push_str(&value.to_string())
            }
            Token::Percentage {
                ref has_sign,
                ref unit_value,
                ..
            } => {
                if *has_sign && *unit_value >= 0. {
                    result.push('+');
                }
                result.push_str(&(unit_value * 100.0).to_string());
                result.push('%');
            }
            Token::Dimension {
                ref has_sign,
                ref value,
                ref unit,
                ..
            } => {
                if *has_sign && *value >= 0. {
                    result.push('+');
                }
                result.push_str(&value.to_string());
                result.push_str(unit.as_ref());
            }
            // #selector, #id...
            Token::IDHash(ref value) => {
                curr_rule = "".to_string();
                result.push('#');
                result.push_str(&format_ident(value));
            }
            // url()
            Token::UnquotedUrl(ref value) => {
                let is_import: bool = curr_rule == "import";

                if is_import {
                    // Reset current at-rule value
                    curr_rule = "".to_string();
                }

                // Skip empty url()'s
                if value.len() < 1 {
                    result.push_str("url()");
                    continue;
                } else if value.starts_with("#") {
                    result.push_str("url(");
                    result.push_str(value);
                    result.push(')');
                    continue;
                }

                result.push_str("url(");
                if is_import {
                    let full_url: Url = resolve_url(document_url, value);
                    match session.retrieve_asset(document_url, &full_url) {
                        Ok((css, final_url, media_type, charset)) => {
                            let mut data_url = create_data_url(
                                &media_type,
                                "utf-8",
                                embed_stylesheet(session, &final_url, &css, &charset).as_bytes(),
                                &final_url,
                            );
                            data_url.set_fragment(full_url.fragment());
                            result.push_str(format_quoted_string(data_url.as_ref()).as_str());
                        }
                        Err(_) => {
                            // Keep remote reference if unable to retrieve the asset
                            if full_url.scheme() == "http" || full_url.scheme() == "https" {
                                result.push_str(format_quoted_string(full_url.as_ref()).as_str());
                            }
                        }
                    }
                } else if is_image_url_prop(curr_prop.as_str()) && session.options.no_images {
                    result.push_str(format_quoted_string(EMPTY_IMAGE_DATA_URL).as_str());
                } else {
                    let full_url: Url = resolve_url(document_url, value);

                    if session.options.output_format == MonolithOutputFormat::HTML {
                        match session.retrieve_asset(document_url, &full_url) {
                            Ok((data, final_url, media_type, charset)) => {
                                let mut data_url =
                                    create_data_url(&media_type, &charset, &data, &final_url);
                                data_url.set_fragment(full_url.fragment());
                                result.push_str(format_quoted_string(data_url.as_ref()).as_str());
                            }
                            Err(_) => {
                                // Keep remote reference if unable to retrieve the asset
                                if full_url.scheme() == "http" || full_url.scheme() == "https" {
                                    result
                                        .push_str(format_quoted_string(full_url.as_ref()).as_str());
                                }
                            }
                        }
                    } else {
                        result.push_str(format_quoted_string(full_url.as_ref()).as_str());
                        session.log_asset_url(&full_url);
                    }
                }
                result.push(')');
            }
            // =
            Token::Delim(ref value) => result.push(*value),
            Token::Function(ref name) => {
                let function_name: &str = &name.clone();
                result.push_str(function_name);
                result.push('(');

                let block_css: String = parser
                    .parse_nested_block(|parser| {
                        process_css(
                            session,
                            document_url,
                            parser,
                            curr_rule.as_str(),
                            curr_prop.as_str(),
                            function_name,
                        )
                    })
                    .unwrap();
                result.push_str(block_css.as_str());

                result.push(')');
            }
            Token::BadUrl(_) | Token::BadString(_) => {}
        }
    }

    // Ensure empty CSS is really empty
    if !result.is_empty() && result.trim().is_empty() {
        result = result.trim().to_string()
    }

    Ok(result)
}
