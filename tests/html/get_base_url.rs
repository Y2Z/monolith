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
    fn present() {
        let html = "<!doctype html>
<html>
    <head>
        <base href=\"https://musicbrainz.org\" />
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(
            html::get_base_url(&dom.document),
            Some("https://musicbrainz.org".to_string())
        );
    }

    #[test]
    fn multiple_tags() {
        let html = "<!doctype html>
<html>
    <head>
        <base href=\"https://www.discogs.com/\" />
        <base href=\"https://musicbrainz.org\" />
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(
            html::get_base_url(&dom.document),
            Some("https://www.discogs.com/".to_string())
        );
    }
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use monolith::html;

    #[test]
    fn absent() {
        let html = "<!doctype html>
<html>
    <head>
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(html::get_base_url(&dom.document), None);
    }

    #[test]
    fn no_href() {
        let html = "<!doctype html>
<html>
    <head>
        <base />
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(html::get_base_url(&dom.document), None);
    }

    #[test]
    fn empty_href() {
        let html = "<!doctype html>
<html>
    <head>
        <base href=\"\" />
    </head>
    <body>
    </body>
</html>";
        let dom = html::html_to_dom(&html.as_bytes().to_vec(), "".to_string());

        assert_eq!(html::get_base_url(&dom.document), Some("".to_string()));
    }
}
