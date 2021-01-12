use crate::skip_fail;
use crate::storage::{Response, Storage};

use anyhow::{format_err, Result};
use async_std::path::{Path, PathBuf};
use async_trait::async_trait;
use chrono::prelude::*;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::codec::{BytesCodec, FramedRead};

// In bytes
const MAX_STREAM_FILE_SIZE: u64 = 5 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
pub struct PasteMeta {
    key: String, // Should be a uuid
    create_time: DateTime<Utc>,
    expire_time: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct SimpleStorage {
    base_dir: PathBuf,
    db: sled::Db,
}

impl SimpleStorage {
    pub fn new(base: &Path) -> Result<SimpleStorage> {
        let mut db_path = PathBuf::from(base);
        db_path.push("pastebin.db");
        let db = sled::open(&db_path)?;

        Ok(SimpleStorage {
            base_dir: PathBuf::from(base),
            db,
        })
    }
}

#[async_trait]
impl Storage for SimpleStorage {
    fn exists(&self, id: &str) -> Result<bool> {
        let key_key = String::from("key") + "." + id;
        Ok(self.db.contains_key(&key_key)?)
    }

    fn validate(&self, id: &str, key: &str) -> Result<bool> {
        let key_key = String::from("key") + "." + id;
        let local_key = self.db.get(&key_key)?;

        if local_key == None {
            return Err(format_err!("Paste does not exists"));
        }

        // Check delete key
        if local_key.unwrap() == key {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn get(&self, id: &str) -> Result<Response> {
        let mut paste_path = PathBuf::from(&self.base_dir);
        paste_path.push(id);

        if !paste_path.is_file().await {
            return Err(format_err!("Internal Error"));
        }

        let mut file = File::open(&paste_path).await?;

        // Check file size
        let meta = fs::metadata(&paste_path).await?;
        if meta.len() < MAX_STREAM_FILE_SIZE {
            let mut content: Vec<u8> = Vec::new();
            file.read_to_end(&mut content).await?;

            Ok(Response::Content(content))
        } else {
            let stream = FramedRead::new(file, BytesCodec::new());
            Ok(Response::Stream(stream))
        }
    }

    async fn new(&self, id: &str, key: &str) -> Result<File> {
        if self.exists(id)? {
            return Err(format_err!("A paste with this id already exists"));
        }

        let mut new_path = PathBuf::from(&self.base_dir);
        new_path.push(id);
        let content_file = fs::File::create(&new_path).await?;

        // Add meta
        let key_key = String::from("key") + "." + id;
        self.db.insert(&key_key, key)?;
        Ok(content_file)
    }

    fn set_expire_time(&self, id: &str, time: &DateTime<Utc>) -> Result<()> {
        let expire_time_key = String::from("expire_time") + "." + id;
        let time_string: String = time.to_rfc3339();
        self.db.insert(&expire_time_key, time_string.as_str())?;
        Ok(())
    }

    async fn update(&self, id: &str) -> Result<File> {
        let path = self.base_dir.join(id);
        fs::remove_file(&path).await?;
        let highlight_path = self.base_dir.join(id.to_owned() + ".highlight");
        if highlight_path.is_file().await {
            fs::remove_file(&highlight_path).await?;
        }
        let f = File::create(&path).await?;
        Ok(f)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut paste_path = PathBuf::from(&self.base_dir);
        paste_path.push(id);

        // Actually delete them
        fs::remove_file(paste_path).await?;
        let key_key = String::from("key") + "." + id;
        let expire_time_key = String::from("expire_time") + "." + id;
        self.db.remove(&key_key)?;
        self.db.remove(&expire_time_key)?;

        Ok(())
    }

    async fn cleanup(&self) -> Result<Vec<String>> {
        debug!("Begin deleting expired pastes...");
        let mut deleted = Vec::new();

        // Go though all expire times
        for paste in self.db.scan_prefix("expire_time") {
            let p = skip_fail!(paste);
            let expire_time_str = skip_fail!(String::from_utf8(p.1.to_vec()));
            let expire_time = skip_fail!(chrono::DateTime::parse_from_rfc3339(&expire_time_str));
            if Utc::now() >= expire_time {
                let mut id = skip_fail!(String::from_utf8(p.0.to_vec()));
                id = id.trim_start_matches("expire_time.").to_string();
                // Try to delete the paste file
                let mut paste_path = PathBuf::from(&self.base_dir);
                paste_path.push(&id);
                skip_fail!(fs::remove_file(&paste_path).await);
                // Now remove the database entries
                let key_key = String::from("key") + "." + &id;
                let expire_time_key = String::from("expire_time") + "." + &id;
                skip_fail!(self.db.remove(&key_key));
                skip_fail!(self.db.remove(&expire_time_key));
                // Finally add to list
                deleted.push(id);
            }
        }
        debug!("Finish deleting expired pastes.");
        Ok(deleted)
    }
}
