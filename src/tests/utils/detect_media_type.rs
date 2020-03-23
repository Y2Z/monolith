use crate::utils;

//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn passing_image_media_types() {
    assert_eq!(utils::detect_media_type(b"GIF87a"), "image/gif");
    assert_eq!(utils::detect_media_type(b"GIF89a"), "image/gif");
    assert_eq!(utils::detect_media_type(b"\xFF\xD8\xFF"), "image/jpeg");
    assert_eq!(
        utils::detect_media_type(b"\x89PNG\x0D\x0A\x1A\x0A"),
        "image/png"
    );
    assert_eq!(utils::detect_media_type(b"<?xml "), "image/svg+xml");
    assert_eq!(utils::detect_media_type(b"<svg "), "image/svg+xml");
    assert_eq!(utils::detect_media_type(b"RIFF....WEBPVP8 "), "image/webp");
    assert_eq!(
        utils::detect_media_type(b"\x00\x00\x01\x00"),
        "image/x-icon"
    );
}

#[test]
fn passing_audio_media_types() {
    assert_eq!(utils::detect_media_type(b"ID3"), "audio/mpeg");
    assert_eq!(utils::detect_media_type(b"\xFF\x0E"), "audio/mpeg");
    assert_eq!(utils::detect_media_type(b"\xFF\x0F"), "audio/mpeg");
    assert_eq!(utils::detect_media_type(b"OggS"), "audio/ogg");
    assert_eq!(utils::detect_media_type(b"RIFF....WAVEfmt "), "audio/wav");
    assert_eq!(utils::detect_media_type(b"fLaC"), "audio/x-flac");
}

#[test]
fn passing_video_media_types() {
    assert_eq!(utils::detect_media_type(b"RIFF....AVI LIST"), "video/avi");
    assert_eq!(utils::detect_media_type(b"....ftyp"), "video/mp4");
    assert_eq!(utils::detect_media_type(b"\x00\x00\x01\x0B"), "video/mpeg");
    assert_eq!(utils::detect_media_type(b"....moov"), "video/quicktime");
    assert_eq!(utils::detect_media_type(b"\x1A\x45\xDF\xA3"), "video/webm");
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[test]
fn failing_unknown_media_type() {
    assert_eq!(utils::detect_media_type(b"abcdef0123456789"), "");
}
