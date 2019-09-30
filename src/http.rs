use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use std::time::Duration;
use utils::{data_to_dataurl, is_data_url};

pub fn retrieve_asset(
    url: &str,
    as_dataurl: bool,
    as_mime: &str,
    opt_user_agent: &str,
    opt_silent: bool,
    opt_insecure: bool,
) -> Result<String, reqwest::Error> {
    if is_data_url(&url).unwrap() {
        Ok(url.to_string())
    } else {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(opt_insecure)
            .build()?;
        let mut response = client.get(url).header(USER_AGENT, opt_user_agent).send()?;
        let final_url = response.url().as_str();

        if !opt_silent {
            if url == final_url {
                eprintln!("[ {} ]", &url);
            } else {
                eprintln!("[ {} -> {} ]", &url, &final_url);
            }
        }

        if as_dataurl {
            // Convert response into a byte array
            let mut data: Vec<u8> = vec![];
            response.copy_to(&mut data)?;

            // Attempt to obtain MIME type by reading the Content-Type header
            let mimetype = if as_mime == "" {
                response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|header| header.to_str().ok())
                    .unwrap_or(&as_mime)
            } else {
                as_mime
            };

            Ok(data_to_dataurl(&mimetype, &data))
        } else {
            Ok(response.text().unwrap())
        }
    }
}
