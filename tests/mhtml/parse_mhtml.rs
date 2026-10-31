//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use std::fs;
    use url::Url;

    use monolith::mhtml::parse_mhtml;

    #[test]
    fn chrome_archive() {
        let data = fs::read("tests/_data_/mhtml/chrome.mhtml").unwrap();
        let archive = parse_mhtml(&data, None).unwrap();

        assert_eq!(archive.len(), 5);

        let root = archive.root();
        assert_eq!(
            root.url.as_ref().unwrap().as_str(),
            "https://example.com/page/"
        );
        assert_eq!(root.media_type, "text/html");
        assert_eq!(
            String::from_utf8_lossy(&root.data),
            r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Archived page</title><link rel="stylesheet" href="style.css"></head><body><img src="img.png"><img src="https://example.com/missing.png"><iframe src="cid:frame-2@mhtml.blink"></iframe></body></html>"#
        );

        let css = archive
            .get(&Url::parse("https://example.com/page/style.css#fragment").unwrap())
            .unwrap();
        assert_eq!(css.media_type, "text/css");
        assert_eq!(css.data, b"body { background: url(\"bg.gif\"); }");

        let png = archive
            .get(&Url::parse("https://example.com/page/img.png").unwrap())
            .unwrap();
        assert_eq!(png.media_type, "image/png");
        assert_eq!(png.data, b"\x89PNG\r\n\x1a\n");

        assert!(!archive.contains(&Url::parse("https://example.com/missing.png").unwrap()));
    }

    #[test]
    fn frame_by_content_id() {
        let data = fs::read("tests/_data_/mhtml/chrome.mhtml").unwrap();
        let archive = parse_mhtml(&data, None).unwrap();

        let frame = archive
            .get(&Url::parse("cid:frame-2@mhtml.blink").unwrap())
            .unwrap();
        // Content-Location comes along, so that the frame's own relative URLs can be resolved
        assert_eq!(
            frame.url.as_ref().unwrap().as_str(),
            "https://example.com/frame.html"
        );
        assert_eq!(
            frame.data,
            b"<html><body><p>Hello from frame</p></body></html>"
        );

        // cid: URLs are allowed to be percent-encoded
        assert!(archive.contains(&Url::parse("cid:frame-2%40mhtml.blink").unwrap()));
    }

    #[test]
    fn crlf_line_breaks() {
        let data = b"MIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/html\r\nContent-Location: https://example.com/\r\n\r\n<p>\r\n</p>\r\n--b\r\nContent-Type: text/plain\r\nContent-Transfer-Encoding: quoted-printable\r\nContent-Location: https://example.com/a.txt\r\n\r\nsoft=\r\n break, =E2=9C=93\r\n--b--\r\n";
        let archive = parse_mhtml(data, None).unwrap();

        assert_eq!(archive.root().data, b"<p>\r\n</p>");
        assert_eq!(
            archive
                .get(&Url::parse("https://example.com/a.txt").unwrap())
                .unwrap()
                .data,
            "soft break, \u{2713}".as_bytes()
        );
    }

    #[test]
    fn internet_explorer_archive() {
        // Root part named by the "start" parameter, relative Content-Location, preamble, LF line breaks
        let data = b"MIME-Version: 1.0\nContent-Type: multipart/related; boundary=\"=_NextPart_01\"; type=\"text/html\"; start=\"<root>\"\n\nThis is a multi-part message in MIME format.\n\n--=_NextPart_01\nContent-Type: image/gif\nContent-Transfer-Encoding: base64\nContent-Location: files/a.gif\n\nR0lG\nODlh\n\n--=_NextPart_01\nContent-Type: text/html; charset=\"windows-1252\"\nContent-Transfer-Encoding: 7bit\nContent-ID: <root>\n\n<img src=files/a.gif>\n--=_NextPart_01--\n";
        let archive_url = Url::parse("file:///tmp/page.mht").unwrap();
        let archive = parse_mhtml(data, Some(&archive_url)).unwrap();

        assert_eq!(archive.len(), 2);
        assert_eq!(archive.root().data, b"<img src=files/a.gif>");
        assert_eq!(archive.root().charset, "windows-1252");
        assert!(archive.root().url.is_none());

        let gif = archive
            .get(&Url::parse("file:///tmp/files/a.gif").unwrap())
            .unwrap();
        assert_eq!(gif.data, b"GIF89a");
    }

    #[test]
    fn first_html_part_is_root() {
        let data = b"Content-Type: multipart/related; boundary=b\n\n--b\nContent-Type: image/gif\nContent-Transfer-Encoding: base64\n\nR0lGODlh\n--b\nContent-Type: text/html\n\n<p>root</p>\n--b--\n";
        let archive = parse_mhtml(data, None).unwrap();

        assert_eq!(archive.root().data, b"<p>root</p>");
    }

    #[test]
    fn truncated_archive() {
        let data = b"Content-Type: multipart/related; boundary=b\n\n--b\nContent-Type: text/html\n\n<p>root</p>\n--b\nContent-Type: text/css\nContent-Location: https://example.com/a.css\n\np{}";
        let archive = parse_mhtml(data, None).unwrap();

        assert_eq!(archive.len(), 2);
        assert_eq!(
            archive
                .get(&Url::parse("https://example.com/a.css").unwrap())
                .unwrap()
                .data,
            b"p{}"
        );
    }

    #[test]
    fn first_duplicate_wins() {
        let data = b"Content-Type: multipart/related; boundary=b\n\n--b\nContent-Type: text/html\n\n<p></p>\n--b\nContent-Location: https://example.com/a.txt\n\none\n--b\nContent-Location: https://example.com/a.txt\n\ntwo\n--b--\n";
        let archive = parse_mhtml(data, None).unwrap();

        assert_eq!(
            archive
                .get(&Url::parse("https://example.com/a.txt").unwrap())
                .unwrap()
                .data,
            b"one"
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
    use monolith::mhtml::parse_mhtml;

    #[test]
    fn html() {
        assert!(parse_mhtml(b"<!DOCTYPE html><html></html>", None).is_none());
    }

    #[test]
    fn no_parts() {
        assert!(
            parse_mhtml(
                b"Content-Type: multipart/related; boundary=b\r\n\r\nnothing here",
                None
            )
            .is_none()
        );
    }
}
