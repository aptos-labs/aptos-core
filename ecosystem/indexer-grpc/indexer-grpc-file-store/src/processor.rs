// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    get_file_store_bucket_name,
    storage::{
        generate_blob_name, get_file_store_metadata, upload_file_store_metadata, TransactionsBlob,
        BLOB_TRANSACTION_CHUNK_SIZE,
    },
};
use aptos_moving_average::MovingAverage;
use cloud_storage::Object;
use redis::{Client, Commands};
use std::{thread::sleep, time::Duration};

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

        let mut metadata = get_file_store_metadata(bucket_name.clone()).await;

        // The current version is the version of the last blob that was uploaded.
        self.current_version = metadata.version;

        let mut metadata_ref = &mut metadata;
        // TODO: fix this with a proper traffic control mechanism.
        let mut prev_metadata_update_time = std::time::Instant::now();

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
                        bucket = bucket_name,
                        version = self.current_version,
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
            if prev_metadata_update_time + Duration::from_secs(10) < std::time::Instant::now() {
                upload_file_store_metadata(bucket_name.clone(), *metadata_ref).await;
                prev_metadata_update_time = std::time::Instant::now();
            }
        }
    }
}
