// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Context, Result};
use velor_indexer_grpc_utils::{
    compression_util::{FileEntry, StorageFormat},
    config::IndexerGrpcFileStoreConfig,
    create_grpc_client,
    file_store_operator_v2::{
        common::{BatchMetadata, IFileStore},
        file_store_operator::FileStoreOperatorV2,
        file_store_reader::FileStoreReader,
    },
};
use velor_protos::{
    internal::fullnode::v1::{
        transactions_from_node_response::Response, GetTransactionsFromNodeRequest,
    },
    transaction::v1::Transaction,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    process::exit,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::info;
use url::Url;

const MAX_SIZE_PER_FILE: usize = 50 * (1 << 20);

pub struct Processor {
    fullnode_grpc_address: Url,
    chain_id: u64,
    starting_version: u64,
    ending_version: u64,
    num_transactions_per_folder: u64,
    file_store_reader: FileStoreReader,
    file_store_writer: Arc<dyn IFileStore>,
    progress_file_path: String,
    backfill_id: u64,
    backfill_processing_task_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressFile {
    version: u64,
    backfill_id: u64,
}

impl Processor {
    pub async fn new(
        fullnode_grpc_address: Url,
        file_store_config: IndexerGrpcFileStoreConfig,
        chain_id: u64,
        progress_file_path: String,
        starting_version: u64,
        ending_version: u64,
        backfill_processing_task_count: usize,
    ) -> Result<Self> {
        let file_store = file_store_config.create_filestore().await;
        ensure!(file_store.is_initialized().await);
        let file_store_reader = FileStoreReader::new(chain_id, file_store.clone()).await;
        let metadata = file_store_reader.get_file_store_metadata().await.unwrap();
        ensure!(metadata.chain_id == chain_id, "Chain ID mismatch.");
        let num_transactions_per_folder = metadata.num_transactions_per_folder;
        ensure!(
            starting_version % num_transactions_per_folder == 0
                && ending_version % num_transactions_per_folder == 0,
            "starting_version and ending_version must be multiply of num_transactions_per_folder ({num_transactions_per_folder})."
        );

        // Resume from the progress file if the file exists. If not, create an empty one.
        let progress_file: ProgressFile = match std::fs::read(&progress_file_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).context("Failed to parse progress file")?,
            Err(_) => {
                let progress_file = ProgressFile {
                    version: starting_version,
                    backfill_id: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                let bytes = serde_json::to_vec(&progress_file)
                    .context("Failed to serialize progress file")?;
                std::fs::write(&progress_file_path, bytes)
                    .context("Failed to write progress file")?;
                progress_file
            },
        };

        ensure!(
            progress_file.version % num_transactions_per_folder == 0,
            "version in the progress file must be the multiply of num_transactions_per_folder ({num_transactions_per_folder})."
        );

        let starting_version = std::cmp::max(starting_version, progress_file.version);
        info!(starting_version = starting_version, "Starting backfill.",);
        if starting_version >= ending_version {
            info!("Backfill is already done.");
            // Backfill is already done.
            exit(0);
        }

        Ok(Self {
            fullnode_grpc_address,
            chain_id,
            starting_version,
            ending_version,
            num_transactions_per_folder,
            file_store_reader,
            file_store_writer: file_store,
            progress_file_path,
            backfill_id: progress_file.backfill_id,
            backfill_processing_task_count,
        })
    }

    pub async fn run(&self) -> Result<()> {
        self.backfill().await
    }

    pub async fn backfill(&self) -> Result<()> {
        let mut version = self.starting_version;
        while version < self.ending_version {
            tokio_scoped::scope(|s| {
                for _ in 0..self.backfill_processing_task_count {
                    let task_version = version;
                    if task_version >= self.ending_version {
                        break;
                    }
                    let mut file_store_operator = FileStoreOperatorV2::new(
                        MAX_SIZE_PER_FILE,
                        self.num_transactions_per_folder,
                        version,
                        BatchMetadata::default(),
                    );

                    info!(
                        "Backfilling versions [{task_version}, {}).",
                        task_version + self.num_transactions_per_folder
                    );

                    let chain_id = self.chain_id as u32;
                    let num_transactions_per_folder = self.num_transactions_per_folder;
                    let fullnode_grpc_address = self.fullnode_grpc_address.clone();

                    let (tx, mut rx) = tokio::sync::mpsc::channel(10);

                    s.spawn(async move {
                        while let Some((transactions, batch_metadata, end_batch)) = rx.recv().await
                        {
                            self.do_upload(transactions, batch_metadata, end_batch)
                                .await
                                .unwrap();
                        }
                    });

                    s.spawn(async move {
                        // Create a grpc client to the fullnode.
                        let mut grpc_client = create_grpc_client(fullnode_grpc_address).await;
                        let request = tonic::Request::new(GetTransactionsFromNodeRequest {
                            starting_version: Some(task_version),
                            transactions_count: Some(num_transactions_per_folder),
                        });
                        let mut stream = grpc_client
                            .get_transactions_from_node(request)
                            .await
                            .unwrap()
                            .into_inner();

                        while let Some(response_item) = stream.next().await {
                            match response_item {
                                Ok(r) => {
                                    assert!(r.chain_id == chain_id);
                                    match r.response.unwrap() {
                                        Response::Data(data) => {
                                            let transactions = data.transactions;
                                            for transaction in transactions {
                                                file_store_operator
                                                    .buffer_and_maybe_dump_transactions_to_file(
                                                        transaction,
                                                        tx.clone(),
                                                    )
                                                    .await
                                                    .unwrap();
                                            }
                                        },
                                        Response::Status(_) => {
                                            continue;
                                        },
                                    }
                                },
                                Err(e) => {
                                    panic!("Error when getting transactions from fullnode: {e}.")
                                },
                            }
                        }

                        info!(
                            "Backfilling versions [{task_version}, {}) is finished.",
                            task_version + num_transactions_per_folder
                        );
                    });

                    version += self.num_transactions_per_folder;
                }
            });

            // Update the progress file.
            let progress_file = ProgressFile {
                version,
                backfill_id: self.backfill_id,
            };
            let bytes =
                serde_json::to_vec(&progress_file).context("Failed to serialize progress file.")?;
            std::fs::write(&self.progress_file_path, &bytes)
                .context("Failed to write progress file.")?;
            info!("Progress file updated to version {}.", version,);
        }

        Ok(())
    }

    async fn do_upload(
        &self,
        transactions: Vec<Transaction>,
        mut batch_metadata: BatchMetadata,
        end_batch: bool,
    ) -> Result<()> {
        let first_version = transactions.first().unwrap().version;

        let path = self
            .file_store_reader
            .get_path_for_version(first_version, Some(self.backfill_id));

        let data_file =
            FileEntry::from_transactions(transactions, StorageFormat::Lz4CompressedProto);

        self.file_store_writer
            .save_raw_file(path, data_file.into_inner())
            .await?;

        if end_batch {
            let path = self
                .file_store_reader
                .get_path_for_batch_metadata(first_version);
            batch_metadata.suffix = Some(self.backfill_id);
            self.file_store_writer
                .save_raw_file(
                    path,
                    serde_json::to_vec(&batch_metadata).map_err(anyhow::Error::msg)?,
                )
                .await?;
        }

        Ok(())
    }
}
