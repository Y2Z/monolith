//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use chrono::prelude::*;
    use reqwest::Url;

    use monolith::html;

    #[test]
    fn http_url() {
        let url: Url = Url::parse("http://192.168.1.1/").unwrap();
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let metadata_comment: String = html::create_metadata_tag(&url);

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
        let url: Url = Url::parse("file:///home/monolith/index.html").unwrap();
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let metadata_comment: String = html::create_metadata_tag(&url);

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
        let url: Url = Url::parse("data:text/html,Hello%2C%20World!").unwrap();
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let metadata_comment: String = html::create_metadata_tag(&url);

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
