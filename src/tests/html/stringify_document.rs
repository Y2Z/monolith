//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::html;
    use crate::opts::Options;

    #[test]
    fn div_as_root_element() {
        let html = "<div><script src=\"some.js\"></script></div>";
        let dom = html::html_to_dom(&html);
        let options = Options::default();

        assert_eq!(
            html::stringify_document(&dom.document, &options),
            "<html><head></head><body><div><script src=\"some.js\"></script></div></body></html>"
        );
    }

    #[test]
    fn full_page_with_no_html_head_or_body() {
        let html = "<title>Isolated document</title>\
                    <link rel=\"something\" href=\"some.css\" />\
                    <meta http-equiv=\"Content-Security-Policy\" content=\"default-src https:\">\
                    <div><script src=\"some.js\"></script></div>";
        let dom = html::html_to_dom(&html);
        let mut options = Options::default();
        options.isolate = true;

        assert_eq!(
            html::stringify_document(
                &dom.document,
                &options
            ),
            "<html>\
                <head>\
                    <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-inline' data:;\"></meta>\
                    <title>Isolated document</title>\
                    <link rel=\"something\" href=\"some.css\">\
                    <meta http-equiv=\"Content-Security-Policy\" content=\"default-src https:\">\
                </head>\
                <body>\
                    <div>\
                        <script src=\"some.js\"></script>\
                    </div>\
                </body>\
            </html>"
        );
    }

    #[test]
    fn doctype_and_the_rest_no_html_head_or_body() {
        let html = "<!doctype html>\
                    <title>Unstyled document</title>\
                    <link rel=\"stylesheet\" href=\"main.css\"/>\
                    <div style=\"display: none;\"></div>";
        let dom = html::html_to_dom(&html);
        let mut options = Options::default();
        options.no_css = true;

        assert_eq!(
            html::stringify_document(&dom.document, &options),
            "<!DOCTYPE html>\
            <html>\
            <head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'none';\"></meta>\
            <title>Unstyled document</title>\
            <link rel=\"stylesheet\" href=\"main.css\">\
            </head>\
            <body><div style=\"display: none;\"></div></body>\
            </html>"
        );
    }

    #[test]
    fn doctype_and_the_rest_no_html_head_or_body_forbid_frames() {
        let html = "<!doctype html>\
                    <title>Frameless document</title>\
                    <link rel=\"something\"/>\
                    <div><script src=\"some.js\"></script></div>";
        let dom = html::html_to_dom(&html);
        let mut options = Options::default();
        options.no_frames = true;

        assert_eq!(
            html::stringify_document(
                &dom.document,
                &options
            ),
            "<!DOCTYPE html>\
                <html>\
                <head>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"frame-src 'none'; child-src 'none';\"></meta>\
                <title>Frameless document</title>\
                <link rel=\"something\">\
                </head>\
                <body><div><script src=\"some.js\"></script></div></body>\
                </html>"
        );
    }

    #[test]
    fn doctype_and_the_rest_all_forbidden() {
        let html = "<!doctype html>\
                    <title>no-frame no-css no-js no-image isolated document</title>\
                    <meta http-equiv=\"Content-Security-Policy\" content=\"default-src https:\">\
                    <link rel=\"stylesheet\" href=\"some.css\">\
                    <div>\
                        <script src=\"some.js\"></script>\
                        <img style=\"width: 100%;\" src=\"some.png\" />\
                        <iframe src=\"some.html\"></iframe>\
                    </div>";
        let dom = html::html_to_dom(&html);
        let mut options = Options::default();
        options.isolate = true;
        options.no_css = true;
        options.no_fonts = true;
        options.no_frames = true;
        options.no_js = true;
        options.no_images = true;

        assert_eq!(
            html::stringify_document(
                &dom.document,
                &options
            ),
            "<!DOCTYPE html>\
                <html>\
                    <head>\
                        <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'unsafe-inline' data:; style-src 'none'; font-src 'none'; frame-src 'none'; child-src 'none'; script-src 'none'; img-src data:;\"></meta>\
                        <title>no-frame no-css no-js no-image isolated document</title>\
                        <meta http-equiv=\"Content-Security-Policy\" content=\"default-src https:\">\
                        <link rel=\"stylesheet\" href=\"some.css\">\
                    </head>\
                    <body>\
                        <div>\
                            <script src=\"some.js\"></script>\
                            <img style=\"width: 100%;\" src=\"some.png\">\
                            <iframe src=\"some.html\"></iframe>\
                        </div>\
                    </body>\
                </html>"
        );
    }
}
