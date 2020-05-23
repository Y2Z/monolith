//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::utils;

    #[test]
    fn image_gif87() {
        assert_eq!(utils::detect_media_type(b"GIF87a", ""), "image/gif");
    }

    #[test]
    fn image_gif89() {
        assert_eq!(utils::detect_media_type(b"GIF89a", ""), "image/gif");
    }

    #[test]
    fn image_jpeg() {
        assert_eq!(utils::detect_media_type(b"\xFF\xD8\xFF", ""), "image/jpeg");
    }

    #[test]
    fn image_png() {
        assert_eq!(
            utils::detect_media_type(b"\x89PNG\x0D\x0A\x1A\x0A", ""),
            "image/png"
        );
    }

    #[test]
    fn image_svg() {
        assert_eq!(utils::detect_media_type(b"<svg ", ""), "image/svg+xml");
    }

    #[test]
    fn image_webp() {
        assert_eq!(
            utils::detect_media_type(b"RIFF....WEBPVP8 ", ""),
            "image/webp"
        );
    }

    #[test]
    fn image_icon() {
        assert_eq!(
            utils::detect_media_type(b"\x00\x00\x01\x00", ""),
            "image/x-icon"
        );
    }

    #[test]
    fn image_svg_filename() {
        assert_eq!(
            utils::detect_media_type(b"<?xml ", "local-file.svg"),
            "image/svg+xml"
        );
    }

    #[test]
    fn image_svg_url_uppercase() {
        assert_eq!(
            utils::detect_media_type(b"", "https://some-site.com/images/local-file.SVG"),
            "image/svg+xml"
        );
    }

    #[test]
    fn audio_mpeg() {
        assert_eq!(utils::detect_media_type(b"ID3", ""), "audio/mpeg");
    }

    #[test]
    fn audio_mpeg_2() {
        assert_eq!(utils::detect_media_type(b"\xFF\x0E", ""), "audio/mpeg");
    }

    #[test]
    fn audio_mpeg_3() {
        assert_eq!(utils::detect_media_type(b"\xFF\x0F", ""), "audio/mpeg");
    }

    #[test]
    fn audio_ogg() {
        assert_eq!(utils::detect_media_type(b"OggS", ""), "audio/ogg");
    }

    #[test]
    fn audio_wav() {
        assert_eq!(
            utils::detect_media_type(b"RIFF....WAVEfmt ", ""),
            "audio/wav"
        );
    }

    #[test]
    fn audio_flac() {
        assert_eq!(utils::detect_media_type(b"fLaC", ""), "audio/x-flac");
    }

    #[test]
    fn video_avi() {
        assert_eq!(
            utils::detect_media_type(b"RIFF....AVI LIST", ""),
            "video/avi"
        );
    }

    #[test]
    fn video_mp4() {
        assert_eq!(utils::detect_media_type(b"....ftyp", ""), "video/mp4");
    }

    #[test]
    fn video_mpeg() {
        assert_eq!(
            utils::detect_media_type(b"\x00\x00\x01\x0B", ""),
            "video/mpeg"
        );
    }

    #[test]
    fn video_quicktime() {
        assert_eq!(utils::detect_media_type(b"....moov", ""), "video/quicktime");
    }

    #[test]
    fn video_webm() {
        assert_eq!(
            utils::detect_media_type(b"\x1A\x45\xDF\xA3", ""),
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
    use crate::utils;

    #[test]
    fn unknown_media_type() {
        assert_eq!(utils::detect_media_type(b"abcdef0123456789", ""), "");
    }
}
