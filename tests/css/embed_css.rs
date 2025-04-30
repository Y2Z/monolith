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
    use monolith::css;
    use monolith::session::Session;
    use monolith::url::EMPTY_IMAGE_DATA_URL;

    #[test]
    fn empty_input() {
        let document_url: Url = Url::parse("data:,").unwrap();
        let options = MonolithOptions::default();
        let mut session: Session = Session::new(None, None, options);

        assert_eq!(css::embed_css(&mut session, &document_url, ""), "");
    }

    #[test]
    fn trim_if_empty() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = MonolithOptions::default();
        let mut session: Session = Session::new(None, None, options);

        assert_eq!(
            css::embed_css(&mut session, &document_url, "\t     \t   "),
            ""
        );
    }

    #[test]
    fn style_exclude_unquoted_images() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = MonolithOptions::default();
        options.no_images = true;
        options.silent = true;
        let mut session: Session = Session::new(None, None, options);

        const STYLE: &str = "/* border: none;*/\
            background-image: url(https://somewhere.com/bg.png); \
            list-style: url(/assets/images/bullet.svg);\
            width:99.998%; \
            margin-top: -20px; \
            line-height: -1; \
            height: calc(100vh - 10pt)";

        assert_eq!(
            css::embed_css(&mut session, &document_url, STYLE),
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
        let mut options = MonolithOptions::default();
        options.no_images = true;
        options.silent = true;
        let mut session: Session = Session::new(None, None, options);

        const STYLE: &str = "/* border: none;*/\
            background-image: url('https://somewhere.com/bg.png'); \
            list-style: url('/assets/images/bullet.svg');\
            width:99.998%; \
            margin-top: -20px; \
            line-height: -1; \
            height: calc(100vh - 10pt)";

        assert_eq!(
            css::embed_css(&mut session, &document_url, STYLE),
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
        let mut options = MonolithOptions::default();
        options.silent = true;
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            #id.class-name:not(:nth-child(3n+0)) {\n  \
            // border: none;\n  \
            background-image: url(\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\");\n\
            }\n\
            \n\
            html > body {}";

        assert_eq!(css::embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn attribute_selectors() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = MonolithOptions::default();
        options.silent = true;
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

        assert_eq!(css::embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn import_string() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = MonolithOptions::default();
        options.silent = true;
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            @charset 'UTF-8';\n\
            \n\
            @import 'data:text/css,html{background-color:%23000}';\n\
            \n\
            @import url('data:text/css,html{color:%23fff}')\n\
            ";

        assert_eq!(
            css::embed_css(&mut session, &document_url, CSS),
            "\
            @charset \"UTF-8\";\n\
            \n\
            @import \"data:text/css;base64,aHRtbHtiYWNrZ3JvdW5kLWNvbG9yOiMwMDB9\";\n\
            \n\
            @import url(\"data:text/css;base64,aHRtbHtjb2xvcjojZmZmfQ==\")\n\
            "
        );
    }

    #[test]
    fn hash_urls() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = MonolithOptions::default();
        options.silent = true;
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

        assert_eq!(css::embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn transform_percentages_and_degrees() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = MonolithOptions::default();
        options.silent = true;
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            div {\n    \
                transform: translate(-50%, -50%) rotate(-45deg);\n\
                transform: translate(50%, 50%) rotate(45deg);\n\
                transform: translate(+50%, +50%) rotate(+45deg);\n\
            }\n\
            ";

        assert_eq!(css::embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn unusual_indents() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = MonolithOptions::default();
        options.silent = true;
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

        assert_eq!(css::embed_css(&mut session, &document_url, CSS), CSS);
    }

    #[test]
    fn exclude_fonts() {
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = MonolithOptions::default();
        options.no_fonts = true;
        options.silent = true;
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

        assert_eq!(css::embed_css(&mut session, &document_url, CSS), CSS_OUT);
    }

    #[test]
    fn content() {
        let document_url: Url = Url::parse("data:,").unwrap();
        let mut options = MonolithOptions::default();
        options.silent = true;
        let mut session: Session = Session::new(None, None, options);

        const CSS: &str = "\
            #language a[href=\"#translations\"]:before {\n\
                content: url(data:,) \"\\A\";\n\
                white-space: pre }\n\
            ";
        const CSS_OUT: &str = "\
            #language a[href=\"#translations\"]:before {\n\
                content: url(\"data:text/plain;base64,\") \"\\a \";\n\
                white-space: pre }\n\
            ";

        assert_eq!(css::embed_css(&mut session, &document_url, CSS), CSS_OUT);
    }

    #[test]
    fn ie_css_hack() {
        let document_url: Url = Url::parse("data:,").unwrap();
        let mut options = MonolithOptions::default();
        options.silent = true;
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

        assert_eq!(css::embed_css(&mut session, &document_url, CSS), CSS_OUT);
    }
}
