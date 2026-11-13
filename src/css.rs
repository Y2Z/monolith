use cssparser::{
    ParseError, Parser, SourcePosition, Token, serialize_identifier, serialize_string,
};
use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};

use crate::core::{MonolithOutputFormat, declared_encoding, print_error_message};
use crate::cors::RequestMode;
use crate::csp::{Destination, Request};
use crate::session::Session;
use crate::source_maps::is_source_map_comment;
use crate::url::{EMPTY_IMAGE_DATA_URL, Url, create_data_url, resolve_url};

const CHARSET_RULE_PREFIX: &str = "@charset \"";
const CHARSET_RULE_SUFFIX: &str = "\";";
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

/// Decodes a stylesheet's raw bytes into text (see `decode_stylesheet`), without knowing
/// the encoding of the document that references it.
pub fn decode_css(data: &[u8], charset: &str) -> String {
    decode_stylesheet(data, charset, None).0
}

/// Decodes a stylesheet's raw bytes into text, following the order the CSS Syntax spec uses
/// to determine a stylesheet's encoding (https://www.w3.org/TR/css-syntax-3/#input-byte-stream):
///
///   1. byte order mark;
///   2. charset given by the protocol (Content-Type header, data URL);
///   3. `@charset` rule at the very beginning of the stylesheet;
///   4. environment encoding: the encoding of whatever references the stylesheet
///      (the document for LINK elements, the importing stylesheet for @import);
///   5. fallback: UTF-8 if the bytes are valid UTF-8, Windows-1252 otherwise.
///
/// A charset of "US-ASCII" is treated the same as no charset at all, because that's what asset
/// retrieval reports when none was given (RFC 2397 default). For text that really is ASCII it
/// makes no difference, and for text that isn't the fallback mirrors what browsers do (WHATWG
/// maps "us-ascii" onto Windows-1252), while also handling the far more common case of UTF-8
/// stylesheets served without any charset.
///
/// The environment encoding is used even when the bytes happen to be valid UTF-8, the way
/// browsers do it: short runs of legacy multi-byte text are often valid UTF-8 by accident
/// (e.g. EUC-JP "両" is the same two bytes as UTF-8 "ξ"). A UTF-8 environment adds nothing
/// to the fallback, so it only matters for stylesheets that aren't valid UTF-8, and those are
/// better off read as Windows-1252 than filled with replacement characters.
///
/// Returns the text along with the encoding it was decoded from. The text no longer carries
/// a byte order mark, and its leading `@charset` rule (if any) is rewritten to say UTF-8,
/// since that's how the text will be embedded from now on.
pub fn decode_stylesheet(
    data: &[u8],
    charset: &str,
    environment_encoding: Option<&'static Encoding>,
) -> (String, &'static Encoding) {
    let environment_encoding =
        environment_encoding.filter(|encoding| ![UTF_8, UTF_16BE, UTF_16LE].contains(encoding));

    let (encoding, bom_length) = if let Some((encoding, bom_length)) = Encoding::for_bom(data) {
        (encoding, bom_length)
    } else if let Some(encoding) = declared_encoding(charset) {
        (encoding, 0)
    } else if let Some(encoding) = charset_rule_encoding(data) {
        (encoding, 0)
    } else if let Some(encoding) = environment_encoding {
        (encoding, 0)
    } else if std::str::from_utf8(data).is_ok() {
        (UTF_8, 0)
    } else {
        (WINDOWS_1252, 0)
    };

    let (text, _) = encoding.decode_without_bom_handling(&data[bom_length..]);

    (rewrite_charset_rule(&text), encoding)
}

/// Decodes a stylesheet (see `decode_stylesheet`) and embeds all of its assets.
/// The resulting text is meant to be stored as UTF-8.
pub fn embed_stylesheet(session: &mut Session, url: &Url, data: &[u8], charset: &str) -> String {
    // A stylesheet that's being expanded already is importing itself, directly or through other
    // stylesheets; its contents are on their way into the result, so there's nothing more to add
    let Some(expansion_key) = session.enter_expansion(url) else {
        return String::new();
    };

    let (css, encoding) = decode_stylesheet(data, charset, session.environment_encoding);

    // Stylesheets imported by this one use its encoding as their environment encoding
    let environment_encoding = session.environment_encoding.replace(encoding);
    let result = embed_css(session, url, &css);
    session.environment_encoding = environment_encoding;

    session.leave_expansion(&expansion_key);

    result
}

/// Stylesheet that imports nothing, and has nothing in it.
const EMPTY_STYLESHEET: &str = "data:text/css,";

/// What an @import's URL becomes: the stylesheet embedded as a data URL, quoted.
/// Stylesheets that import stylesheets that import them (or that import themselves) get ignored
/// the way browsers ignore them: imports in such cycles become empty stylesheets, by URL, before
/// and after redirects (see Session::enter_expansion()). The same stylesheet imported more than
/// once (but not in a cycle) gets embedded each time.
fn import_stylesheet(session: &mut Session, document_url: &Url, value: &str) -> String {
    let import_url: Url = resolve_url(document_url, value);

    if session.is_expanding(&import_url) {
        if session.is_too_deep() && !session.options.silent {
            print_error_message(&format!("{} (assets are nested too deep)", import_url));
        }
        return format_quoted_string(EMPTY_STYLESHEET);
    }

    match session.retrieve_for(
        document_url,
        &import_url,
        RequestMode::NoCors,
        &Request::to(Destination::Style),
    ) {
        // What the policies don't allow doesn't get imported
        Ok(None) => format_quoted_string(EMPTY_STYLESHEET),
        Ok(Some((data, final_url, media_type, charset))) => {
            // What's in data URLs can't be named by nonce, policies have to allow them as they are
            session.saved_content.has_imports = true;
            // The stylesheet can refer to itself by the URL it was requested by, even if it
            // ended up somewhere else
            let alias = session.enter_expansion_alias(&import_url, &final_url);
            let css = embed_stylesheet(session, &final_url, &data, &charset);
            if let Some(key) = alias {
                session.leave_expansion_alias(&key);
            }

            let data_url =
                create_data_url(&media_type, "utf-8", css, &final_url, import_url.fragment());
            format_quoted_string(&data_url)
        }
        // Keep remote reference if unable to retrieve the asset
        Err(_) if import_url.scheme() == "http" || import_url.scheme() == "https" => {
            format_quoted_string(import_url.as_ref())
        }
        Err(_) => String::new(),
    }
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
    if let Some(rest) = css.strip_prefix(CHARSET_RULE_PREFIX)
        && let Some(label_end) = rest.find(CHARSET_RULE_SUFFIX)
        && !rest[..label_end].eq_ignore_ascii_case("UTF-8")
    {
        return format!("{}UTF-8{}", CHARSET_RULE_PREFIX, &rest[label_end..]);
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

const FONT_FACE_RULE: &str = "font-face";

/// How what url() points at gets requested: fonts in CORS mode, everything else not.
/// What url()s are for, when it comes to policies (see csp.rs).
fn url_request(rule_name: &str) -> Request<'static> {
    Request::to(match rule_name == FONT_FACE_RULE {
        true => Destination::Font,
        false => Destination::Image,
    })
}

fn url_mode(rule_name: &str) -> RequestMode {
    match rule_name == FONT_FACE_RULE {
        true => RequestMode::FONT,
        false => RequestMode::NoCors,
    }
}

pub fn process_css(
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
                // Source maps don't come along (see source_maps.rs)
                if !is_source_map_comment(token_slice) {
                    result.push_str(token_slice);
                }
            }
            Token::Semicolon => result.push(';'),
            Token::Colon => result.push(':'),
            Token::Comma => result.push(','),
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                if session.options.remove_fonts && curr_rule == "font-face" {
                    continue;
                }

                let closure: &str = if token == &Token::ParenthesisBlock {
                    result.push('(');
                    ")"
                } else if token == &Token::SquareBracketBlock {
                    result.push('[');
                    "]"
                } else {
                    result.push('{');
                    "}"
                };

                // What's inside of @font-face blocks is about fonts, all the way down
                let block_rule: &str = if curr_rule == FONT_FACE_RULE {
                    FONT_FACE_RULE
                } else {
                    rule_name
                };
                let block_css: String = parser
                    .parse_nested_block(|parser| {
                        process_css(
                            session,
                            document_url,
                            parser,
                            block_rule,
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
                if session.options.remove_fonts && curr_rule == "font-face" {
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
                    if value.is_empty() {
                        result.push_str("''");
                        continue;
                    }

                    result.push_str(&import_stylesheet(session, document_url, value));
                } else if func_name == "url" {
                    // Skip empty url()'s
                    if value.is_empty() {
                        continue;
                    }

                    if session.options.remove_images && is_image_url_prop(curr_prop.as_str()) {
                        result.push_str(format_quoted_string(EMPTY_IMAGE_DATA_URL).as_str());
                    } else {
                        let resolved_url: Url = resolve_url(document_url, value);

                        if session.options.output_format != MonolithOutputFormat::MHTML {
                            match session.retrieve_for(
                                document_url,
                                &resolved_url,
                                url_mode(rule_name),
                                &url_request(rule_name),
                            ) {
                                // Fonts other origins don't allow the page to use don't get used
                                Ok(None) => result.push_str(&format_quoted_string("")),
                                Ok(Some((data, final_url, media_type, charset))) => {
                                    // TODO: if it's @font-face, exclude definitions of non-woff/woff-2 fonts (if woff/woff-2 are present)
                                    let data_url = create_data_url(
                                        &media_type,
                                        &charset,
                                        data,
                                        &final_url,
                                        resolved_url.fragment(),
                                    );
                                    result.push_str(format_quoted_string(&data_url).as_str());
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
                            if session.is_allowed(&resolved_url, &url_request(rule_name)) {
                                result
                                    .push_str(format_quoted_string(resolved_url.as_ref()).as_str());
                                session.log_asset_url_as(&resolved_url, url_mode(rule_name));
                            } else {
                                result.push_str(&format_quoted_string(""));
                            }
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
                if value.is_empty() {
                    result.push_str("url()");
                    continue;
                } else if !is_import && value.starts_with("#") {
                    result.push_str("url(");
                    result.push_str(value);
                    result.push(')');
                    continue;
                }

                result.push_str("url(");
                if is_import {
                    result.push_str(&import_stylesheet(session, document_url, value));
                } else if is_image_url_prop(curr_prop.as_str()) && session.options.remove_images {
                    result.push_str(format_quoted_string(EMPTY_IMAGE_DATA_URL).as_str());
                } else {
                    let full_url: Url = resolve_url(document_url, value);

                    if session.options.output_format != MonolithOutputFormat::MHTML {
                        match session.retrieve_for(
                            document_url,
                            &full_url,
                            url_mode(rule_name),
                            &url_request(rule_name),
                        ) {
                            // Fonts other origins don't allow the page to use don't get used
                            Ok(None) => result.push_str(&format_quoted_string("")),
                            Ok(Some((data, final_url, media_type, charset))) => {
                                let data_url = create_data_url(
                                    &media_type,
                                    &charset,
                                    data,
                                    &final_url,
                                    full_url.fragment(),
                                );
                                result.push_str(format_quoted_string(&data_url).as_str());
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
                        if session.is_allowed(&full_url, &url_request(rule_name)) {
                            result.push_str(format_quoted_string(full_url.as_ref()).as_str());
                            session.log_asset_url_as(&full_url, url_mode(rule_name));
                        } else {
                            result.push_str(&format_quoted_string(""));
                        }
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

                let function_rule: &str = if rule_name == FONT_FACE_RULE {
                    FONT_FACE_RULE
                } else {
                    curr_rule.as_str()
                };
                let block_css: String = parser
                    .parse_nested_block(|parser| {
                        process_css(
                            session,
                            document_url,
                            parser,
                            function_rule,
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
