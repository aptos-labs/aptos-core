// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    config::IndexerGrpcFileStoreConfig,
    file_store_operator::{FileStoreOperator, GcsFileStoreOperator},
};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
use tracing::{error, info};

const NUM_OF_PROCESSING_THREADS: usize = 100;
const UPDATE_INTERVAL_IN_MILLISECONDS: u64 = 10000;

pub struct Processor {
    legacy_file_store_operator: GcsFileStoreOperator,
    new_file_store_operator: GcsFileStoreOperator,
    chain_id: u64,
}

impl Processor {
    pub fn new(
        legacy_file_store_config: IndexerGrpcFileStoreConfig,
        new_file_store_config: IndexerGrpcFileStoreConfig,
        chain_id: u64,
    ) -> Self {
        let legacy_file_store_operator = match legacy_file_store_config {
            IndexerGrpcFileStoreConfig::GcsFileStore(config) => {
                let service_account_path = config.gcs_file_store_service_account_key_path;
                let bucket_name = config.gcs_file_store_bucket_name;
                GcsFileStoreOperator::new(
                    bucket_name,
                    service_account_path,
                    config.enable_compression,
                )
            },
            _ => panic!("Only GCS file store config supported."),
        };
        let new_file_store_operator = match new_file_store_config {
            IndexerGrpcFileStoreConfig::GcsFileStore(config) => {
                let service_account_path = config.gcs_file_store_service_account_key_path;
                let bucket_name = config.gcs_file_store_bucket_name;
                GcsFileStoreOperator::new(
                    bucket_name,
                    service_account_path,
                    config.enable_compression,
                )
            },
            _ => panic!("Only GCS file store config supported."),
        };
        Self {
            legacy_file_store_operator,
            new_file_store_operator,
            chain_id,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Start to run gcs migration processor");
        self.legacy_file_store_operator
            .verify_storage_bucket_existence()
            .await;
        // get the legacy file store metadata.
        let legacy_file_store_metadata = self
            .legacy_file_store_operator
            .get_file_store_metadata()
            .await
            .expect("Failed to get legacy file store metadata");
        let max_version = legacy_file_store_metadata.version;
        // verify the chain ID.
        if legacy_file_store_metadata.chain_id != self.chain_id {
            panic!(
                "Chain ID mismatch: expected {}, got {}",
                self.chain_id, legacy_file_store_metadata.chain_id
            );
        }

        self.new_file_store_operator
            .verify_storage_bucket_existence()
            .await;

        let new_file_store_metadata = self.new_file_store_operator.get_file_store_metadata().await;
        let next_version_to_process = match new_file_store_metadata {
            Some(metadata) => {
                if metadata.chain_id != self.chain_id {
                    panic!(
                        "Chain ID mismatch: expected {}, got {}",
                        self.chain_id, metadata.chain_id
                    );
                }
                metadata.version
            },
            None => {
                self.new_file_store_operator
                    .update_file_store_metadata_internal(self.chain_id, 0)
                    .await?;
                0
            },
        };
        info!("Start to process from version: {}", next_version_to_process);
        let task_allocation = Arc::new(Mutex::new(next_version_to_process));
        let running_tasks: Arc<Mutex<BTreeSet<u64>>> = Arc::new(Mutex::new(BTreeSet::new()));
        let mut task_handlers = Vec::new();
        let chain_id = self.chain_id;
        for _ in 0..NUM_OF_PROCESSING_THREADS {
            let legacy_file_store_operator = self.legacy_file_store_operator.clone();
            let new_file_store_operator = self.new_file_store_operator.clone();
            let task_allocation = task_allocation.clone();
            let running_tasks = running_tasks.clone();
            let t = tokio::spawn(async move {
                loop {
                    let version_to_process = {
                        let mut task_allocation = task_allocation.lock().unwrap();
                        let ret = *task_allocation;
                        if ret >= max_version {
                            // Finish processing.
                            break;
                        }
                        *task_allocation += 1000;
                        ret
                    };
                    {
                        let running_tasks = running_tasks.lock().unwrap();
                        if running_tasks.contains(&version_to_process) {
                            panic!("Duplicated version to process: {}", version_to_process);
                        }
                    }
                    // Insert into running tasks.
                    {
                        let mut running_tasks = running_tasks.lock().unwrap();
                        running_tasks.insert(version_to_process);
                    }
                    let legacy_file_store_operator = legacy_file_store_operator.clone();
                    let mut new_file_store_operator = new_file_store_operator.clone();

                    // Simple retry to process the version.
                    loop {
                        // download files from legacy bucket.
                        let transactions = match legacy_file_store_operator
                            .get_transactions(version_to_process, 1)
                            .await
                        {
                            Ok(transactions) => transactions,
                            Err(e) => {
                                error!("Failed to download transactions from legacy bucket: {}", e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                continue;
                            },
                        };

                        // upload files to new bucket.
                        match new_file_store_operator
                            .upload_transaction_batch(2, transactions)
                            .await
                        {
                            Ok(_) => {
                                break;
                            },
                            Err(e) => {
                                error!("Failed to upload transactions to new bucket: {}", e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                continue;
                            },
                        }
                    }
                    // Remove from running tasks.
                    {
                        let mut running_tasks = running_tasks.lock().unwrap();
                        running_tasks.remove(&version_to_process);
                    }
                }
            });
            task_handlers.push(t);
        }

        // sleep for 10 seconds.
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        // watchdog thread.
        let mut new_file_store_operator = self.new_file_store_operator.clone();
        let t = tokio::spawn(async move {
            let mut failure_count = 0;
            loop {
                if failure_count >= 100 {
                    panic!("Failed to update file store metadata for 100 times");
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    UPDATE_INTERVAL_IN_MILLISECONDS,
                ))
                .await;
                let new_metadata_version = {
                    let running_tasks = running_tasks.lock().unwrap();
                    if running_tasks.is_empty() {
                        // All tasks are finished.
                        break;
                    }
                    // get first running task.
                    let first_running_task = *running_tasks.iter().next().unwrap();
                    first_running_task
                };
                let mut new_file_store_operator = new_file_store_operator.clone();
                let current_metadata = new_file_store_operator
                    .get_file_store_metadata()
                    .await
                    .unwrap();
                if current_metadata.version >= new_metadata_version {
                    continue;
                }
                match new_file_store_operator
                    .update_file_store_metadata_internal(chain_id, new_metadata_version)
                    .await
                {
                    Ok(_) => {
                        failure_count = 0;
                        info!(
                            "Updated file store metadata to version: {}",
                            new_metadata_version
                        );
                    },
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        failure_count += 1;
                    },
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            info!("All tasks are finished");
            // update the file store metadata to the max version.
            new_file_store_operator
                .update_file_store_metadata_internal(chain_id, max_version)
                .await
                .expect("Failed to update file store metadata");
        });
        task_handlers.push(t);
        // join all.
        for t in task_handlers {
            t.await?;
        }
        // Both processing tasks and the watchdog task are finished.
        Ok(())
    }
}
