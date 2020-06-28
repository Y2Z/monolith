//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::blocking::Client;
    use std::collections::HashMap;
    use std::env;

    use crate::url;
    use crate::utils;

    #[test]
    fn read_data_url() {
        let cache = &mut HashMap::new();
        let client = Client::new();

        // If both source and target are data URLs,
        // ensure the result contains target data URL
        let (data, final_url, media_type) = utils::retrieve_asset(
            cache,
            &client,
            "data:text/html;base64,c291cmNl",
            "data:text/html;base64,dGFyZ2V0",
            false,
        )
        .unwrap();
        assert_eq!(
            url::data_to_data_url(&media_type, &data, &final_url),
            url::data_to_data_url("text/html", "target".as_bytes(), "")
        );
        assert_eq!(
            final_url,
            url::data_to_data_url("text/html", "target".as_bytes(), "")
        );
        assert_eq!(&media_type, "text/html");
    }

    #[test]
    fn read_local_file_with_file_url_parent() {
        let cache = &mut HashMap::new();
        let client = Client::new();

        let file_url_protocol: &str = if cfg!(windows) { "file:///" } else { "file://" };

        // Inclusion of local assets from local sources should be allowed
        let cwd = env::current_dir().unwrap();
        let (data, final_url, _media_type) = utils::retrieve_asset(
            cache,
            &client,
            &format!(
                "{file}{cwd}/src/tests/data/basic/local-file.html",
                file = file_url_protocol,
                cwd = cwd.to_str().unwrap()
            ),
            &format!(
                "{file}{cwd}/src/tests/data/basic/local-script.js",
                file = file_url_protocol,
                cwd = cwd.to_str().unwrap()
            ),
            false,
        )
        .unwrap();
        assert_eq!(url::data_to_data_url("application/javascript", &data, &final_url), "data:application/javascript;base64,ZG9jdW1lbnQuYm9keS5zdHlsZS5iYWNrZ3JvdW5kQ29sb3IgPSAiZ3JlZW4iOwpkb2N1bWVudC5ib2R5LnN0eWxlLmNvbG9yID0gInJlZCI7Cg==");
        assert_eq!(
            &final_url,
            &format!(
                "{file}{cwd}/src/tests/data/basic/local-script.js",
                file = file_url_protocol,
                cwd = cwd.to_str().unwrap()
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
    use reqwest::blocking::Client;
    use std::collections::HashMap;

    use crate::utils;

    #[test]
    fn read_local_file_with_data_url_parent() {
        let cache = &mut HashMap::new();
        let client = Client::new();

        // Inclusion of local assets from data URL sources should not be allowed
        match utils::retrieve_asset(
            cache,
            &client,
            "data:text/html;base64,SoUrCe",
            "file:///etc/passwd",
            false,
        ) {
            Ok((..)) => {
                assert!(false);
            }
            Err(_) => {
                assert!(true);
            }
        }
    }

    #[test]
    fn read_local_file_with_https_parent() {
        let cache = &mut HashMap::new();
        let client = Client::new();

        // Inclusion of local assets from remote sources should not be allowed
        match utils::retrieve_asset(
            cache,
            &client,
            "https://kernel.org/",
            "file:///etc/passwd",
            false,
        ) {
            Ok((..)) => {
                assert!(false);
            }
            Err(_) => {
                assert!(true);
            }
        }
    }
}
