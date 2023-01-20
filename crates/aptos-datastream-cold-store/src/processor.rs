// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_datastream_common::{RunningMode, CACHE_KEY_CHAIN_ID, CACHE_KEY_CHAIN_NAME, CACHE_KEY_RUNNING_MODE};
use std::thread::sleep;
use std::sync::{Arc, Mutex};
use deadpool_redis::redis::cmd;
use std::time::Duration;

pub struct Processor {
    pub redis_pool: Arc<deadpool_redis::Pool>,
    chain_id: u32,
    chain_name: String,
    mode: Arc<Mutex<RunningMode>>,
}

impl Processor {
    pub fn new(redis_address: String, chain_id: u32, chain_name: String) -> Self {
        Self {
            redis_pool: Arc::new(deadpool_redis::Config::from_url(redis_address).create_pool(Some(deadpool_redis::Runtime::Tokio1)).unwrap()),
            chain_id,
            chain_name,
            mode: Arc::new(Mutex::new(RunningMode::Normal)),
        }
    }

    pub async fn start(&self) {
        let redis_pool = self.redis_pool.clone();
        let mut conn = redis_pool.get().await.unwrap();

        let if_cache_populated: bool = cmd("EXISTS")
            .arg(&[CACHE_KEY_RUNNING_MODE.to_string()])
            .query_async(&mut conn)
            .await
            .expect("Check if cahce is populated.");

        match if_cache_populated {
            true => {
                let (mode, chain_id, chain_name): (String, String, String) = cmd("MGET")
                    .arg(&[CACHE_KEY_RUNNING_MODE.to_string(), CACHE_KEY_CHAIN_ID.to_string(), CACHE_KEY_CHAIN_NAME.to_string()])
                    .query_async(&mut conn)
                    .await
                    .expect("Get running mode from cache.");

                assert_eq!(chain_id.parse::<u32>().unwrap(), self.chain_id);
                assert_eq!(chain_name, self.chain_name);

                *self.mode.lock().unwrap() = serde_json::from_str(&mode).unwrap();
            },
            false => {
                cmd("MSET")
                    .arg(&[
                        CACHE_KEY_RUNNING_MODE.to_string(), serde_json::to_string(&RunningMode::Default).unwrap(),
                        CACHE_KEY_CHAIN_ID.to_string(), self.chain_id.to_string(),
                        CACHE_KEY_CHAIN_NAME.to_string(), self.chain_name.to_string(),
                        ])
                    .query_async::<_, ()>(&mut conn)
                    .await
                    .expect("Populate the cache with mode Default.");
                *self.mode.lock().unwrap() = RunningMode::Default;
            },
        }
    }

    pub async fn monitor(&self) {
        let mut conn = self.redis_pool.get().await.unwrap();
        loop {
            let running_mode: String = cmd("GET")
                .arg(&[CACHE_KEY_RUNNING_MODE.to_string()])
                .query_async(&mut conn)
                .await
                .expect("Get running mode from cache.");

            *self.mode.lock().unwrap() = serde_json::from_str(&running_mode).unwrap();
            sleep(Duration::from_secs(1));
        }
    }
    pub async fn process(&self) {
        let mode = self.mode.lock().unwrap();
        match *mode {
            RunningMode::Normal => {
                println!("Normal mode");
            },
            RunningMode::Recovery => {
                println!("Recovery mode");
            },
            RunningMode::Bootstrap => {
                println!("Bootstrap mode");
            },
            RunningMode::Maintenance => {
                sleep(Duration::from_secs(10));
            },
            RunningMode::Default => {
                println!("Default mode");
            },
        }

    }
}
