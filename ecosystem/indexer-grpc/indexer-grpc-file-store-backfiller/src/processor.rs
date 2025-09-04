// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Context, Result};
use velor_indexer_grpc_utils::{
    compression_util::StorageFormat, config::IndexerGrpcFileStoreConfig, create_grpc_client,
    file_store_operator::FileStoreOperator,
};
use velor_protos::{
    internal::fullnode::v1::{
        stream_status::StatusType, transactions_from_node_response::Response,
        GetTransactionsFromNodeRequest, TransactionsFromNodeResponse,
    },
    transaction::v1::Transaction,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    process::exit,
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;

/// Processor tails the data in cache and stores the data in file store.
pub struct Processor {
    file_store_operator: Box<dyn FileStoreOperator>,
    chain_id: u64,
    grpc_stream: Option<tonic::Streaming<TransactionsFromNodeResponse>>,
    starting_version: u64,
    progress_file_path: String,
    ending_version: Option<u64>,
    validation_mode: bool,
    backfill_processing_task_count: usize,
    validating_task_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressFile {
    version: u64,
}

impl Processor {
    pub async fn new(
        fullnode_grpc_address: url::Url,
        file_store_config: IndexerGrpcFileStoreConfig,
        chain_id: u64,
        enable_cache_compression: bool,
        progress_file_path: String,
        starting_version: Option<u64>,
        transactions_count: Option<u64>,
        validation_mode: bool,
        backfill_processing_task_count: usize,
        validating_task_count: usize,
    ) -> Result<Self> {
        let _cache_storage_format = if enable_cache_compression {
            StorageFormat::Lz4CompressedProto
        } else {
            StorageFormat::Base64UncompressedProto
        };
        if validation_mode {
            return Ok(Self {
                file_store_operator: file_store_config.create(),
                chain_id,
                grpc_stream: None,
                starting_version: starting_version.unwrap_or(0),
                progress_file_path,
                ending_version: transactions_count.map(|c| starting_version.unwrap_or(0) + c),
                validation_mode,
                backfill_processing_task_count,
                validating_task_count,
            });
        }

        let starting_version = starting_version.unwrap_or(0);
        let expected_end_version = transactions_count.map(|c| starting_version + c);
        // Resume from the progress file if the file exists. If not, create an empty one.
        let progress_file: ProgressFile = match std::fs::read(&progress_file_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).context("Failed to parse progress file")?,
            Err(_) => {
                let progress_file = ProgressFile {
                    version: starting_version,
                };
                let bytes = serde_json::to_vec(&progress_file)
                    .context("Failed to serialize progress file")?;
                std::fs::write(&progress_file_path, bytes)
                    .context("Failed to write progress file")?;
                progress_file
            },
        };
        let expected_starting_version = std::cmp::max(starting_version, progress_file.version);
        tracing::info!(
            starting_version = expected_starting_version,
            "Starting backfill.",
        );
        if let Some(expected_end_version) = expected_end_version {
            if expected_starting_version >= expected_end_version {
                tracing::info!("Backfill is already done.");
                // Backfill is already done.
                exit(0);
            }
        }

        // Create a grpc client to the fullnode.
        let mut grpc_client = create_grpc_client(fullnode_grpc_address.clone()).await;
        let request = tonic::Request::new(GetTransactionsFromNodeRequest {
            starting_version: Some(expected_starting_version),
            transactions_count,
        });
        let stream = grpc_client
            .get_transactions_from_node(request)
            .await?
            .into_inner();
        let file_store_operator: Box<dyn FileStoreOperator> = file_store_config.create();
        file_store_operator.verify_storage_bucket_existence().await;
        // Metadata is guaranteed to exist now
        let metadata = file_store_operator.get_file_store_metadata().await.unwrap();
        ensure!(metadata.chain_id == chain_id, "Chain ID mismatch.");
        Ok(Self {
            file_store_operator,
            chain_id,
            grpc_stream: Some(stream),
            starting_version: expected_starting_version,
            progress_file_path,
            ending_version: expected_end_version,
            validation_mode,
            backfill_processing_task_count,
            validating_task_count,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        if self.validation_mode {
            self.validate().await?;
        } else {
            self.backfill().await?;
        }
        Ok(())
    }

    pub async fn backfill(&mut self) -> Result<()> {
        let (sender, receiver) = tokio::sync::mpsc::channel::<Vec<Transaction>>(1000);
        // Get the stream out.
        // This is required, since the batch returned by the stream is not guaranteed to be 1000.
        let mut transactions_buffer = BTreeMap::new();
        let mut next_version_to_process = self.starting_version;
        // TODO: Use a more efficient data structure, like channel to manage the processed versions.
        let finished_starting_versions = Arc::new(Mutex::new(BTreeSet::new()));
        let chain_id = self.chain_id;
        let ending_version = self.ending_version;

        let mut grpc_stream = self.grpc_stream.take().expect("Stream is not initialized.");
        let init_frame = grpc_stream
            .next()
            .await
            .expect("Failed to get the first frame")?
            .response
            .unwrap();
        match init_frame {
            Response::Status(signal) => {
                if signal.r#type() != StatusType::Init {
                    anyhow::bail!("Unexpected status signal type");
                }
            },
            _ => {
                anyhow::bail!("Unexpected response type");
            },
        }
        let mut tasks = Vec::new();
        let receiver_ref = std::sync::Arc::new(Mutex::new(receiver));
        let file_store_operator = self.file_store_operator.clone_box();
        for _ in 0..self.backfill_processing_task_count {
            tracing::info!("Creating a new task");
            let mut current_file_store_operator = file_store_operator.clone_box();
            let current_finished_starting_versions = finished_starting_versions.clone();
            let receiver_ref = receiver_ref.clone();
            let task = tokio::spawn(async move {
                tracing::info!("Task started");
                loop {
                    let transactions = {
                        let mut receiver = receiver_ref.lock().await;
                        // Connection may end.
                        let transactions = match receiver.recv().await {
                            Some(transactions) => transactions,
                            None => return Ok(()),
                        };
                        // Data quality check.
                        ensure!(transactions.len() == 1000, "Unexpected transaction count");
                        ensure!(
                            transactions[0].version % 1000 == 0,
                            "Unexpected starting version"
                        );
                        for (ide, t) in transactions.iter().enumerate() {
                            ensure!(
                                t.version == transactions[0].version + ide as u64,
                                "Unexpected version"
                            );
                        }
                        transactions
                    };
                    let starting_version = transactions[0].version;
                    // If uploading failure, crash the process and let k8s restart it.
                    current_file_store_operator
                        .upload_transaction_batch(chain_id, transactions)
                        .await
                        .unwrap();
                    {
                        let mut finished_starting_versions =
                            current_finished_starting_versions.lock().await;
                        finished_starting_versions.insert(starting_version);
                    }
                }
            });
            tasks.push(task);
        }
        let progress_file_path = self.progress_file_path.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(5000)).await;
                {
                    {
                        let mut finished_starting_versions =
                            finished_starting_versions.lock().await;
                        let mut need_to_update = false;
                        loop {
                            if finished_starting_versions.contains(&next_version_to_process) {
                                finished_starting_versions.remove(&next_version_to_process);
                                next_version_to_process += 1000;
                                need_to_update = true;
                            } else {
                                break;
                            }
                        }
                        if !need_to_update {
                            continue;
                        }
                    }
                    // Update the progress file.
                    let progress_file = ProgressFile {
                        version: next_version_to_process,
                    };
                    let bytes = serde_json::to_vec(&progress_file)
                        .context("Failed to serialize progress file")?;
                    std::fs::write(&progress_file_path, &bytes)
                        .context("Failed to write progress file")?;
                    tracing::info!(
                        "Progress file updated to version {}",
                        next_version_to_process
                    );
                    if let Some(ending_version) = ending_version {
                        if ending_version <= next_version_to_process {
                            // Backfill is done.
                            std::process::exit(0);
                        }
                    }
                }
            }
        });
        tasks.push(task);

        // Clone the sender to extend its lifetime.
        let sender = sender.clone();
        // Start the stream.
        loop {
            let item = grpc_stream.next().await;
            let item = item.unwrap();
            let response = match item {
                Ok(response) => response,
                Err(e) => {
                    tracing::error!("Failed to get response: {:?}", e);
                    panic!("Failed to get response: {:?}", e);
                },
            };

            let resp = response.response.unwrap();
            match resp {
                Response::Data(txns) => {
                    let transactions = txns.transactions;
                    for txn in transactions {
                        let version = txn.version;
                        // Partial batch may be received; split and insert into buffer.
                        transactions_buffer.insert(version, txn);
                    }
                },
                Response::Status(signal) => {
                    if signal.r#type() != StatusType::BatchEnd {
                        anyhow::bail!("Unexpected status signal type");
                    }
                    while transactions_buffer.len() >= 1000 {
                        // Take the first 1000 transactions.
                        let mut transactions = Vec::new();
                        // Pop the first 1000 transactions from buffer.
                        for _ in 0..1000 {
                            let (_, txn) = transactions_buffer.pop_first().unwrap();
                            transactions.push(txn);
                        }
                        sender.send(transactions).await?;
                    }
                },
            }
        }
    }

    pub async fn validate(&mut self) -> Result<()> {
        let progress_file = {
            let bytes =
                std::fs::read(&self.progress_file_path).context("Failed to read progress file");
            match bytes {
                Ok(bytes) => {
                    serde_json::from_slice(&bytes).context("Failed to parse progress file")?
                },
                _ => ProgressFile { version: 0 },
            }
        };
        let start_version = std::cmp::max(self.starting_version, progress_file.version);
        let mut current_version = start_version;
        let expected_end_version = self.ending_version.unwrap();
        if start_version >= expected_end_version {
            // Validation is already done.
            exit(0);
        }
        let version_allocator = Arc::new(Mutex::new(start_version));
        let gap_detector = Arc::new(Mutex::new(BTreeSet::new()));
        let mut tasks = Vec::new();

        for _ in 0..self.validating_task_count {
            let version_allocator = version_allocator.clone();
            let gap_detector = gap_detector.clone();
            let file_operator = self.file_store_operator.clone_box();

            let task = tokio::spawn(async move {
                loop {
                    let version = {
                        let mut version_allocator = version_allocator.lock().await;
                        let version = *version_allocator;
                        if version >= expected_end_version {
                            return Ok(());
                        }
                        *version_allocator += 1000;
                        version
                    };
                    let transactions = file_operator.get_transactions(version, 1).await.unwrap();
                    for (idx, t) in transactions.iter().enumerate() {
                        ensure!(t.version == version + idx as u64, "Unexpected version");
                    }

                    let mut gap_detector = gap_detector.lock().await;
                    gap_detector.insert(version);
                }
            });
            tasks.push(task);
        }
        let progress_file_path = self.progress_file_path.clone();
        // Check the gap detector and update the progress file.
        let task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(5000)).await;
                loop {
                    if current_version >= expected_end_version {
                        return Ok(());
                    }
                    let mut gap_detector = gap_detector.lock().await;
                    if gap_detector.contains(&current_version) {
                        gap_detector.remove(&current_version);
                        current_version += 1000;
                    } else {
                        break;
                    }
                }
                let progress_file = ProgressFile {
                    version: current_version,
                };
                let bytes = serde_json::to_vec(&progress_file)
                    .context("Failed to serialize progress file")?;
                std::fs::write(&progress_file_path, &bytes).context("io error")?;
            }
        });
        tasks.push(task);
        // join all tasks
        for task in tasks {
            task.await??;
        }

        Ok(())
    }
}
