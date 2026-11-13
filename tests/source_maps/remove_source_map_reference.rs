//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::source_maps::{
        remove_source_map_reference, remove_source_map_reference_from_text,
    };

    #[test]
    fn last_line() {
        assert_eq!(
            remove_source_map_reference(b"a();\n//# sourceMappingURL=a.js.map\n"),
            b"a();\n"
        );
        assert_eq!(
            remove_source_map_reference(b"a{}\r\n/*# sourceMappingURL=a.css.map */\r\n\r\n"),
            b"a{}\r\n"
        );
        assert_eq!(
            remove_source_map_reference(b"//# sourceMappingURL=a.js.map"),
            b""
        );
    }

    #[test]
    fn text() {
        assert_eq!(
            remove_source_map_reference_from_text("\u{e9}();\n//@ sourceMappingURL=\u{e9}.js.map"),
            "\u{e9}();\n"
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
    use monolith::source_maps::remove_source_map_reference;

    #[test]
    fn anywhere_but_the_last_line() {
        let script = b"//# sourceMappingURL=a.js.map\na();\n";
        assert_eq!(remove_source_map_reference(script), script);
        let script = b"a = '\\\n//# sourceMappingURL=a.js.map'; b();";
        assert_eq!(remove_source_map_reference(script), script);
    }

    #[test]
    fn utf16() {
        let script = b"\xFF\xFE/\x00/\x00#\x00";
        assert_eq!(remove_source_map_reference(script), script);
    }
}
