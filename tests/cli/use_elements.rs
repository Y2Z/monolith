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
    fn svgs_only_come_from_the_page_s_own_origin() {
        let mut cmd = crate::cli::monolith();
        let path_html: &Path = Path::new("tests/_data_/use/page/index.html");

        let out = cmd.arg(path_html.as_os_str()).output().unwrap();
        let html = crate::cli::without_metadata(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);

        // Files in the page's directory are its own (see cors.rs)
        assert!(html.contains("<use href=\"#x\"><symbol id=\"x\""), "{html}");
        // Others aren't, and data URLs don't get used either
        assert!(
            html.contains("<svg id=\"other\"><use></use></svg>"),
            "{html}"
        );
        assert!(
            html.contains("<svg id=\"data\"><use></use></svg>"),
            "{html}"
        );
        assert!(
            stderr.contains("elsewhere/sprite.svg (browsers don't use SVGs of other origins)"),
            "{stderr}"
        );
        assert!(
            stderr.contains("(browsers don't use SVGs of data URLs)"),
            "{stderr}"
        );

        out.assert().code(0);
    }
}
