//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use monolith::core::detect_media_type;

    // Signatures are padded, since some formats are only recognized with enough of their header present
    // (e.g. WebM needs more than 256 bytes to be told apart from other Matroska files)
    fn header(start: &[u8]) -> Vec<u8> {
        let mut data = start.to_vec();
        data.resize(512.max(start.len()), 0);
        data
    }

    #[test]
    fn by_signature() {
        let dummy_url: Url = Url::parse("data:,").unwrap();

        for (data, expected) in [
            (header(b"GIF87a"), "image/gif"),
            (header(b"GIF89a"), "image/gif"),
            (header(b"\xFF\xD8\xFF\xE0"), "image/jpeg"),
            (header(b"\x89PNG\x0D\x0A\x1A\x0A"), "image/png"),
            (header(b"<svg "), "image/svg+xml"),
            (header(b"RIFF\x24\x00\x00\x00WEBPVP8 "), "image/webp"),
            (header(b"\x00\x00\x01\x00\x01\x00"), "image/x-icon"),
            (header(b"ID3\x04\x00"), "audio/mpeg"),
            (header(b"\xFF\xFB\x90\x00"), "audio/mpeg"),
            (header(b"OggS\x00\x02"), "audio/ogg"),
            (header(b"RIFF\x24\x00\x00\x00WAVEfmt "), "audio/wav"),
            (header(b"fLaC\x00\x00\x00\x22"), "audio/flac"),
            (header(b"RIFF\x24\x00\x00\x00AVI LIST"), "video/avi"),
            (header(b"\x00\x00\x00\x18ftypisom"), "video/mp4"),
            (header(b"\x00\x00\x01\xBA"), "video/mpeg"),
            (header(b"\x00\x00\x00\x14ftypqt  "), "video/quicktime"),
            (
                header(b"\x1A\x45\xDF\xA3\x9F\x42\x86\x81\x01\x42\x82\x84webm"),
                "video/webm",
            ),
            (header(b"wOF2\x00\x01\x00\x00"), "font/woff2"),
        ] {
            assert_eq!(detect_media_type(&data, &dummy_url), expected);
        }
    }

    #[test]
    fn signature_beats_file_name() {
        let file_url: Url = Url::parse("file:///tmp/actually-a-png.jpg").unwrap();
        assert_eq!(
            detect_media_type(b"\x89PNG\x0D\x0A\x1A\x0A", &file_url),
            "image/png"
        );
    }

    #[test]
    fn image_svg_filename() {
        let file_url: Url = Url::parse("file:///tmp/local-file.svg").unwrap();
        assert_eq!(detect_media_type(b"<?xml ", &file_url), "image/svg+xml");
    }

    #[test]
    fn image_svg_url_uppercase() {
        let https_url: Url = Url::parse("https://some-site.com/images/local-file.SVG").unwrap();
        assert_eq!(detect_media_type(b"", &https_url), "image/svg+xml");
    }

    #[test]
    fn image_svg_after_xml_declaration() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(
                b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"/>",
                &dummy_url
            ),
            "image/svg+xml"
        );
    }

    #[test]
    fn text_formats_by_file_name() {
        for (url, expected) in [
            ("https://some-site.com/style.css", "text/css"),
            ("https://some-site.com/app.js", "text/javascript"),
            ("https://some-site.com/data.json", "application/json"),
            ("file:///tmp/page.html", "text/html"),
            ("file:///tmp/My%20Notes.txt", "text/plain"),
        ] {
            assert_eq!(detect_media_type(b"", &Url::parse(url).unwrap()), expected);
        }
    }

    #[test]
    fn html_by_content() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"<!DOCTYPE html><html></html>", &dummy_url),
            "text/html"
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
    use reqwest::Url;

    use monolith::core::detect_media_type;

    #[test]
    fn unknown_media_type() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"abcdef0123456789", &dummy_url), "");
    }

    #[test]
    fn unknown_file_extension() {
        let file_url: Url = Url::parse("file:///tmp/archive.unknown").unwrap();
        assert_eq!(detect_media_type(b"abcdef0123456789", &file_url), "");
    }

    #[test]
    fn directory_url() {
        let https_url: Url = Url::parse("https://some-site.com/images/").unwrap();
        assert_eq!(detect_media_type(b"", &https_url), "");
    }
}
