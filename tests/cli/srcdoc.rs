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
    fn srcdoc_documents_get_embedded_like_any_other() {
        let mut cmd = crate::cli::monolith();
        let path_html: &Path = Path::new("tests/_data_/srcdoc/index.html");

        let out = cmd.arg("-q").arg(path_html.as_os_str()).output().unwrap();
        let html = crate::cli::without_metadata(&out.stdout);

        // Relative to what they're in, BASE elements of the page included
        assert!(
            html.contains("href=&quot;data:text/css;charset=utf-8,p{color:blue}%0A&quot;"),
            "{html}"
        );
        // Unless they have BASE elements of their own
        assert!(
            html.contains("href=&quot;data:text/css;charset=utf-8,p{color:green}%0A&quot;"),
            "{html}"
        );

        out.assert().code(0);
    }
}
