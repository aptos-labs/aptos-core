// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{generate_blob_name, get_file_store_blob_folder_name, get_file_store_bucket_name};
use aptos_indexer_grpc_utils::{
    storage::{get_file_store_metadata, FileStoreMetadata},
    CACHE_KEY_CHAIN_ID,
};
use aptos_logger::info;
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
    starting_version: u64,
    encoded_proto_data_vec: Vec<String>,
    blob_size: u64,
) -> anyhow::Result<()> {
    let blob_object: TransactionsBlob = TransactionsBlob {
        starting_version,
        transactions: encoded_proto_data_vec,
    };

    match Object::create(
        &get_file_store_bucket_name(),
        serde_json::to_vec(&blob_object).unwrap(),
        format!(
            "{}/{}",
            get_file_store_blob_folder_name(),
            generate_blob_name(starting_version, blob_size)
        )
        .as_str(),
        "application/json",
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            info!(
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

    pub async fn run(&mut self) {
        let bucket_name = get_file_store_bucket_name();
        let metadata = get_file_store_metadata(bucket_name).await;
        self.process(metadata).await;
    }

    // Starts the processing.
    async fn process(&mut self, mut metadata: FileStoreMetadata) {
        let mut conn = self.redis_client.get_connection().unwrap();
        let mut ma = MovingAverage::new(10_000);
        let bucket_name = get_file_store_bucket_name();

        let redis_chain_id = conn
            .get::<String, String>(CACHE_KEY_CHAIN_ID.to_string())
            .unwrap();

        // It's fatal if the chain_id doesn't match; this is a safety check.
        assert_eq!(redis_chain_id, metadata.chain_id.to_string());
        let blob_size = metadata.blob_size;
        // The current version is the version of the last blob that was uploaded.
        self.current_version = metadata.version;

        loop {
            let versions = (self.current_version..self.current_version + blob_size)
                .map(|e| e.to_string())
                .collect::<Vec<String>>();
            let encoded_proto_data_vec = match conn.mget::<Vec<String>, Vec<String>>(versions) {
                Ok(data) => data,
                Err(err) => {
                    info!(
                        error = err.to_string(),
                        "[indexer file store] Hit the head; retrying in 1 second"
                    );
                    sleep(Duration::from_secs(1));
                    continue;
                },
            };
            match upload_blob_transactions(self.current_version, encoded_proto_data_vec, blob_size)
                .await
            {
                Ok(_) => {
                    self.current_version += blob_size;
                    metadata.version += blob_size;

                    ma.tick_now(blob_size);
                    info!(
                        version = self.current_version,
                        tps = (ma.avg() * 1000.0) as u64,
                        "[indexer file store] Processed a blob"
                    );
                },
                Err(err) => {
                    info!(
                        error = err.to_string(),
                        "[indexer file store] Failed to process a blob; retrying in 1 second"
                    );
                },
            }
            // If the metadata is not updated, the indexer will be restarted.
            Object::create(
                bucket_name.as_str(),
                serde_json::to_vec(&metadata).unwrap(),
                "metadata.json",
                "application/json",
            )
            .await
            .unwrap();
        }
    }
}
