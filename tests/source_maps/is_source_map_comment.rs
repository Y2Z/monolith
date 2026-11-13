//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::source_maps::is_source_map_comment;

    #[test]
    fn all_forms_of_it() {
        assert!(is_source_map_comment("//# sourceMappingURL=a.js.map"));
        assert!(is_source_map_comment("//@ sourceMappingURL=a.js.map"));
        assert!(is_source_map_comment("/*# sourceMappingURL=a.css.map */"));
        assert!(is_source_map_comment("/*@ sourceMappingURL=a.css.map*/"));
        assert!(is_source_map_comment(
            "//# sourceMappingURL=data:application/json;base64,e30="
        ));
        assert!(is_source_map_comment("  //# sourceMappingURL=a.js.map  "));
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
    use monolith::source_maps::is_source_map_comment;

    #[test]
    fn other_comments() {
        assert!(!is_source_map_comment("// sourceMappingURL=a.js.map"));
        assert!(!is_source_map_comment("/* sourceMappingURL=a.css.map */"));
        assert!(!is_source_map_comment("//# sourceURL=a.js"));
        assert!(!is_source_map_comment("//# sourceMappingURL="));
        assert!(!is_source_map_comment("/*# sourceMappingURL=a.css.map"));
        assert!(!is_source_map_comment(
            "/*# sourceMappingURL=a.css.map */ b {} /* c */"
        ));
        assert!(!is_source_map_comment(
            "x = 1; //# sourceMappingURL=a.js.map"
        ));
    }
}
