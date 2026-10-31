//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::mhtml::is_mhtml;

    #[test]
    fn chrome_archive() {
        assert!(is_mhtml(
            b"From: <Saved by Blink>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/related;\r\n\ttype=\"text/html\";\r\n\tboundary=\"----MultipartBoundary--abc----\"\r\n\r\n"
        ));
    }

    #[test]
    fn internet_explorer_archive() {
        assert!(is_mhtml(
            b"MIME-Version: 1.0\nContent-Type: multipart/related; boundary=\"=_NextPart_01\"; type=\"text/html\"\n\n"
        ));
    }

    #[test]
    fn archive_with_bom() {
        assert!(is_mhtml(
            b"\xEF\xBB\xBFContent-Type: Multipart/Related; boundary=x\r\n\r\n--x\r\n\r\n<p></p>\r\n--x--\r\n"
        ));
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
    use monolith::mhtml::is_mhtml;

    #[test]
    fn html() {
        assert!(!is_mhtml(b"<!DOCTYPE html><html></html>"));
    }

    #[test]
    fn empty() {
        assert!(!is_mhtml(b""));
    }

    #[test]
    fn headers_without_content_type() {
        assert!(!is_mhtml(b"Hello: world\r\n\r\n<html></html>"));
    }

    #[test]
    fn not_multipart_related() {
        assert!(!is_mhtml(
            b"Content-Type: multipart/alternative; boundary=x\r\n\r\n--x\r\n\r\nhi\r\n--x--\r\n"
        ));
    }

    #[test]
    fn no_boundary() {
        assert!(!is_mhtml(b"Content-Type: multipart/related\r\n\r\n"));
    }
}
