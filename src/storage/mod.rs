use crate::storage::rediscachedstorage::RedisCachedStorage;
use crate::storage::simplestorage::SimpleStorage;

use anyhow::Result;
use async_std::path::Path;
use async_trait::async_trait;
use chrono::prelude::*;
use dyn_clone::DynClone;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

pub enum Response {
    Content(Vec<u8>),
    Stream(FramedRead<File, BytesCodec>),
}

#[async_trait]
pub trait Storage: DynClone + Send {
    // Non-mutating methods
    fn exists(&self, id: &str) -> Result<bool>;
    fn validate(&self, id: &str, key: &str) -> Result<bool>;
    async fn get(&self, id: &str) -> Result<Response>;
    fn get_name(&self, id: &str) -> Result<Option<String>>;
    fn size(&self, id: &str) -> Result<u64>;

    // Mutating methods
    async fn new(&self, id: &str, key: &str) -> Result<File>;
    fn set_expire_time(&self, id: &str, time: &DateTime<Utc>) -> Result<()>;
    fn set_name(&self, id: &str, name: &str) -> Result<()>;
    async fn update_size(&self, id: &str) -> Result<()>;
    async fn update(&self, id: &str) -> Result<File>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn cleanup(&self) -> Result<Vec<String>>; // Delete expired pastes
}

pub struct StorageBox {
    pub inner: Box<dyn Storage>,
}

impl StorageBox {
    pub async fn new(base_dir: &Path, redis_addr: Option<String>) -> Result<Self> {
        match redis_addr {
            Some(addr) => {
                return Ok(StorageBox {
                    inner: Box::new(RedisCachedStorage::new(base_dir, &addr).await?),
                });
            }
            None => {
                return Ok(StorageBox {
                    inner: Box::new(SimpleStorage::new(base_dir)?),
                });
            }
        }
    }
}

impl Clone for StorageBox {
    fn clone(&self) -> StorageBox {
        StorageBox {
            inner: dyn_clone::clone_box(&*self.inner),
        }
    }
}

pub mod rediscachedstorage;
pub mod simplestorage;
