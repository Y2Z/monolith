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

    #[test]
    fn image_gif87() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"GIF87a", &dummy_url), "image/gif");
    }

    #[test]
    fn image_gif89() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"GIF89a", &dummy_url), "image/gif");
    }

    #[test]
    fn image_jpeg() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"\xFF\xD8\xFF", &dummy_url), "image/jpeg");
    }

    #[test]
    fn image_png() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"\x89PNG\x0D\x0A\x1A\x0A", &dummy_url),
            "image/png"
        );
    }

    #[test]
    fn image_svg() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"<svg ", &dummy_url), "image/svg+xml");
    }

    #[test]
    fn image_webp() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"RIFF....WEBPVP8 ", &dummy_url),
            "image/webp"
        );
    }

    #[test]
    fn image_icon() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"\x00\x00\x01\x00", &dummy_url),
            "image/x-icon"
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
    fn audio_mpeg() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"ID3", &dummy_url), "audio/mpeg");
    }

    #[test]
    fn audio_mpeg_2() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"\xFF\x0E", &dummy_url), "audio/mpeg");
    }

    #[test]
    fn audio_mpeg_3() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"\xFF\x0F", &dummy_url), "audio/mpeg");
    }

    #[test]
    fn audio_ogg() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"OggS", &dummy_url), "audio/ogg");
    }

    #[test]
    fn audio_wav() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"RIFF....WAVEfmt ", &dummy_url),
            "audio/wav"
        );
    }

    #[test]
    fn audio_flac() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"fLaC", &dummy_url), "audio/x-flac");
    }

    #[test]
    fn video_avi() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"RIFF....AVI LIST", &dummy_url),
            "video/avi"
        );
    }

    #[test]
    fn video_mp4() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(detect_media_type(b"....ftyp", &dummy_url), "video/mp4");
    }

    #[test]
    fn video_mpeg() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"\x00\x00\x01\x0B", &dummy_url),
            "video/mpeg"
        );
    }

    #[test]
    fn video_quicktime() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"....moov", &dummy_url),
            "video/quicktime"
        );
    }

    #[test]
    fn video_webm() {
        let dummy_url: Url = Url::parse("data:,").unwrap();
        assert_eq!(
            detect_media_type(b"\x1A\x45\xDF\xA3", &dummy_url),
            "video/webm"
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
}
