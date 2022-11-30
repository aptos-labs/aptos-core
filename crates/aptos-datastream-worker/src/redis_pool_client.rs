// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use deadpool_redis::{redis::cmd, Config, Pool, Runtime};
use std::sync::Arc;

#[derive(Debug)]
pub struct RedisWork {
    pub key: String,
    pub val: String,
}

impl RedisWork {
    pub fn new(version: u64, val: String) -> Self {
        Self {
            key: format!("txn:{}", version),
            val,
        }
    }
}

/// Client that talks with Redis
pub struct RedisClient {
    redis_pool: Arc<Pool>,
    address: String,
}

impl RedisClient {
    pub fn new(address: String) -> Self {
        let cfg = Config::from_url(format!("redis://{}", address));
        let redis_pool = Arc::new(cfg.create_pool(Some(Runtime::Tokio1)).unwrap());
        Self {
            redis_pool,
            address,
        }
    }

    pub async fn set(&self, key: String, data: String) {
        let mut conn = self.redis_pool.get().await.unwrap();
        cmd("SET")
            .arg(&[key, data])
            .query_async::<_, ()>(&mut conn)
            .await
            .unwrap();
    }

    pub async fn get(&self, key: String) -> String {
        let mut conn = self.redis_pool.get().await.unwrap();
        let value: String = cmd("GET").arg(&[key]).query_async(&mut conn).await.unwrap();
        value
    }
}
