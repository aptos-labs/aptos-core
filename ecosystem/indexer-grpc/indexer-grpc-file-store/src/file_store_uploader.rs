// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::data_manager::DataManager;
use anyhow::Result;
use aptos_indexer_grpc_utils::{
    compression_util::{FileEntry, StorageFormat},
    config::IndexerGrpcFileStoreConfig,
    file_store_operator_v2::{
        BatchMetadata, FileStoreMetadata, FileStoreOperatorV2, FileStoreReader, IFileStore,
        METADATA_FILE_NAME,
    },
};
use aptos_protos::transaction::v1::Transaction;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::mpsc::channel, time::Instant};
use tracing::info;

const NUM_TXNS_PER_FOLDER: u64 = 100000;
const MAX_SIZE_PER_FILE: usize = 20 * (1 << 20);

pub(crate) struct FileStoreUploader {
    chain_id: u64,
    reader: FileStoreReader,
    // TODO(grao): Change to IFileStoreReader when the trait_upcasting feature is in stable Rust.
    writer: Arc<dyn IFileStore>,

    last_batch_metadata_update_time: Option<Instant>,
    last_metadata_update_time: Instant,
}

impl FileStoreUploader {
    pub(crate) async fn new(
        chain_id: u64,
        file_store_config: IndexerGrpcFileStoreConfig,
    ) -> Result<Self> {
        let file_store = file_store_config.create_filestore().await;
        if !file_store.is_initialized().await {
            info!(
                chain_id = chain_id,
                "FileStore is not initialized, initializing..."
            );
            info!("Transactions per folder: {NUM_TXNS_PER_FOLDER}.");
            let metadata = FileStoreMetadata {
                chain_id,
                num_transactions_per_folder: NUM_TXNS_PER_FOLDER,
                version: 0,
            };
            let raw_data = serde_json::to_vec(&metadata).unwrap();
            file_store
                .save_raw_file(PathBuf::from(METADATA_FILE_NAME), raw_data)
                .await
                .unwrap_or_else(|e| panic!("Failed to initialize FileStore: {e:?}."));
        }

        let reader = FileStoreReader::new(chain_id, file_store.clone()).await;

        Ok(Self {
            chain_id,
            reader,
            writer: file_store,
            last_batch_metadata_update_time: None,
            last_metadata_update_time: Instant::now(),
        })
    }

    pub(crate) async fn start(&mut self, data_manager: Arc<DataManager>) -> Result<()> {
        let version = self
            .reader
            .get_latest_version()
            .await
            .expect("Latest version must exist.");
        let mut file_store_operator = FileStoreOperatorV2::new(MAX_SIZE_PER_FILE, version).await;
        tokio_scoped::scope(|s| {
            let (tx, mut rx) = channel(5);
            s.spawn(async move {
                while let Some((transactions, batch_metadata, end_batch)) = rx.recv().await {
                    self.do_upload(transactions, batch_metadata, end_batch)
                        .await
                        .unwrap();
                }
            });
            s.spawn(async move {
                loop {
                    let transactions = data_manager
                        .get_transactions_from_cache(
                            file_store_operator.version(),
                            MAX_SIZE_PER_FILE,
                            /*update_file_store_version=*/ true,
                        )
                        .await;
                    let len = transactions.len();
                    for transaction in transactions {
                        file_store_operator
                            .buffer_and_maybe_dump_transactions_to_file(
                                transaction,
                                NUM_TXNS_PER_FOLDER,
                                tx.clone(),
                            )
                            .await
                            .unwrap();
                    }
                    if len == 0 {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            });
        });

        Ok(())
    }

    async fn do_upload(
        &mut self,
        transactions: Vec<Transaction>,
        batch_metadata: BatchMetadata,
        end_batch: bool,
    ) -> Result<()> {
        let first_version = transactions.first().unwrap().version;
        let last_version = transactions.last().unwrap().version;
        let data_file =
            FileEntry::from_transactions(transactions, StorageFormat::Lz4CompressedProto);
        let path = self.reader.get_path_for_version(first_version);

        info!("Dumping transactions [{first_version}, {last_version}] to file {path:?}.");

        self.writer
            .save_raw_file(path, data_file.into_inner())
            .await?;

        let mut update_batch_metadata = false;
        let max_update_frequency = self.writer.max_update_frequency();
        if self.last_batch_metadata_update_time.is_none()
            || Instant::now() - self.last_batch_metadata_update_time.unwrap()
                >= max_update_frequency
        {
            update_batch_metadata = true;
        } else if end_batch {
            update_batch_metadata = true;
            tokio::time::sleep_until(
                self.last_batch_metadata_update_time.unwrap() + max_update_frequency,
            )
            .await;
        }

        if update_batch_metadata {
            let batch_metadata_path = self.reader.get_path_for_batch_metadata(first_version);
            self.writer
                .save_raw_file(
                    batch_metadata_path,
                    serde_json::to_vec(&batch_metadata).map_err(anyhow::Error::msg)?,
                )
                .await?;

            if end_batch {
                self.last_batch_metadata_update_time = None;
            } else {
                self.last_batch_metadata_update_time = Some(Instant::now());
            }

            if Instant::now() - self.last_metadata_update_time >= max_update_frequency {
                self.update_file_store_metadata(last_version + 1).await?;
                self.last_metadata_update_time = Instant::now();
            }
        }

        Ok(())
    }

    /// Updates the file store metadata.
    async fn update_file_store_metadata(&self, version: u64) -> Result<()> {
        let metadata = FileStoreMetadata {
            chain_id: self.chain_id,
            num_transactions_per_folder: NUM_TXNS_PER_FOLDER,
            version,
        };

        let raw_data = serde_json::to_vec(&metadata).map_err(anyhow::Error::msg)?;
        self.writer
            .save_raw_file(PathBuf::from(METADATA_FILE_NAME), raw_data)
            .await
    }
}
