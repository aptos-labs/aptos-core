// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_manager::DataManager,
    metrics::{FILE_STORE_UPLOADED_BYTES, FILE_STORE_VERSION, TIMER},
};
use anyhow::Result;
use aptos_indexer_grpc_utils::{
    compression_util::{FileEntry, StorageFormat},
    config::IndexerGrpcFileStoreConfig,
    file_store_operator_v2::{
        common::{BatchMetadata, FileStoreMetadata, IFileStore, METADATA_FILE_NAME},
        file_store_operator::FileStoreOperatorV2,
        file_store_reader::FileStoreReader,
    },
};
use aptos_protos::transaction::v1::Transaction;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc::channel, oneshot::Sender},
    time::Instant,
};
use tracing::info;

const NUM_TXNS_PER_FOLDER: u64 = 100000;
const MAX_SIZE_PER_FILE: usize = 50 * (1 << 20);
const MAX_NUM_FOLDERS_TO_CHECK_FOR_RECOVERY: usize = 5;
const MIN_UPDATE_FREQUENCY: Duration = Duration::from_secs(10);

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
        // NOTE: We cannot change NUM_TXNS_PER_FOLDER without backfilling the data, put a check
        // here to make sure we don't change it accidentally.
        assert_eq!(
            reader
                .get_file_store_metadata()
                .await
                .unwrap()
                .num_transactions_per_folder,
            NUM_TXNS_PER_FOLDER
        );

        Ok(Self {
            chain_id,
            reader,
            writer: file_store,
            last_batch_metadata_update_time: None,
            last_metadata_update_time: Instant::now(),
        })
    }

    /// Recovers the batch metadata in memory buffer for the unfinished batch from file store.
    async fn recover(&self) -> Result<(u64, BatchMetadata)> {
        let _timer = TIMER.with_label_values(&["recover"]).start_timer();

        let mut version = self
            .reader
            .get_latest_version()
            .await
            .expect("Latest version must exist.");
        info!("Starting recovering process, current version in storage: {version}.");
        let mut num_folders_checked = 0;
        let mut buffered_batch_metadata_to_recover = BatchMetadata::default();
        while let Some(batch_metadata) = self.reader.get_batch_metadata(version).await {
            let batch_last_version = batch_metadata.files.last().unwrap().last_version;
            version = batch_last_version;
            if version % NUM_TXNS_PER_FOLDER != 0 {
                buffered_batch_metadata_to_recover = batch_metadata;
                break;
            }
            num_folders_checked += 1;
            if num_folders_checked >= MAX_NUM_FOLDERS_TO_CHECK_FOR_RECOVERY {
                panic!(
                    "File store metadata is way behind batch metadata, data might be corrupted."
                );
            }
        }

        self.update_file_store_metadata(version).await?;

        info!("Finished recovering process, recovered at version: {version}.");

        Ok((version, buffered_batch_metadata_to_recover))
    }

    pub(crate) async fn start(
        &mut self,
        data_manager: Arc<DataManager>,
        recover_tx: Sender<()>,
    ) -> Result<()> {
        let (version, batch_metadata) = self.recover().await?;

        let mut file_store_operator = FileStoreOperatorV2::new(
            MAX_SIZE_PER_FILE,
            NUM_TXNS_PER_FOLDER,
            version,
            batch_metadata,
        )
        .await;

        recover_tx.send(()).expect("Receiver should exist.");

        tokio_scoped::scope(|s| {
            let (tx, mut rx) = channel::<(_, BatchMetadata, _)>(5);
            s.spawn(async move {
                while let Some((transactions, batch_metadata, end_batch)) = rx.recv().await {
                    let bytes_to_upload = batch_metadata.files.last().unwrap().size_bytes as u64;
                    self.do_upload(transactions, batch_metadata, end_batch)
                        .await
                        .unwrap();
                    FILE_STORE_UPLOADED_BYTES.inc_by(bytes_to_upload);
                }
            });
            s.spawn(async move {
                loop {
                    let _timer = TIMER
                        .with_label_values(&["file_store_uploader_main_loop"])
                        .start_timer();
                    let next_version = file_store_operator.version();
                    let transactions = {
                        let _timer = TIMER
                            .with_label_values(&["get_transactions_from_cache"])
                            .start_timer();
                        data_manager
                            .get_transactions_from_cache(
                                next_version,
                                MAX_SIZE_PER_FILE,
                                /*update_file_store_version=*/ true,
                            )
                            .await
                    };
                    let len = transactions.len();
                    for transaction in transactions {
                        file_store_operator
                            .buffer_and_maybe_dump_transactions_to_file(transaction, tx.clone())
                            .await
                            .unwrap();
                    }
                    if len == 0 {
                        info!("No transaction was returned from cache, requested version: {next_version}.");
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
        let _timer = TIMER.with_label_values(&["do_upload"]).start_timer();

        let first_version = transactions.first().unwrap().version;
        let last_version = transactions.last().unwrap().version;
        let data_file = {
            let _timer = TIMER
                .with_label_values(&["do_upload__prepare_file"])
                .start_timer();
            FileEntry::from_transactions(transactions, StorageFormat::Lz4CompressedProto)
        };
        let path = self.reader.get_path_for_version(first_version);

        info!("Dumping transactions [{first_version}, {last_version}] to file {path:?}.");

        {
            let _timer = TIMER
                .with_label_values(&["do_upload__save_file"])
                .start_timer();
            self.writer
                .save_raw_file(path, data_file.into_inner())
                .await?;
        }

        let mut update_batch_metadata = false;
        let max_update_frequency = self.writer.max_update_frequency();
        if self.last_batch_metadata_update_time.is_none()
            || Instant::now() - self.last_batch_metadata_update_time.unwrap()
                >= MIN_UPDATE_FREQUENCY
        {
            update_batch_metadata = true;
        } else if end_batch {
            update_batch_metadata = true;
            tokio::time::sleep_until(
                self.last_batch_metadata_update_time.unwrap() + max_update_frequency,
            )
            .await;
        }

        if !update_batch_metadata {
            return Ok(());
        }

        let batch_metadata_path = self.reader.get_path_for_batch_metadata(first_version);
        {
            let _timer = TIMER
                .with_label_values(&["do_upload__update_batch_metadata"])
                .start_timer();
            self.writer
                .save_raw_file(
                    batch_metadata_path,
                    serde_json::to_vec(&batch_metadata).map_err(anyhow::Error::msg)?,
                )
                .await?;
        }

        if end_batch {
            self.last_batch_metadata_update_time = None;
        } else {
            self.last_batch_metadata_update_time = Some(Instant::now());
        }

        if Instant::now() - self.last_metadata_update_time >= max_update_frequency {
            let _timer = TIMER
                .with_label_values(&["do_upload__update_metadata"])
                .start_timer();
            self.update_file_store_metadata(last_version + 1).await?;
            self.last_metadata_update_time = Instant::now();
        }

        Ok(())
    }

    /// Updates the file store metadata.
    async fn update_file_store_metadata(&self, version: u64) -> Result<()> {
        FILE_STORE_VERSION.set(version as i64);
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
