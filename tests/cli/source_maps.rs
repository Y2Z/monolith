//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use assert_cmd::prelude::*;
    use std::path::Path;

    #[test]
    fn references_to_source_maps_go() {
        let mut cmd = crate::cli::monolith();
        let path_html: &Path = Path::new("tests/_data_/source-maps/index.html");

        let out = cmd.arg("-q").arg(path_html.as_os_str()).output().unwrap();
        let html = crate::cli::without_metadata(&out.stdout);

        assert_eq!(html.matches("sourceMappingURL").count(), 1, "{html}");
        // What only looks like one stays
        assert!(html.contains("var b = \"//# sourceMappingURL=kept.js.map\";"));
        assert!(html.contains("console.log(1);"));
        assert!(html.contains("body{color:green}"));
        assert!(html.contains("p{color:red}"));

        out.assert().code(0);
    }

    #[test]
    fn references_to_source_maps_go_from_data_urls_too() {
        let mut cmd = crate::cli::monolith();
        let path_html: &Path = Path::new("tests/_data_/source-maps/async.html");

        let out = cmd.arg("-q").arg(path_html.as_os_str()).output().unwrap();
        let html = crate::cli::without_metadata(&out.stdout);

        // Scripts that run whenever they've loaded stay separate
        assert!(
            html.contains("<script async=\"\" src=\"data:text/javascript,console.log(1);%0A\">"),
            "{html}"
        );

        out.assert().code(0);
    }

    #[test]
    fn references_to_source_maps_go_from_mhtml_too() {
        let mut cmd = crate::cli::monolith();
        let path_html: &Path = Path::new("tests/_data_/source-maps/index.html");

        let out = cmd
            .args(["-q", "-F", "mhtml"])
            .arg(path_html.as_os_str())
            .output()
            .unwrap();
        let mhtml = String::from_utf8_lossy(&out.stdout);

        // Quoted-printable has "=" as "=3D"
        assert_eq!(mhtml.matches("sourceMappingURL").count(), 1, "{mhtml}");
        assert!(mhtml.contains("kept.js.map"), "{mhtml}");

        out.assert().code(0);
    }
}
