use crate::utils::{clean_url, data_to_dataurl, is_data_url};
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use std::collections::HashMap;

pub fn retrieve_asset(
    cache: &mut HashMap<String, String>,
    client: &Client,
    url: &str,
    as_dataurl: bool,
    mime: &str,
    opt_silent: bool,
) -> Result<(String, String), reqwest::Error> {
    let cache_key = clean_url(&url);

    if is_data_url(&url).unwrap() {
        Ok((url.to_string(), url.to_string()))
    } else {
        if cache.contains_key(&cache_key) {
            // url is in cache
            if !opt_silent {
                eprintln!("{} (from cache)", &url);
            }
            let data = cache.get(&cache_key).unwrap();
            Ok((data.to_string(), url.to_string()))
        } else {
            // url not in cache, we request it
            let mut response = client.get(url).send()?;

            if !opt_silent {
                if url == response.url().as_str() {
                    eprintln!("{}", &url);
                } else {
                    eprintln!("{} -> {}", &url, &response.url().as_str());
                }
            }

            let new_cache_key = clean_url(response.url().to_string());

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
                cache.insert(new_cache_key, dataurl.to_string());
                Ok((dataurl, response.url().to_string()))
            } else {
                let content = response.text().unwrap();
                // insert in cache
                cache.insert(new_cache_key, content.clone());
                Ok((content, response.url().to_string()))
            }
        }
    }
}
