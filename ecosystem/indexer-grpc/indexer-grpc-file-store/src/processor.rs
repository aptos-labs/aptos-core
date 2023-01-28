// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    get_file_store_bucket_name,
    storage::{
        generate_blob_name, get_file_store_metadata, upload_file_store_metadata,
        BLOB_TRANSACTION_CHUNK_SIZE,
    },
    CACHE_KEY_CHAIN_ID,
};
use aptos_moving_average::MovingAverage;
use cloud_storage::Object;
use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use std::{thread::sleep, time::Duration};

#[derive(Serialize, Deserialize)]
struct TransactionsBlob {
    /// The version of the first transaction in the blob.
    pub starting_version: u64,
    /// The transactions in the blob.
    pub transactions: Vec<String>,
}

pub struct Processor {
    pub redis_client: Client,
    current_version: u64,
}

async fn upload_blob_transactions(
    bucket_name: String,
    blob_object: TransactionsBlob,
) -> anyhow::Result<()> {
    match Object::create(
        bucket_name.as_str(),
        serde_json::to_vec(&blob_object).unwrap(),
        generate_blob_name(blob_object.starting_version).as_str(),
        "application/json",
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            aptos_logger::info!(
                error = err.to_string(),
                "[indexer file store] Failed to process a blob; retrying in 1 second"
            );
            sleep(Duration::from_secs(1));
            Err(err.into())
        },
    }
}

impl Processor {
    pub fn new(redis_address: String) -> Self {
        Self {
            redis_client: Client::open(format!("redis://{}", redis_address)).unwrap(),
            current_version: 0,
        }
    }

    // Starts the processing.
    pub async fn run(&mut self) {
        let mut conn = self.redis_client.get_connection().unwrap();
        let mut ma = MovingAverage::new(10_000);

        let bucket_name = get_file_store_bucket_name();
        let redis_chain_id = conn
            .get::<String, String>(CACHE_KEY_CHAIN_ID.to_string())
            .unwrap();

        let mut metadata = get_file_store_metadata(bucket_name.clone()).await;
        // It's fatal if the chain_id doesn't match; this is a safety check.
        assert_eq!(redis_chain_id, metadata.chain_id.to_string());

        // The current version is the version of the last blob that was uploaded.
        self.current_version = metadata.version;

        let mut metadata_ref = &mut metadata;

        loop {
            let versions = (self.current_version
                ..self.current_version + BLOB_TRANSACTION_CHUNK_SIZE)
                .map(|e| e.to_string())
                .collect::<Vec<String>>();
            let transactions_blob = match conn.mget::<Vec<String>, Vec<String>>(versions) {
                Ok(data) => TransactionsBlob {
                    starting_version: self.current_version,
                    transactions: data,
                },
                Err(err) => {
                    aptos_logger::info!(
                        error = err.to_string(),
                        "[indexer file store] Hit the head; retrying in 1 second"
                    );
                    sleep(Duration::from_secs(1));
                    continue;
                },
            };

            match upload_blob_transactions(bucket_name.clone(), transactions_blob).await {
                Ok(_) => {
                    self.current_version += BLOB_TRANSACTION_CHUNK_SIZE;
                    metadata_ref.version += BLOB_TRANSACTION_CHUNK_SIZE;

                    ma.tick_now(BLOB_TRANSACTION_CHUNK_SIZE);
                    aptos_logger::info!(
                        version = self.current_version,
                        tps = (ma.avg() * 1000.0) as u64,
                        "[indexer file store] Processed a blob"
                    );
                },
                Err(err) => {
                    aptos_logger::error!(
                        error = err.to_string(),
                        "[indexer file store] Failed to process a blob; retrying in 1 second"
                    );
                    continue;
                },
            }
            upload_file_store_metadata(bucket_name.clone(), *metadata_ref).await;
        }
    }
}
