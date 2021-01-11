use crate::storage::simplestorage::SimpleStorage;
use crate::storage::Response;
/// This storage backend still utilizes filesystem, but attempts to speed things up by caching small pastes
/// and their metadata into redis.
use crate::storage::Storage;

use anyhow::{format_err, Result};
use async_std::path::Path;
use async_trait::async_trait;
use chrono::prelude::*;
use log::{debug, error, info, warn};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use tokio::fs::File;

pub struct RedisCachedStorage {
    con: MultiplexedConnection,
    backend: SimpleStorage,
}

impl RedisCachedStorage {
    async fn delete_in_redis(&self, id: &str) -> Result<()> {
        // First check if content is in redis
        let mut con = self.con.clone();
        let content_redis_location = String::from(id) + ".content";
        let highlight_redis_location = String::from(id) + ".highlight";
        con.del::<&str, ()>(&content_redis_location)
            .await
            .unwrap_or(());
        con.del::<&str, ()>(&highlight_redis_location)
            .await
            .unwrap_or(()); // It can fail

        Ok(())
    }
}

impl RedisCachedStorage {
    pub async fn new(base: &Path, redis_addr: &str) -> Result<RedisCachedStorage> {
        let backend = SimpleStorage::new(base)?;
        let client = redis::Client::open(redis_addr)?;
        info!("Connecting to Redis instance on {}", redis_addr);
        let con = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|err| {
                format_err!("Failed to establish Redis connection: {}", err.to_string())
            })?;

        return Ok(RedisCachedStorage { con, backend });
    }
}

impl Clone for RedisCachedStorage {
    fn clone(&self) -> RedisCachedStorage {
        RedisCachedStorage {
            con: self.con.clone(),
            backend: self.backend.clone(),
        }
    }
}

#[async_trait]
impl Storage for RedisCachedStorage {
    fn exists(&self, id: &str) -> Result<bool> {
        self.backend.exists(id)
    }

    fn validate(&self, id: &str, key: &str) -> Result<bool> {
        self.backend.validate(id, key)
    }

    async fn get(&self, id: &str) -> Result<Response> {
        // First check if content is in redis
        let mut con = self.con.clone();
        let content_redis_location = String::from(id) + ".content";
        let exists_in_redis = match con.exists(&content_redis_location).await {
            Ok(res) => res,
            Err(_err) => {
                return self.backend.get(id).await;
            }
        };

        if exists_in_redis {
            debug!("Paste {} hit cache.", id);
            let result = match con.get(&content_redis_location).await {
                Ok(res) => res,
                Err(_err) => {
                    return self.backend.get(id).await;
                }
            };
            return Ok(Response::Content(result));
        } else {
            let result = self.backend.get(id).await?;
            match result {
                Response::Content(vec) => {
                    debug!("Paste {} miss cache, attemping to add to cache...", id);
                    // Try to write to the redis cache
                    let res = con
                        .set::<&str, &[u8], ()>(&content_redis_location, &vec)
                        .await;
                    match res {
                        Ok(_ok) => (),
                        Err(err) => warn!("Failed to write to Redis: {}", err.to_string()),
                    };

                    return Ok(Response::Content(vec));
                }
                Response::Stream(stream) => {
                    debug!("Paste {} too large for cache, passing stream.", id);
                    return Ok(Response::Stream(stream));
                }
            }
        }
    }

    async fn new(&self, id: &str, key: &str) -> Result<File> {
        self.backend.new(id, key).await
    }

    fn set_expire_time(&self, id: &str, time: &DateTime<Utc>) -> Result<()> {
        self.backend.set_expire_time(id, time)
    }

    async fn update(&self, id: &str) -> Result<File> {
        let res = self.backend.update(id).await?;
        self.delete_in_redis(id).await?;
        Ok(res)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        self.backend.delete(id).await?;
        self.delete_in_redis(id).await?;
        Ok(())
    }

    async fn cleanup(&self) -> Result<Vec<String>> {
        let deleted = self.backend.cleanup().await?;

        // Delete paste in Redis
        for id in &deleted {
            self.delete_in_redis(&id).await?;
        }

        Ok(deleted)
    }
}
