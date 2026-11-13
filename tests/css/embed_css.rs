//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use monolith::core::MonolithOptions;
    use monolith::css::{embed_css, embed_stylesheet};
    use monolith::session::Session;
    use monolith::url::EMPTY_IMAGE_DATA_URL;

    /// Extracts the first data URL out of serialized CSS and decodes it
    fn first_embedded_data_url(css: &str) -> (String, String) {
        let start = css.find("data:").expect("no data URL found");
        let end = start + css[start..].find('"').expect("unterminated data URL");
        let (_, charset, data) =
            monolith::url::parse_data_url(&Url::parse(&css[start..end]).unwrap());
        (
            charset,
            String::from_utf8(data).expect("embedded CSS is not UTF-8"),
        )
    }

    #[test]
    fn imported_stylesheet_encoding() {
        let url = Url::parse("https://example.com/").unwrap();

        for (charset, bytes) in [
            ("windows-1252", b"p::before{content:'caf\xe9'}".as_slice()),
            ("utf-8", "p::before{content:'café'}".as_bytes()),
            ("", "p::before{content:'café'}".as_bytes()),
            ("", b"p::before{content:'caf\xe9'}".as_slice()),
        ] {
            // Base64, which can go anywhere in CSS, including the unquoted url() below
            let mut asset = dataurl::DataUrl::new();
            asset.set_media_type(Some("text/css")).unwrap();
            if !charset.is_empty() {
                asset.set_parameter("charset", Some(charset)).unwrap();
            }
            asset.set_data_encoding(dataurl::DataEncoding::Base64);
            asset.set_data(bytes);

            for source in [
                format!("@import \"{}\";", asset),
                format!("@import url({});", asset),
                format!("@import url(\"{}\");", asset),
            ] {
                let mut session = Session::new(None, None, MonolithOptions::default());
                let result = embed_css(&mut session, &url, &source);

                assert_eq!(
                    first_embedded_data_url(&result),
                    (
                        "utf-8".to_string(),
                        "p::before{content:\"café\"}".to_string()
                    ),
                    "charset {:?}, source {}",
                    charset,
                    source
                );
            }
        }
    }

    #[test]
    fn imported_stylesheet_charset_rule() {
        let url = Url::parse("https://example.com/").unwrap();
        let asset = monolith::url::create_data_url(
            "text/css",
            "",
            b"@charset \"windows-1252\";p::before{content:'caf\xe9'}",
            &url,
            None,
        );
        let mut session = Session::new(None, None, MonolithOptions::default());
        let result = embed_css(&mut session, &url, &format!("@import \"{}\";", asset));

        assert_eq!(
            first_embedded_data_url(&result),
            (
                "utf-8".to_string(),
                "@charset \"UTF-8\";p::before{content:\"café\"}".to_string()
            )
        );
    }

    #[test]
    fn imported_stylesheet_environment_encoding() {
        let url = Url::parse("https://example.com/").unwrap();
        let asset = monolith::url::create_data_url(
            "text/css",
            "",
            b"p{font-family:'\x83\x81\x83\x43\x83\x8a\x83\x49'}",
            &url,
            None,
        );
        let mut session = Session::new(None, None, MonolithOptions::default());
        session.environment_encoding = Some(encoding_rs::SHIFT_JIS);

        let result = embed_css(&mut session, &url, &format!("@import \"{}\";", asset));

        assert_eq!(
            first_embedded_data_url(&result),
            (
                "utf-8".to_string(),
                "p{font-family:\"メイリオ\"}".to_string()
            )
        );
        assert_eq!(session.environment_encoding, Some(encoding_rs::SHIFT_JIS));
    }

    #[test]
    fn nested_import_inherits_importing_stylesheet_encoding() {
        let url = Url::parse("https://example.com/").unwrap();
        // EUC-JP "両", which also happens to be valid UTF-8 ("ξ")
        let inner = monolith::url::create_data_url(
            "text/css",
            "",
            b"p::before{content:'\xce\xbe'}",
            &url,
            None,
        );
        let outer = monolith::url::create_data_url(
            "text/css",
            "",
            format!("@charset \"EUC-JP\";@import \"{}\";", inner).as_bytes(),
            &url,
            None,
        );
        let mut session = Session::new(None, None, MonolithOptions::default());
        session.environment_encoding = Some(encoding_rs::SHIFT_JIS);

        let result = embed_css(&mut session, &url, &format!("@import \"{}\";", outer));

        let (outer_charset, outer_css) = first_embedded_data_url(&result);
        assert_eq!(outer_charset, "utf-8");
        assert!(outer_css.starts_with("@charset \"UTF-8\";@import \"data:"));
        assert_eq!(
            first_embedded_data_url(&outer_css),
            ("utf-8".to_string(), "p::before{content:\"両\"}".to_string())
        );
        assert_eq!(session.environment_encoding, Some(encoding_rs::SHIFT_JIS));
    }

    #[test]
    fn empty_input() {
        let document_url: Url = Url::parse("data:,").unwrap();
        let options = MonolithOptions::default();
        let mut session: Session = Session::new(None, None, options);

        assert_eq!(embed_css(&mut session, &document_url, ""), "");
    }

    #[test]
    fn trim_if_empty() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions::default();
        let mut session: Session = Session::new(None, None, options);

        assert_eq!(embed_css(&mut session, &document_url, "\t     \t   "), "");
    }

    #[test]
    fn style_exclude_unquoted_images() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions {
            remove_images: true,
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const STYLE: &str = "/* border: none;*/\
            background-image: url(https://somewhere.com/bg.png); \
            list-style: url(/assets/images/bullet.svg);\
            width:99.998%; \
            margin-top: -20px; \
            line-height: -1; \
            height: calc(100vh - 10pt)";

        assert_eq!(
            embed_css(&mut session, &document_url, STYLE),
            format!(
                "/* border: none;*/\
                background-image: url(\"{empty_image}\"); \
                list-style: url(\"{empty_image}\");\
                width:99.998%; \
                margin-top: -20px; \
                line-height: -1; \
                height: calc(100vh - 10pt)",
                empty_image = EMPTY_IMAGE_DATA_URL
            )
        );
    }

    #[test]
    fn style_exclude_single_quoted_images() {
        let document_url: Url = Url::parse("data:,").unwrap();
        let options = MonolithOptions {
            remove_images: true,
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const STYLE: &str = "/* border: none;*/\
            background-image: url('https://somewhere.com/bg.png'); \
            list-style: url('/assets/images/bullet.svg');\
            width:99.998%; \
            margin-top: -20px; \
            line-height: -1; \
            height: calc(100vh - 10pt)";

        assert_eq!(
            embed_css(&mut session, &document_url, STYLE),
            format!(
                "/* border: none;*/\
                background-image: url(\"{empty_image}\"); \
                list-style: url(\"{empty_image}\");\
                width:99.998%; \
                margin-top: -20px; \
                line-height: -1; \
                height: calc(100vh - 10pt)",
                empty_image = EMPTY_IMAGE_DATA_URL
            )
        );
    }

    #[test]
    fn style_block() {
        let document_url: Url = Url::parse("file:///").unwrap();
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            #id.class-name:not(:nth-child(3n+0)) {\n  \
            // border: none;\n  \
            background-image: url(\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\");\n\
            }\n\
            \n\
            html > body {}";

        assert_eq!(embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn attribute_selectors() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            [data-value] {
                /* Attribute exists */
            }

            [data-value=\"foo\"] {
                /* Attribute has this exact value */
            }

            [data-value*=\"foo\"] {
                /* Attribute value contains this value somewhere in it */
            }

            [data-value~=\"foo\"] {
                /* Attribute has this value in a space-separated list somewhere */
            }

            [data-value^=\"foo\"] {
                /* Attribute value starts with this */
            }

            [data-value|=\"foo\"] {
                /* Attribute value starts with this in a dash-separated list */
            }

            [data-value$=\"foo\"] {
                /* Attribute value ends with this */
            }
            ";

        assert_eq!(embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn import_string() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            @charset 'UTF-8';\n\
            \n\
            @import 'data:text/css,html{background-color:%23000}';\n\
            \n\
            @import url('data:text/css,html{color:%23fff}')\n\
            ";

        assert_eq!(
            embed_css(&mut session, &document_url, CSS),
            "\
            @charset \"UTF-8\";\n\
            \n\
            @import \"data:text/css;charset=utf-8,html{background-color:%23000}\";\n\
            \n\
            @import url(\"data:text/css;charset=utf-8,html{color:%23fff}\")\n\
            "
        );
    }

    #[test]
    fn hash_urls() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            body {\n    \
                behavior: url(#default#something);\n\
            }\n\
            \n\
            .scissorHalf {\n    \
                offset-path: url(#somePath);\n\
            }\n\
            ";

        assert_eq!(embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn transform_percentages_and_degrees() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            div {\n    \
                transform: translate(-50%, -50%) rotate(-45deg);\n\
                transform: translate(50%, 50%) rotate(45deg);\n\
                transform: translate(+50%, +50%) rotate(+45deg);\n\
            }\n\
            ";

        assert_eq!(embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn unusual_indents() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            .is\\:good:hover {\n    \
                color: green\n\
            }\n\
            \n\
            #\\~\\!\\@\\$\\%\\^\\&\\*\\(\\)\\+\\=\\,\\.\\/\\\\\\'\\\"\\;\\:\\?\\>\\<\\[\\]\\{\\}\\|\\`\\# {\n    \
                color: black\n\
            }\n\
            ";

        assert_eq!(embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn exclude_fonts() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions {
            remove_fonts: true,
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            @font-face {\n    \
                font-family: 'My Font';\n    \
                src: url(my_font.woff);\n\
            }\n\
            \n\
            #identifier {\n    \
                font-family: 'My Font' Arial\n\
            }\n\
            \n\
            @font-face {\n    \
                font-family: 'My Font';\n    \
                src: url(my_font.woff);\n\
            }\n\
            \n\
            div {\n    \
                font-family: 'My Font' Verdana\n\
            }\n\
            ";
        const CSS_OUT: &str = " \
            \n\
            \n\
            #identifier {\n    \
                font-family: \"My Font\" Arial\n\
            }\n\
            \n \
            \n\
            \n\
            div {\n    \
                font-family: \"My Font\" Verdana\n\
            }\n\
            ";

        assert_eq!(embed_css(&mut session, &document_url, CSS), CSS_OUT);
    }

    #[test]
    fn content() {
        let document_url: Url = Url::parse("data:,").unwrap();
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            #language a[href=\"#translations\"]:before {\n\
                content: url(data:,) \"\\A\";\n\
                white-space: pre }\n\
            ";
        const CSS_OUT: &str = "\
            #language a[href=\"#translations\"]:before {\n\
                content: url(\"data:text/plain,\") \"\\a \";\n\
                white-space: pre }\n\
            ";

        assert_eq!(embed_css(&mut session, &document_url, CSS), CSS_OUT);
    }

    #[test]
    fn self_importing_stylesheet() {
        let url = Url::parse("https://example.com/site.css").unwrap();
        let self_importing = "@import url(\"site.css\");body{color:#123456}";
        let mut session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &url,
            &self_importing.as_bytes().to_vec(),
            "text/css".to_string(),
            "".to_string(),
        );

        let result = embed_stylesheet(&mut session, &url, self_importing.as_bytes(), "");

        // The rule from the stylesheet that imports itself is kept, and the cyclic
        // import is replaced with an empty stylesheet
        assert!(result.contains("body{color:#123456}"), "{result}");
        assert_eq!(result.matches("data:text/css").count(), 1, "{result}");
    }

    #[test]
    fn mutually_importing_stylesheets() {
        let url_a = Url::parse("https://example.com/a.css").unwrap();
        let url_b = Url::parse("https://example.com/b.css").unwrap();
        let mut session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &url_b,
            &b"@import \"a.css\";.b{color:blue}".to_vec(),
            "text/css".to_string(),
            "".to_string(),
        );
        session.add_asset(
            &url_a,
            &b"@import \"b.css\";.a{color:red}".to_vec(),
            "text/css".to_string(),
            "".to_string(),
        );

        let result = embed_stylesheet(&mut session, &url_a, b"@import \"b.css\";.a{color:red}", "");

        // Rules from both stylesheets are kept, and the import that closes the cycle is empty
        assert_eq!(result.matches(".a{color:red}").count(), 1, "{result}");
        assert_eq!(result.matches(".b{color:blue}").count(), 1, "{result}");
    }

    #[test]
    fn sibling_imports_of_same_stylesheet_both_embed() {
        let document_url = Url::parse("https://example.com/page.html").unwrap();
        let stylesheet_url = Url::parse("https://example.com/shared.css").unwrap();
        let mut session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );
        session.add_asset(
            &stylesheet_url,
            &b".shared{color:green}".to_vec(),
            "text/css".to_string(),
            "".to_string(),
        );

        let result = embed_css(
            &mut session,
            &document_url,
            "@import \"shared.css\";@import \"shared.css\";",
        );

        // Importing the same stylesheet more than once is not a cycle
        assert_eq!(
            result.matches(".shared{color:green}").count(),
            2,
            "{result}"
        );
    }

    #[test]
    fn deep_import_chain_stops_at_max_depth() {
        let document_url = Url::parse("https://example.com/page.html").unwrap();
        let depth = 40;
        let mut session = Session::new(
            None,
            None,
            MonolithOptions {
                silent: true,
                ..Default::default()
            },
        );

        for level in 0..depth {
            let url = Url::parse(&format!("https://example.com/s{level}.css")).unwrap();
            let css = if level + 1 < depth {
                format!("@import \"s{}.css\";.level{level}{{color:red}}", level + 1)
            } else {
                format!(".level{level}{{color:red}}")
            };
            session.add_asset(
                &url,
                &css.into_bytes(),
                "text/css".to_string(),
                "".to_string(),
            );
        }

        let result = embed_css(&mut session, &document_url, "@import \"s0.css\";");

        // Nesting is bounded, so levels beyond the limit never get embedded
        assert!(result.contains(".level0{color:red}"), "{result}");
        assert!(
            !result.contains(&format!(".level{}{{color:red}}", depth - 1)),
            "{result}"
        );
    }

    #[test]
    fn ie_css_hack() {
        let document_url: Url = Url::parse("data:,").unwrap();
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            div#p>svg>foreignObject>section:not(\\9) {\n\
                width: 300px;\n\
                width: 500px\\9;\n\
            }\n\
            ";
        const CSS_OUT: &str = "\
            div#p>svg>foreignObject>section:not(\\9) {\n\
                width: 300px;\n\
                width: 500px\t;\n\
            }\n\
            ";

        assert_eq!(embed_css(&mut session, &document_url, CSS), CSS_OUT);
    }
}
