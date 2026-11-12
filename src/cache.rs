use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use chrono::{DateTime, Utc};
use redb::{Database, Error, ReadableDatabase, TableDefinition};

/// Everything there is to know about getting a resource, the way it happened: what got asked for,
/// and what came back. Archives get made out of these.
#[derive(Clone, Debug, Default)]
pub struct Exchange {
    /// What came back, with transfer and content encodings already undone
    pub body: Vec<u8>,
    /// For what wasn't received over HTTP (see status); responses have it among their headers
    pub content_type: String,
    pub date: DateTime<Utc>,
    /// "HTTP/1.1", "HTTP/2"...
    pub http_version: String,
    pub method: String,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    /// None for what wasn't received over HTTP, such as local files, and assets of archives
    pub status: Option<u16>,
    pub status_text: String,
    pub url: String,
}

pub struct CacheMetadataItem {
    data: Option<Vec<u8>>, // Asset's blob; used for caching small files or if on-disk database isn't utilized
    media_type: Option<String>, // MIME-type, things like "text/plain", "image/png"...
    charset: Option<String>, // "UTF-8", "UTF-16"...
}

// #[derive(Debug)]
pub struct Cache {
    min_file_size: usize, // Only use database for assets larger than this size (in bytes), otherwise keep them in RAM
    metadata: HashMap<String, CacheMetadataItem>, // Dictionary of metadata (and occasionally data [mostly for very small files])
    db: Option<Database>, // Pointer to database instance; None if not yet initialized or if failed to initialize
    db_ok: Option<bool>, // None by default, Some(true) if was able to initialize database, Some (false) if an error occurred
    db_file_path: Option<String>, // Filesystem path to file used for storing database
    exchanges: Vec<(Exchange, bool)>, // Network exchanges, along with whether their bodies are in the database
}

const FILE_WRITE_BUF_LEN: usize = 1024 * 100; // On-disk cache file write buffer size (in bytes)
const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("_");
const EXCHANGES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("exchanges");

impl Cache {
    pub fn new(min_file_size: usize, db_file_path: Option<String>) -> Cache {
        let mut cache = Cache {
            min_file_size,
            metadata: HashMap::new(),
            db: None,
            db_ok: None,
            db_file_path: db_file_path.clone(),
            exchanges: vec![],
        };

        if let Some(db_file_path) = &db_file_path {
            // Attempt to initialize on-disk database
            match Database::create(Path::new(db_file_path)) {
                Ok(db) => {
                    cache.db = Some(db);
                    cache.db_ok = Some(true);
                    cache
                }
                Err(..) => {
                    cache.db_ok = Some(false);
                    cache
                }
            }
        } else {
            cache.db_ok = Some(false);
            cache
        }
    }

    pub fn set(&mut self, key: &str, data: &Vec<u8>, media_type: String, charset: String) {
        let mut cache_metadata_item: CacheMetadataItem = CacheMetadataItem {
            data: if self.db_ok.is_some() && self.db_ok.unwrap() {
                None
            } else {
                Some(data.to_owned().to_vec())
            },
            media_type: Some(media_type.to_owned()),
            charset: Some(charset),
        };

        if (self.db_ok.is_none() || !self.db_ok.unwrap()) || data.len() <= self.min_file_size {
            cache_metadata_item.data = Some(data.to_owned().to_vec());
        } else {
            match self.db.as_ref().unwrap().begin_write() {
                Ok(mut write_txn) => {
                    {
                        let _ = write_txn.set_durability(redb::Durability::None);
                        let mut table = write_txn.open_table(TABLE).unwrap();
                        table.insert(key, &*data.to_owned()).unwrap();
                    }
                    write_txn.commit().unwrap();
                }
                Err(..) => {
                    // Fall back to caching everything in memory
                    cache_metadata_item.data = Some(data.to_owned().to_vec());
                }
            }
        }

        self.metadata
            .insert((*key).to_string(), cache_metadata_item);
    }

    pub fn get(&self, key: &str) -> Result<(Vec<u8>, String, String), Error> {
        if self.metadata.contains_key(key) {
            let metadata_item = self.metadata.get(key).unwrap();

            if let Some(data) = &metadata_item.data {
                return Ok((
                    data.to_vec(),
                    metadata_item.media_type.as_ref().expect("").to_string(),
                    metadata_item.charset.as_ref().expect("").to_string(),
                ));
            } else if self.db_ok.is_some() && self.db_ok.unwrap() {
                let read_txn = self.db.as_ref().unwrap().begin_read()?;
                let table = read_txn.open_table(TABLE)?;
                let data = table.get(key)?;
                let bytes = data.unwrap();

                return Ok((
                    bytes.value().to_vec(),
                    metadata_item.media_type.as_ref().expect("").to_string(),
                    metadata_item.charset.as_ref().expect("").to_string(),
                ));
            }
        }

        Err(Error::TransactionInProgress) // XXX
    }

    /// Keeps a record of a network exchange, bodies that are large enough go into the database.
    pub fn add_exchange(&mut self, mut exchange: Exchange) {
        let index = self.exchanges.len() as u64;
        let mut is_on_disk = false;

        if self.db_ok == Some(true)
            && exchange.body.len() > self.min_file_size
            && let Ok(mut write_txn) = self.db.as_ref().unwrap().begin_write()
        {
            let _ = write_txn.set_durability(redb::Durability::None);
            let stored = write_txn
                .open_table(EXCHANGES_TABLE)
                .and_then(|mut table| {
                    table.insert(index, exchange.body.as_slice())?;
                    Ok(())
                })
                .is_ok();
            if stored && write_txn.commit().is_ok() {
                exchange.body = vec![];
                is_on_disk = true;
            }
        }

        self.exchanges.push((exchange, is_on_disk));
    }

    pub fn exchange_count(&self) -> usize {
        self.exchanges.len()
    }

    /// Provides a recorded exchange, with its body.
    pub fn exchange(&self, index: usize) -> Option<Exchange> {
        let (exchange, is_on_disk) = self.exchanges.get(index)?;
        let mut exchange = exchange.clone();

        if *is_on_disk {
            let read_txn = self.db.as_ref()?.begin_read().ok()?;
            let table = read_txn.open_table(EXCHANGES_TABLE).ok()?;
            exchange.body = table.get(index as u64).ok()??.value().to_vec();
        }

        Some(exchange)
    }

    /// Forgets recorded exchanges, those that come after start over from the first index.
    pub fn clear_exchanges(&mut self) {
        self.exchanges.clear();
    }

    /// Whether anything has been recorded for given URL.
    pub fn has_exchange(&self, url: &str) -> bool {
        self.exchanges
            .iter()
            .any(|(exchange, _)| exchange.url == url)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }

    pub fn destroy_database_file(&mut self) {
        if self.db_ok.is_none() || !self.db_ok.unwrap() {
            return;
        }

        // Destroy database instance (prevents writes into file)
        self.db = None;
        self.db_ok = Some(false);

        // Wipe database file
        if let Some(db_file_path) = self.db_file_path.to_owned() {
            // Overwrite file with zeroes
            if let Ok(temp_file) = File::options()
                .read(true)
                .write(true)
                .open(db_file_path.clone())
            {
                let mut buffer = [0; FILE_WRITE_BUF_LEN];
                let mut remaining_size: usize = temp_file.metadata().unwrap().len() as usize;
                let mut writer = BufWriter::new(temp_file);

                while remaining_size > 0 {
                    let bytes_to_write: usize = if remaining_size < FILE_WRITE_BUF_LEN {
                        remaining_size
                    } else {
                        FILE_WRITE_BUF_LEN
                    };
                    let buffer = &mut buffer[..bytes_to_write];
                    // All of it has to get overwritten, nothing of what's been saved is to remain
                    if writer.write_all(buffer).is_err() {
                        break;
                    }

                    remaining_size -= bytes_to_write;
                }
                let _ = writer.flush();
            }
        }
    }
}
