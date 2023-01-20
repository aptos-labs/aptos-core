// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use deadpool_redis::{Config, Runtime};
use std::{net::ToSocketAddrs, sync::Arc};
// use warp::Filter;
use cloud_storage::{Bucket, NewBucket,Object, Error, GoogleError};
use aptos_datastream_cold_store::constants::{APTOS_DATASTREAM_COLD_STORE_METADATA_FILE_NAME};
use aptos_datastream_cold_store::metadata::BlobMetadata;
use aptos_datastream_common::RunningMode;
use std::sync::Mutex;
use deadpool_redis::redis::cmd;

pub fn get_redis_address() -> String {
    std::env::var("REDIS_ADDRESS").expect("REDIS_ADDRESS is required.")
}

pub fn get_redis_port() -> String {
    std::env::var("REDIS_PORT").expect("REDIS_PORT is required.")
}

pub fn get_chain_id() -> u32 {
    std::env::var("CHAIN_ID").expect("CHAIN_ID is required.").parse().unwrap()
}

pub fn get_env() -> String {
    std::env::var("ENV").expect("ENV is required.")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    aptos_logger::Logger::new().init();
    let redis_address = get_redis_address();
    let redis_port = get_redis_port();

    let chain_id = get_chain_id();
    let env = get_env();
    // let storage_client = Arc::new(cloud_storage::Client::new());
    let processor = Arc::new(aptos_datastream_cold_store::processor::Processor::new(
        format!("redis://{}:{}", redis_address, redis_port),
        chain_id,
        env,
    ));
    let monitor_processor = processor.clone();
    processor.start().await;
    tokio::spawn(async move {
        loop {
            monitor_processor.monitor().await;
        }
    });
    tokio::spawn(async move {
        loop {
            processor.process().await;
        }
    });
    std::thread::park();
    Ok(())
}
