use cssparser::{
    serialize_identifier, serialize_string, ParseError, Parser, ParserInput, SourcePosition, Token,
};
use reqwest::blocking::Client;
use std::collections::HashMap;
use url::Url;

use crate::opts::Options;
use crate::url::{create_data_url, resolve_url};
use crate::utils::retrieve_asset;

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

pub fn embed_css(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    document_url: &Url,
    css: &str,
    options: &Options,
    depth: u32,
) -> String {
    let mut input = ParserInput::new(&css);
    let mut parser = Parser::new(&mut input);

    process_css(
        cache,
        client,
        document_url,
        &mut parser,
        options,
        depth,
        "",
        "",
        "",
    )
    .unwrap()
}

pub fn format_ident(ident: &str) -> String {
    let mut res: String = String::new();
    let _ = serialize_identifier(ident, &mut res);
    res = res.trim_end().to_string();
    res
}

pub fn format_quoted_string(string: &str) -> String {
    let mut res: String = String::new();
    let _ = serialize_string(string, &mut res);
    res
}

pub fn is_image_url_prop(prop_name: &str) -> bool {
    CSS_PROPS_WITH_IMAGE_URLS
        .iter()
        .find(|p| prop_name.eq_ignore_ascii_case(p))
        .is_some()
}

pub fn process_css<'a>(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    document_url: &Url,
    parser: &mut Parser,
    options: &Options,
    depth: u32,
    rule_name: &str,
    prop_name: &str,
    func_name: &str,
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
                if options.no_fonts && curr_rule == "font-face" {
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
                            document_url,
                            parser,
                            options,
                            depth,
                            rule_name,
                            curr_prop.as_str(),
                            func_name,
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
                result.push_str(&format_ident(value));
            }
            // @import, @font-face, @charset, @media...
            Token::AtKeyword(ref value) => {
                curr_rule = str!(value);
                if options.no_fonts && curr_rule == "font-face" {
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
                    if value.len() == 0 {
                        result.push_str("''");
                        continue;
                    }

                    let import_full_url: Url = resolve_url(&document_url, value);
                    match retrieve_asset(
                        cache,
                        client,
                        &document_url,
                        &import_full_url,
                        options,
                        depth + 1,
                    ) {
                        Ok((
                            import_contents,
                            import_final_url,
                            import_media_type,
                            import_charset,
                        )) => {
                            let mut import_data_url = create_data_url(
                                &import_media_type,
                                &import_charset,
                                embed_css(
                                    cache,
                                    client,
                                    &import_final_url,
                                    &String::from_utf8_lossy(&import_contents),
                                    options,
                                    depth + 1,
                                )
                                .as_bytes(),
                                &import_final_url,
                            );
                            import_data_url.set_fragment(import_full_url.fragment());
                            result.push_str(
                                format_quoted_string(&import_data_url.to_string()).as_str(),
                            );
                        }
                        Err(_) => {
                            // Keep remote reference if unable to retrieve the asset
                            if import_full_url.scheme() == "http"
                                || import_full_url.scheme() == "https"
                            {
                                result.push_str(
                                    format_quoted_string(&import_full_url.to_string()).as_str(),
                                );
                            }
                        }
                    }
                } else {
                    if func_name == "url" {
                        // Skip empty url()'s
                        if value.len() == 0 {
                            continue;
                        }

                        if options.no_images && is_image_url_prop(curr_prop.as_str()) {
                            result.push_str(format_quoted_string(empty_image!()).as_str());
                        } else {
                            let resolved_url: Url = resolve_url(&document_url, value);
                            match retrieve_asset(
                                cache,
                                client,
                                &document_url,
                                &resolved_url,
                                options,
                                depth + 1,
                            ) {
                                Ok((data, final_url, media_type, charset)) => {
                                    let mut data_url =
                                        create_data_url(&media_type, &charset, &data, &final_url);
                                    data_url.set_fragment(resolved_url.fragment());
                                    result.push_str(
                                        format_quoted_string(&data_url.to_string()).as_str(),
                                    );
                                }
                                Err(_) => {
                                    // Keep remote reference if unable to retrieve the asset
                                    if resolved_url.scheme() == "http"
                                        || resolved_url.scheme() == "https"
                                    {
                                        result.push_str(
                                            format_quoted_string(&resolved_url.to_string())
                                                .as_str(),
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        result.push_str(format_quoted_string(value).as_str());
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
                result.push_str(str!(unit_value * 100.0).as_str());
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
                result.push_str(&format_ident(value));
            }
            // url()
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
                    let full_url: Url = resolve_url(&document_url, value);
                    match retrieve_asset(
                        cache,
                        client,
                        &document_url,
                        &full_url,
                        options,
                        depth + 1,
                    ) {
                        Ok((css, final_url, media_type, charset)) => {
                            let mut data_url = create_data_url(
                                &media_type,
                                &charset,
                                embed_css(
                                    cache,
                                    client,
                                    &final_url,
                                    &String::from_utf8_lossy(&css),
                                    options,
                                    depth + 1,
                                )
                                .as_bytes(),
                                &final_url,
                            );
                            data_url.set_fragment(full_url.fragment());
                            result.push_str(format_quoted_string(&data_url.to_string()).as_str());
                        }
                        Err(_) => {
                            // Keep remote reference if unable to retrieve the asset
                            if full_url.scheme() == "http" || full_url.scheme() == "https" {
                                result
                                    .push_str(format_quoted_string(&full_url.to_string()).as_str());
                            }
                        }
                    }
                } else {
                    if is_image_url_prop(curr_prop.as_str()) && options.no_images {
                        result.push_str(format_quoted_string(empty_image!()).as_str());
                    } else {
                        let full_url: Url = resolve_url(&document_url, value);
                        match retrieve_asset(
                            cache,
                            client,
                            &document_url,
                            &full_url,
                            options,
                            depth + 1,
                        ) {
                            Ok((data, final_url, media_type, charset)) => {
                                let mut data_url =
                                    create_data_url(&media_type, &charset, &data, &final_url);
                                data_url.set_fragment(full_url.fragment());
                                result
                                    .push_str(format_quoted_string(&data_url.to_string()).as_str());
                            }
                            Err(_) => {
                                // Keep remote reference if unable to retrieve the asset
                                if full_url.scheme() == "http" || full_url.scheme() == "https" {
                                    result.push_str(
                                        format_quoted_string(&full_url.to_string()).as_str(),
                                    );
                                }
                            }
                        }
                    }
                }
                result.push_str(")");
            }
            // =
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
                            document_url,
                            parser,
                            options,
                            depth,
                            curr_rule.as_str(),
                            curr_prop.as_str(),
                            function_name,
                        )
                    })
                    .unwrap();
                result.push_str(block_css.as_str());

                result.push_str(")");
            }
            Token::BadUrl(_) | Token::BadString(_) => {}
        }
    }

    // Ensure empty CSS is really empty
    if result.len() > 0 && result.trim().len() == 0 {
        result = result.trim().to_string()
    }

    Ok(result)
}
