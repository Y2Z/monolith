//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::blocking::Client;
    use reqwest::Url;
    use std::collections::HashMap;

    use crate::css;
    use crate::opts::Options;

    #[test]
    fn empty_input() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("data:,").unwrap();
        let options = Options::default();

        assert_eq!(
            css::embed_css(cache, &client, &document_url, "", &options, 0),
            ""
        );
    }

    #[test]
    fn trim_if_empty() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let options = Options::default();

        assert_eq!(
            css::embed_css(cache, &client, &document_url, "\t     \t   ", &options, 0,),
            ""
        );
    }

    #[test]
    fn style_exclude_unquoted_images() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;

        const STYLE: &str = "/* border: none;*/\
            background-image: url(https://somewhere.com/bg.png); \
            list-style: url(/assets/images/bullet.svg);\
            width:99.998%; \
            margin-top: -20px; \
            line-height: -1; \
            height: calc(100vh - 10pt)";

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &STYLE, &options, 0,),
            format!(
                "/* border: none;*/\
                background-image: url(\"{empty_image}\"); \
                list-style: url(\"{empty_image}\");\
                width:99.998%; \
                margin-top: -20px; \
                line-height: -1; \
                height: calc(100vh - 10pt)",
                empty_image = empty_image!()
            )
        );
    }

    #[test]
    fn style_exclude_single_quoted_images() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("data:,").unwrap();
        let mut options = Options::default();
        options.no_images = true;
        options.silent = true;

        const STYLE: &str = "/* border: none;*/\
            background-image: url('https://somewhere.com/bg.png'); \
            list-style: url('/assets/images/bullet.svg');\
            width:99.998%; \
            margin-top: -20px; \
            line-height: -1; \
            height: calc(100vh - 10pt)";

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &STYLE, &options, 0),
            format!(
                "/* border: none;*/\
                background-image: url(\"{empty_image}\"); \
                list-style: url(\"{empty_image}\");\
                width:99.998%; \
                margin-top: -20px; \
                line-height: -1; \
                height: calc(100vh - 10pt)",
                empty_image = empty_image!()
            )
        );
    }

    #[test]
    fn style_block() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("file:///").unwrap();
        let mut options = Options::default();
        options.silent = true;

        const CSS: &str = "\
            #id.class-name:not(:nth-child(3n+0)) {\n  \
            // border: none;\n  \
            background-image: url(\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\");\n\
            }\n\
            \n\
            html > body {}";

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0),
            CSS
        );
    }

    #[test]
    fn attribute_selectors() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = Options::default();
        options.silent = true;

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

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0),
            CSS
        );
    }

    #[test]
    fn import_string() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = Options::default();
        options.silent = true;

        const CSS: &str = "\
            @charset 'UTF-8';\n\
            \n\
            @import 'data:text/css,html{background-color:%23000}';\n\
            \n\
            @import url('data:text/css,html{color:%23fff}')\n\
            ";

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0,),
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
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = Options::default();
        options.silent = true;

        const CSS: &str = "\
            body {\n    \
                behavior: url(#default#something);\n\
            }\n\
            \n\
            .scissorHalf {\n    \
                offset-path: url(#somePath);\n\
            }\n\
            ";

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0,),
            CSS
        );
    }

    #[test]
    fn transform_percentages_and_degrees() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = Options::default();
        options.silent = true;

        const CSS: &str = "\
            div {\n    \
                transform: translate(-50%, -50%) rotate(-45deg);\n\
                transform: translate(50%, 50%) rotate(45deg);\n\
                transform: translate(+50%, +50%) rotate(+45deg);\n\
            }\n\
            ";

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0,),
            CSS
        );
    }

    #[test]
    fn unusual_indents() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = Options::default();
        options.silent = true;

        const CSS: &str = "\
            .is\\:good:hover {\n    \
                color: green\n\
            }\n\
            \n\
            #\\~\\!\\@\\$\\%\\^\\&\\*\\(\\)\\+\\=\\,\\.\\/\\\\\\'\\\"\\;\\:\\?\\>\\<\\[\\]\\{\\}\\|\\`\\# {\n    \
                color: black\n\
            }\n\
            ";

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0,),
            CSS
        );
    }

    #[test]
    fn exclude_fonts() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("https://doesntmatter.local/").unwrap();
        let mut options = Options::default();
        options.no_fonts = true;
        options.silent = true;

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

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0,),
            CSS_OUT
        );
    }

    #[test]
    fn content() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("data:,").unwrap();
        let mut options = Options::default();
        options.silent = true;

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

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0,),
            CSS_OUT
        );
    }

    #[test]
    fn ie_css_hack() {
        let cache = &mut HashMap::new();
        let client = Client::new();
        let document_url: Url = Url::parse("data:,").unwrap();
        let mut options = Options::default();
        options.silent = true;

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

        assert_eq!(
            css::embed_css(cache, &client, &document_url, &CSS, &options, 0,),
            CSS_OUT
        );
    }
}
