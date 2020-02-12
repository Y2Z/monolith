use crate::utils::{clean_url, data_to_data_url, is_data_url};
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use std::collections::HashMap;

pub fn retrieve_asset(
    cache: &mut HashMap<String, String>,
    client: &Client,
    url: &str,
    as_data_url: bool,
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
            let res_url = response.url().to_string();

            if !opt_silent {
                if url == res_url {
                    eprintln!("{}", &url);
                } else {
                    eprintln!("{} -> {}", &url, &res_url);
                }
            }

            let new_cache_key = clean_url(&res_url);

            if as_data_url {
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
                let data_url = data_to_data_url(&mimetype, &data);
                // insert in cache
                cache.insert(new_cache_key, data_url.clone());
                Ok((data_url, res_url))
            } else {
                let content = response.text().unwrap();
                // insert in cache
                cache.insert(new_cache_key, content.clone());
                Ok((content, res_url))
            }
        }
    }
}
