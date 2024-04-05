// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_utils::config::IndexerGrpcFileStoreConfig;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

pub(crate) struct Worker {
    file_store_config: IndexerGrpcFileStoreConfig,
    starting_version: Option<u64>,
}

impl Worker {
    pub async fn new(
        file_store_config: IndexerGrpcFileStoreConfig,
        starting_version: Option<u64>,
    ) -> Result<Self> {
        Ok(Self {
            file_store_config,
            starting_version,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Running. Starting version: {}, ending version pending retrieval...", self.starting_version.unwrap_or(0));
        let file_store_operator = self.file_store_config.create();
        let starting_version = self.starting_version.unwrap_or(0);
        let cursor = Arc::new(Mutex::new(starting_version));
        let data_to_process = Arc::new(Mutex::new(BTreeMap::new()));
        let task_count = 32;
        let current_metadata = file_store_operator.get_file_store_metadata().await.unwrap();
        let ending_version = current_metadata.version;
        tracing::info!("Run operation starting. Starting version: {}, ending version: {}", starting_version, ending_version);
        for _ in 0..task_count {
            let file_store_operator = file_store_operator.clone_box();
            let cursor = cursor.clone();
            let data_to_process = data_to_process.clone();
            tracing::info!("Preparing to spawn {} tasks for data processing.", task_count);

            tokio::spawn(async move {
                loop {
                    let version_to_process = {
                        let mut cursor_now = cursor.lock().await;
                        let version_to_process = *cursor_now;
                        *cursor_now += 1000;
                        version_to_process
                    };
                    if version_to_process >= ending_version {
                        break;
                    }
                    tracing::info!("Task for version {} started.", version_to_process);
                    let transactions = file_store_operator
                        .get_transactions(version_to_process, 3)
                        .await
                        .unwrap();
                    let mut data_to_process_now = data_to_process.lock().await;
                    data_to_process_now.insert(version_to_process, transactions);
                    tracing::info!("Task for version {} completed.", version_to_process);
                }
            });
        }
        // Checker loop.
        let mut version_to_check = 0;
        loop {
            if version_to_check >= ending_version {
                break;
            }
            // Get the lowest version in data_to_process.
            let transactions = {
                let mut data_to_process_now = data_to_process.lock().await;
                if !data_to_process_now.contains_key(&version_to_check) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                    continue;
                }
                data_to_process_now.remove(&version_to_check).unwrap()
            };
            for transaction in transactions {
                // Log the expected and actual version before the assertion
                if transaction.version != version_to_check {
                    tracing::error!("Version mismatch: expected {}, got {}", version_to_check, transaction.version);
                }
                assert_eq!(transaction.version, version_to_check, "Version mismatch: expected {}, got {}", version_to_check, transaction.version);
                version_to_check += 1;
            }
        }

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            tracing::info!("Data Integrity Checker is done!");
        }
    }
}
