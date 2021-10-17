//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::html;

    #[test]
    fn meta_content_type() {
        let html = "<!doctype html>
<html>
    <head>
        <meta http-equiv=\"content-type\" content=\"text/html;charset=GB2312\" />
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(html::get_charset(&dom.document), Some("GB2312".to_string()));
    }

    #[test]
    fn meta_charset() {
        let html = "<!doctype html>
<html>
    <head>
        <meta charset=\"GB2312\" />
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(html::get_charset(&dom.document), Some("GB2312".to_string()));
    }

    #[test]
    fn multiple_conflicting_meta_charset_first() {
        let html = "<!doctype html>
<html>
    <head>
        <meta charset=\"utf-8\" />
        <meta http-equiv=\"content-type\" content=\"text/html;charset=GB2312\" />
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(html::get_charset(&dom.document), Some("utf-8".to_string()));
    }
    #[test]
    fn multiple_conflicting_meta_content_type_first() {
        let html = "<!doctype html>
<html>
    <head>
        <meta http-equiv=\"content-type\" content=\"text/html;charset=GB2312\" />
        <meta charset=\"utf-8\" />
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(html::get_charset(&dom.document), Some("GB2312".to_string()));
    }
}
