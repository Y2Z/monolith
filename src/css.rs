use cssparser::{ParseError, Parser, ParserInput, SourcePosition, Token};
use reqwest::blocking::Client;
use std::collections::HashMap;

use crate::utils::{data_to_data_url, get_url_fragment, is_http_url, resolve_url, retrieve_asset};

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
const CSS_SPECIAL_CHARS: &str = "~!@$%^&*()+=,./'\";:?><[]{}|`#";

pub fn is_image_url_prop(prop_name: &str) -> bool {
    CSS_PROPS_WITH_IMAGE_URLS
        .iter()
        .find(|p| prop_name.eq_ignore_ascii_case(p))
        .is_some()
}

pub fn enquote(input: String, double: bool) -> String {
    if double {
        format!("\"{}\"", input.replace("\"", "\\\""))
    } else {
        format!("'{}'", input.replace("'", "\\'"))
    }
}

pub fn escape(value: &str) -> String {
    let mut res = str!(&value);

    res = res.replace("\\", "\\\\");

    for c in CSS_SPECIAL_CHARS.chars() {
        res = res.replace(c, format!("\\{}", c).as_str());
    }

    res
}

pub fn process_css<'a>(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    parent_url: &str,
    parser: &mut Parser,
    rule_name: &str,
    prop_name: &str,
    func_name: &str,
    opt_no_fonts: bool,
    opt_no_images: bool,
    opt_silent: bool,
) -> Result<String, ParseError<'a, String>> {
    let mut result: String = str!();

    let mut curr_rule: String = str!(rule_name.clone());
    let mut curr_prop: String = str!(prop_name.clone());
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
                result.push_str(str!(token_slice).as_str());
            }
            Token::Semicolon => result.push_str(";"),
            Token::Colon => result.push_str(":"),
            Token::Comma => result.push_str(","),
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                if opt_no_fonts && curr_rule == "font-face" {
                    continue;
                }

                let closure: &str;
                if token == &Token::ParenthesisBlock {
                    result.push_str("(");
                    closure = ")";
                } else if token == &Token::SquareBracketBlock {
                    result.push_str("[");
                    closure = "]";
                } else {
                    result.push_str("{");
                    closure = "}";
                }

                let block_css: String = parser
                    .parse_nested_block(|parser| {
                        process_css(
                            cache,
                            client,
                            parent_url,
                            parser,
                            rule_name,
                            curr_prop.as_str(),
                            func_name,
                            opt_no_fonts,
                            opt_no_images,
                            opt_silent,
                        )
                    })
                    .unwrap();
                result.push_str(block_css.as_str());

                result.push_str(closure);
            }
            Token::CloseParenthesis => result.push_str(")"),
            Token::CloseSquareBracket => result.push_str("]"),
            Token::CloseCurlyBracket => result.push_str("}"),
            Token::IncludeMatch => result.push_str("~="),
            Token::DashMatch => result.push_str("|="),
            Token::PrefixMatch => result.push_str("^="),
            Token::SuffixMatch => result.push_str("$="),
            Token::SubstringMatch => result.push_str("*="),
            Token::CDO => result.push_str("<!--"),
            Token::CDC => result.push_str("-->"),
            Token::WhiteSpace(ref value) => {
                result.push_str(value);
            }
            // div...
            Token::Ident(ref value) => {
                curr_rule = str!();
                curr_prop = str!(value);
                result.push_str(&escape(value));
            }
            // @import, @font-face, @charset, @media...
            Token::AtKeyword(ref value) => {
                curr_rule = str!(value);
                if opt_no_fonts && curr_rule == "font-face" {
                    continue;
                }
                result.push_str("@");
                result.push_str(value);
            }
            Token::Hash(ref value) => {
                result.push_str("#");
                result.push_str(value);
            }
            Token::QuotedString(ref value) => {
                if curr_rule == "import" {
                    // Reset current at-rule value
                    curr_rule = str!();

                    // Skip empty import values
                    if value.len() < 1 {
                        result.push_str("''");
                        continue;
                    }

                    let import_full_url = resolve_url(&parent_url, value).unwrap_or_default();
                    let import_url_fragment = get_url_fragment(import_full_url.clone());
                    match retrieve_asset(cache, client, &parent_url, &import_full_url, opt_silent) {
                        Ok((import_contents, import_final_url, _import_media_type)) => {
                            result.push_str(
                                enquote(
                                    data_to_data_url(
                                        "text/css",
                                        embed_css(
                                            cache,
                                            client,
                                            &import_final_url,
                                            &String::from_utf8_lossy(&import_contents),
                                            opt_no_fonts,
                                            opt_no_images,
                                            opt_silent,
                                        )
                                        .as_bytes(),
                                        &import_final_url,
                                        &import_url_fragment,
                                    ),
                                    false,
                                )
                                .as_str(),
                            );
                        }
                        Err(_) => {
                            // Keep remote reference if unable to retrieve the asset
                            if is_http_url(import_full_url.clone()) {
                                result.push_str(enquote(import_full_url, false).as_str());
                            }
                        }
                    }
                } else {
                    if func_name == "url" {
                        // Skip empty url()'s
                        if value.len() < 1 {
                            continue;
                        }

                        if opt_no_images && is_image_url_prop(curr_prop.as_str()) {
                            result.push_str(enquote(str!(empty_image!()), false).as_str());
                        } else {
                            let resolved_url = resolve_url(&parent_url, value).unwrap_or_default();
                            let url_fragment = get_url_fragment(resolved_url.clone());
                            match retrieve_asset(
                                cache,
                                client,
                                &parent_url,
                                &resolved_url,
                                opt_silent,
                            ) {
                                Ok((data, final_url, media_type)) => {
                                    let data_url = data_to_data_url(
                                        &media_type,
                                        &data,
                                        &final_url,
                                        &url_fragment,
                                    );
                                    result.push_str(enquote(data_url, false).as_str());
                                }
                                Err(_) => {
                                    // Keep remote reference if unable to retrieve the asset
                                    if is_http_url(resolved_url.clone()) {
                                        result.push_str(enquote(resolved_url, false).as_str());
                                    }
                                }
                            }
                        }
                    } else {
                        result.push_str(enquote(str!(value), false).as_str());
                    }
                }
            }
            Token::Number {
                ref has_sign,
                ref value,
                ..
            } => {
                if *has_sign && *value >= 0. {
                    result.push_str("+");
                }
                result.push_str(&value.to_string())
            }
            Token::Percentage {
                ref has_sign,
                ref unit_value,
                ..
            } => {
                if *has_sign && *unit_value >= 0. {
                    result.push_str("+");
                }
                result.push_str(str!(unit_value * 100.).as_str());
                result.push_str("%");
            }
            Token::Dimension {
                ref has_sign,
                ref value,
                ref unit,
                ..
            } => {
                if *has_sign && *value >= 0. {
                    result.push_str("+");
                }
                result.push_str(str!(value).as_str());
                result.push_str(str!(unit).as_str());
            }
            // #selector, #id...
            Token::IDHash(ref value) => {
                curr_rule = str!();
                result.push_str("#");
                result.push_str(&escape(value));
            }
            Token::UnquotedUrl(ref value) => {
                let is_import: bool = curr_rule == "import";

                if is_import {
                    // Reset current at-rule value
                    curr_rule = str!();
                }

                // Skip empty url()'s
                if value.len() < 1 {
                    result.push_str("url()");
                    continue;
                } else if value.starts_with("#") {
                    result.push_str("url(");
                    result.push_str(value);
                    result.push_str(")");
                    continue;
                }

                result.push_str("url(");
                if is_import {
                    let full_url = resolve_url(&parent_url, value).unwrap_or_default();
                    let url_fragment = get_url_fragment(full_url.clone());
                    match retrieve_asset(cache, client, &parent_url, &full_url, opt_silent) {
                        Ok((css, final_url, _media_type)) => {
                            let data_url = data_to_data_url(
                                "text/css",
                                embed_css(
                                    cache,
                                    client,
                                    &final_url,
                                    &String::from_utf8_lossy(&css),
                                    opt_no_fonts,
                                    opt_no_images,
                                    opt_silent,
                                )
                                .as_bytes(),
                                &final_url,
                                &url_fragment,
                            );
                            result.push_str(enquote(data_url, false).as_str());
                        }
                        Err(_) => {
                            // Keep remote reference if unable to retrieve the asset
                            if is_http_url(full_url.clone()) {
                                result.push_str(enquote(full_url, false).as_str());
                            }
                        }
                    }
                } else {
                    if opt_no_images && is_image_url_prop(curr_prop.as_str()) {
                        result.push_str(enquote(str!(empty_image!()), false).as_str());
                    } else {
                        let full_url = resolve_url(&parent_url, value).unwrap_or_default();
                        let url_fragment = get_url_fragment(full_url.clone());
                        match retrieve_asset(cache, client, &parent_url, &full_url, opt_silent) {
                            Ok((data, final_url, media_type)) => {
                                let data_url =
                                    data_to_data_url(&media_type, &data, &final_url, &url_fragment);
                                result.push_str(enquote(data_url, false).as_str());
                            }
                            Err(_) => {
                                // Keep remote reference if unable to retrieve the asset
                                if is_http_url(full_url.clone()) {
                                    result.push_str(enquote(full_url, false).as_str());
                                }
                            }
                        }
                    }
                }
                result.push_str(")");
            }
            Token::Delim(ref value) => result.push_str(&value.to_string()),
            Token::Function(ref name) => {
                let function_name: &str = &name.clone();
                result.push_str(function_name);
                result.push_str("(");

                let block_css: String = parser
                    .parse_nested_block(|parser| {
                        process_css(
                            cache,
                            client,
                            parent_url,
                            parser,
                            curr_rule.as_str(),
                            curr_prop.as_str(),
                            function_name,
                            opt_no_fonts,
                            opt_no_images,
                            opt_silent,
                        )
                    })
                    .unwrap();
                result.push_str(block_css.as_str());

                result.push_str(")");
            }
            Token::BadUrl(_) | Token::BadString(_) => {}
        }
    }

    Ok(result)
}

pub fn embed_css(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    parent_url: &str,
    css: &str,
    opt_no_fonts: bool,
    opt_no_images: bool,
    opt_silent: bool,
) -> String {
    let mut input = ParserInput::new(&css);
    let mut parser = Parser::new(&mut input);

    process_css(
        cache,
        client,
        parent_url,
        &mut parser,
        "",
        "",
        "",
        opt_no_fonts,
        opt_no_images,
        opt_silent,
    )
    .unwrap()
}
