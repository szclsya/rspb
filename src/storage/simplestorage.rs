use crate::skip_fail;
use crate::storage::{PasteMeta, Response, Storage};

use anyhow::{format_err, Result};
use async_std::path::{Path, PathBuf};
use async_trait::async_trait;
use chrono::prelude::*;
use log::{debug, error};
use std::collections::HashMap;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio_util::codec::{BytesCodec, FramedRead};

// In bytes
const MAX_STREAM_FILE_SIZE: u64 = 5 * 1024 * 1024;
const MAX_SIZE: u64 = 10 * 1024 * 1024;

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
        Ok(self.db.contains_key(id)?)
    }

    async fn get(&self, id: &str) -> Result<Response> {
        let mut paste_path = PathBuf::from(&self.base_dir);
        paste_path.push(id);

        if !paste_path.is_file().await {
            return Err(format_err!("Internal Error"));
        }

        let mut file = File::open(&paste_path).await?;

        // Check file size
        if self.get_meta(id)?.size < MAX_STREAM_FILE_SIZE {
            let mut content: Vec<u8> = Vec::new();
            file.read_to_end(&mut content).await?;

            Ok(Response::Content(content))
        } else {
            let stream = FramedRead::new(file, BytesCodec::new());
            Ok(Response::Stream(stream))
        }
    }

    fn get_meta(&self, id: &str) -> Result<PasteMeta> {
        let meta: PasteMeta = match self.db.get(&id)? {
            Some(bin) => bincode::deserialize(&bin)?,
            None => {
                return Err(format_err!("Paste not found".to_string()));
            }
        };

        Ok(meta)
    }

    fn get_all_meta(&self) -> Result<Vec<(String, PasteMeta)>> {
        let mut metas = Vec::new();
        for id_u8 in self.db.iter().keys() {
            let id = String::from_utf8(id_u8?.to_vec())?;
            let meta = self.get_meta(&id)?;
            metas.push((id, meta));
        }

        Ok(metas)
    }

    async fn new(&self, id: &str, key: &str) -> Result<File> {
        if self.exists(id)? {
            return Err(format_err!("A paste with this id already exists"));
        }

        let mut new_path = PathBuf::from(&self.base_dir);
        new_path.push(id);
        let content_file = fs::File::create(&new_path).await?;

        // Add meta
        self.set_meta(
            &id,
            &PasteMeta {
                create_time: Utc::now(),
                expire_time: None,
                atime: None,
                name: None,
                size: 0, // Set it to 0 for now
                key: key.to_string(),
            },
        )?;

        Ok(content_file)
    }

    fn set_meta(&self, id: &str, meta: &PasteMeta) -> Result<()> {
        let meta_bin = bincode::serialize(&meta)?;
        self.db.insert(&id, meta_bin)?;

        Ok(())
    }

    async fn update_size(&self, id: &str) -> Result<()> {
        let mut meta = self.get_meta(id)?;

        let paste_path = self.base_dir.join(id);
        let file_meta = fs::metadata(&paste_path).await?;

        meta.size = file_meta.len();
        self.set_meta(id, &meta)?;
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
        self.db.remove(&id)?;

        Ok(())
    }

    async fn cleanup(&self, max_size: Option<u64>) -> Result<Vec<String>> {
        debug!("Begin deleting expired pastes...");
        let mut deleted = Vec::new();

        // Calculate delete coefficient
        let mut total_size = 0;
        let mut delete_coefficient: HashMap<String, u64> = HashMap::new();
        // Go though all expire times
        for id_u8 in self.db.iter().keys() {
            let id = skip_fail!(String::from_utf8(skip_fail!(id_u8).to_vec()));
            let meta = skip_fail!(self.get_meta(&id));
            if let Some(exp_time) = meta.expire_time {
                if Utc::now() >= exp_time {
                    // It's expired, delete it!
                    skip_fail!(self.delete(&id).await);
                    deleted.push(id.clone());
                }
            } else if meta.size == 0 {
                // Delete empty paste
                skip_fail!(self.delete(&id).await);
                deleted.push(id.clone());
            } else if max_size.is_some() {
                // Calculate total size
                total_size += meta.size;
                // Calculate delete coefficient
                delete_coefficient.insert(
                    id,
                    calculate_delete_coefficient(
                        meta.size,
                        (Utc::now() - meta.create_time).num_seconds() as u64,
                    ),
                );
            }
        }

        // delete largest until within constraints
        if let Some(max_size) = max_size {
            while total_size > max_size {
                // Find the current largest coefficient
                let (id, _) = delete_coefficient.iter().max_by_key(|entry| entry.1).unwrap();
                debug!("Deleting {} due to size constraint.", &id);
                skip_fail!(self.delete(&id).await);
            }
        }
        debug!("Finish deleting expired pastes.");
        Ok(deleted)
    }
}

fn calculate_delete_coefficient(size: u64, oldness: u64) -> u64 {
    if size > MAX_SIZE {
        oldness * (size - MAX_SIZE)
    } else {
        oldness
    }
}
