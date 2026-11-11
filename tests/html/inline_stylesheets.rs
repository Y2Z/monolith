#[cfg(test)]
mod helpers {
    use base64::Engine;
    use monolith::core::MonolithOptions;
    use monolith::html::{html_to_dom, inline_stylesheets, serialize_document};

    pub fn stylesheet(css: &str) -> String {
        format!(
            "data:text/css;base64,{}",
            base64::prelude::BASE64_STANDARD.encode(css)
        )
    }

    pub fn inline(html: &str) -> String {
        let dom = html_to_dom(html.as_bytes(), "utf-8".to_string());
        inline_stylesheets(&dom, &dom.document);

        String::from_utf8(serialize_document(
            dom,
            "utf-8".to_string(),
            &MonolithOptions::default(),
        ))
        .unwrap()
    }
}

#[cfg(test)]
mod passing {
    use super::helpers::{inline, stylesheet};

    #[test]
    fn stylesheets_become_style_elements() {
        let result = inline(&format!(
            "<link rel=stylesheet href=\"{}\" media=print title=Printed id=main \
             crossorigin=anonymous integrity=sha384-x type=text/css>",
            stylesheet("p { color: red }")
        ));

        assert!(
            result.contains(
                "<style media=\"print\" title=\"Printed\" id=\"main\">p { color: red }</style>"
            ),
            "{}",
            result
        );
        assert!(!result.contains("<link"));
    }

    #[test]
    fn inside_of_everything_else_too() {
        let result = inline(&format!(
            "<body><div><template shadowrootmode=open><link rel=stylesheet href=\"{}\"></template></div>",
            stylesheet("b {}")
        ));

        assert!(result.contains("<style>b {}</style>"), "{}", result);
    }

    #[test]
    fn rel_in_any_case() {
        let result = inline(&format!(
            "<link rel=StyleSheet href=\"{}\">",
            stylesheet("i {}")
        ));

        assert!(result.contains("<style>i {}</style>"));
    }
}

#[cfg(test)]
mod failing {
    use super::helpers::{inline, stylesheet};

    #[test]
    fn links_that_can_tell_the_difference_stay() {
        let css = stylesheet("p {}");
        for attributes in [
            "rel=stylesheet onload=\"ready()\"",
            "rel=stylesheet onerror=\"fallback()\"",
            "rel=\"alternate stylesheet\" title=Other",
            "rel=stylesheet disabled",
            "rel=preload as=style",
        ] {
            let result = inline(&format!("<link {} href=\"{}\">", attributes, css));

            assert!(!result.contains("<style"), "{}", attributes);
        }
    }

    #[test]
    fn what_would_end_style_elements_early() {
        let result = inline(&format!(
            "<link rel=stylesheet href=\"{}\">",
            stylesheet("p::after { content: '</style><script>alert(1)</script>' }")
        ));

        assert!(!result.contains("<style"));
    }

    #[test]
    fn what_is_not_a_stylesheet_in_a_data_url() {
        for href in ["https://a.test/a.css", "data:text/plain,p{}", "not a url"] {
            let result = inline(&format!("<link rel=stylesheet href=\"{}\">", href));

            assert!(!result.contains("<style"), "{}", href);
        }
    }
}
