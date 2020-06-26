//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::html;

    #[test]
    fn isolated() {
        let opt_isolate: bool = true;
        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let csp_content = html::csp(
            opt_isolate,
            opt_no_css,
            opt_no_frames,
            opt_no_js,
            opt_no_images,
        );

        assert_eq!(csp_content, "default-src 'unsafe-inline' data:;");
    }

    #[test]
    fn no_css() {
        let opt_isolate: bool = false;
        let opt_no_css: bool = true;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let csp_content = html::csp(
            opt_isolate,
            opt_no_css,
            opt_no_frames,
            opt_no_js,
            opt_no_images,
        );

        assert_eq!(csp_content, "style-src 'none';");
    }

    #[test]
    fn no_frames() {
        let opt_isolate: bool = false;
        let opt_no_css: bool = false;
        let opt_no_frames: bool = true;
        let opt_no_js: bool = false;
        let opt_no_images: bool = false;
        let csp_content = html::csp(
            opt_isolate,
            opt_no_css,
            opt_no_frames,
            opt_no_js,
            opt_no_images,
        );

        assert_eq!(csp_content, "frame-src 'none'; child-src 'none';");
    }

    #[test]
    fn no_js() {
        let opt_isolate: bool = false;
        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = true;
        let opt_no_images: bool = false;
        let csp_content = html::csp(
            opt_isolate,
            opt_no_css,
            opt_no_frames,
            opt_no_js,
            opt_no_images,
        );

        assert_eq!(csp_content, "script-src 'none';");
    }

    #[test]
    fn no_image() {
        let opt_isolate: bool = false;
        let opt_no_css: bool = false;
        let opt_no_frames: bool = false;
        let opt_no_js: bool = false;
        let opt_no_images: bool = true;
        let csp_content = html::csp(
            opt_isolate,
            opt_no_css,
            opt_no_frames,
            opt_no_js,
            opt_no_images,
        );

        assert_eq!(csp_content, "img-src data:;");
    }

    #[test]
    fn all() {
        let opt_isolate: bool = true;
        let opt_no_css: bool = true;
        let opt_no_frames: bool = true;
        let opt_no_js: bool = true;
        let opt_no_images: bool = true;
        let csp_content = html::csp(
            opt_isolate,
            opt_no_css,
            opt_no_frames,
            opt_no_js,
            opt_no_images,
        );

        assert_eq!(csp_content, "default-src 'unsafe-inline' data:; style-src 'none'; frame-src 'none'; child-src 'none'; script-src 'none'; img-src data:;");
    }
}
