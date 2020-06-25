//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use chrono::prelude::*;

    use crate::html;

    #[test]
    fn http_url() {
        let url = "http://192.168.1.1/";
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let metadata_comment: String = html::metadata_tag(url);

        assert_eq!(
            metadata_comment,
            format!(
                "<!-- Saved from {} at {} using {} v{} -->",
                &url,
                timestamp,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            )
        );
    }

    #[test]
    fn file_url() {
        let url = "file:///home/monolith/index.html";
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let metadata_comment: String = html::metadata_tag(url);

        assert_eq!(
            metadata_comment,
            format!(
                "<!-- Saved from local source at {} using {} v{} -->",
                timestamp,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            )
        );
    }

    #[test]
    fn data_url() {
        let url = "data:text/html,Hello%2C%20World!";
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let metadata_comment: String = html::metadata_tag(url);

        assert_eq!(
            metadata_comment,
            format!(
                "<!-- Saved from local source at {} using {} v{} -->",
                timestamp,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            )
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
    use crate::html;

    #[test]
    fn empty_string() {
        assert_eq!(html::metadata_tag(""), "");
    }
}
