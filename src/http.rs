use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use utils::{data_to_dataurl, is_data_url};

pub fn retrieve_asset(
    cache: &mut HashMap<String, String>,
    url: &str,
    as_dataurl: bool,
    mime: &str,
    opt_user_agent: &str,
    opt_silent: bool,
    opt_insecure: bool,
) -> Result<(String, String), reqwest::Error> {
    if is_data_url(&url).unwrap() {
        Ok((url.to_string(), url.to_string()))
    } else {
        if cache.contains_key(&url.to_string()) {
            // url is in cache
            if !opt_silent {
                eprintln!("[ {} ] (from cache)", &url);
            }
            let data = cache.get(&url.to_string()).unwrap();
            Ok((data.to_string(), url.to_string()))
        } else {
            // url not in cache, we request it
            let client = Client::builder()
                .timeout(Duration::from_secs(10))
                .danger_accept_invalid_certs(opt_insecure)
                .build()?;
            let mut response = client.get(url).header(USER_AGENT, opt_user_agent).send()?;

            if !opt_silent {
                if url == response.url().as_str() {
                    eprintln!("[ {} ]", &url);
                } else {
                    eprintln!("[ {} -> {} ]", &url, &response.url().as_str());
                }
            }

            if as_dataurl {
                // Convert response into a byte array
                let mut data: Vec<u8> = vec![];
                response.copy_to(&mut data)?;

                // Attempt to obtain MIME type by reading the Content-Type header
                let mimetype = if mime == "" {
                    response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|header| header.to_str().ok())
                        .unwrap_or(&mime)
                } else {
                    mime
                };
                let dataurl = data_to_dataurl(&mimetype, &data);
                // insert in cache
                cache.insert(response.url().to_string(), dataurl.to_string());
                Ok((dataurl, response.url().to_string()))
            } else {
                let content = response.text().unwrap();
                // insert in cache
                cache.insert(response.url().to_string(), content.clone());
                Ok((content, response.url().to_string()))
            }
        }
    }
}
