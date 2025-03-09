use std::fs;
use std::process;

use tempfile::Builder;

use monolith::cache::Cache;
use monolith::cookies::parse_cookie_file_contents;
use monolith::core::create_monolithic_file;
use monolith::opts::Options;

const CACHE_ASSET_FILE_SIZE_THRESHOLD: usize = 1024 * 50; // Minimum asset file size (in bytes)

fn main() {
    let mut options = Options::from_args();

    // Set up cache (attempt to create temporary file)
    let temp_cache_file = match Builder::new().prefix(".monolith-").keep(false).tempfile() {
        Ok(tempfile) => Some(tempfile),
        Err(_) => None,
    };
    let mut cache = Cache::new(
        CACHE_ASSET_FILE_SIZE_THRESHOLD,
        if temp_cache_file.is_some() {
            Some(temp_cache_file.unwrap().path().display().to_string())
        } else {
            None
        },
    );

    // Read and parse cookie file
    if let Some(opt_cookie_file) = options.cookie_file.clone() {
        match fs::read_to_string(opt_cookie_file) {
            Ok(str) => match parse_cookie_file_contents(&str) {
                Ok(cookies) => {
                    options.cookies = cookies;
                }
                Err(_) => {
                    eprintln!("Could not parse specified cookie file");
                    process::exit(1);
                }
            },
            Err(_) => {
                eprintln!("Could not read specified cookie file");
                process::exit(1);
            }
        }
    }

    create_monolithic_file(&mut cache, &options);

    // Remove temporary file used for storing cache's database
    // drop(temp_file);
}
