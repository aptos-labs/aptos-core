// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod gcs;
pub mod local;

use crate::{
    compression_util::{FileEntry, StorageFormat},
    counters::TRANSACTION_STORE_FETCH_RETRIES,
};
use anyhow::Result;
use aptos_protos::transaction::v1::Transaction;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{sync::mpsc::Sender, time::Duration};
use tracing::{error, trace};

pub const METADATA_FILE_NAME: &str = "metadata.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct FileStoreMetadata {
    pub chain_id: u64,
    pub num_transactions_per_folder: u64,
    pub version: u64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct BatchMetadata {
    // "[first_version, last_version), size_bytes"
    pub files: Vec<(u64, u64, usize)>,
}

#[async_trait::async_trait]
pub trait IFileStoreReader: Sync + Send {
    /// The tag of the store, for logging.
    fn tag(&self) -> &str;

    async fn is_initialized(&self) -> bool;

    async fn get_raw_file(&self, file_path: PathBuf) -> Result<Option<Vec<u8>>>;
}

#[async_trait::async_trait]
pub trait IFileStoreWriter: Sync + Send {
    async fn save_raw_file(&self, file_path: PathBuf, data: Vec<u8>) -> Result<()>;

    fn max_update_frequency(&self) -> Duration;
}

#[async_trait::async_trait]
pub trait IFileStore: IFileStoreReader + IFileStoreWriter {}

impl<T> IFileStore for T where T: IFileStoreReader + IFileStoreWriter {}

pub struct FileStoreReader {
    chain_id: u64,
    // TODO(grao): Change to IFileStoreReader when the trait_upcasting feature is in stable Rust.
    reader: Arc<dyn IFileStore>,
    num_transactions_per_folder: u64,
    cached_file_store_version: AtomicU64,
}

impl FileStoreReader {
    pub async fn new(chain_id: u64, reader: Arc<dyn IFileStore>) -> Self {
        assert!(reader.is_initialized().await);

        let mut myself = Self {
            chain_id,
            reader,
            num_transactions_per_folder: 0,
            cached_file_store_version: AtomicU64::new(0),
        };

        let metadata = Self::get_file_store_metadata(&myself)
            .await
            .expect("Failed to fetch num_transactions_per_folder.");

        assert!(chain_id == metadata.chain_id);

        myself.num_transactions_per_folder = metadata.num_transactions_per_folder;

        myself
    }

    pub fn get_path_for_version(&self, version: u64) -> PathBuf {
        let mut buf = self.get_folder_name(version);
        buf.push(format!("{}", version));
        buf
    }

    pub fn get_path_for_batch_metadata(&self, version: u64) -> PathBuf {
        let folder = self.get_folder_name(version);
        let mut batch_metadata_path = PathBuf::new();
        batch_metadata_path.push(folder);
        batch_metadata_path.push(METADATA_FILE_NAME);
        batch_metadata_path
    }

    pub async fn get_transaction_batch(
        &self,
        version: u64,
        retries: u8,
        max_files: Option<usize>,
        tx: Sender<(Vec<Transaction>, usize)>,
    ) {
        trace!(
            "Getting transactions from file store, version: {version}, max_files: {max_files:?}."
        );
        let batch_metadata = self.get_batch_metadata(version).await;
        if batch_metadata.is_none() {
            // TODO(grao): This is unexpected, should only happen when data is corrupted. Consider
            // make it panic!.
            error!("Failed to get the batch metadata, unable to serve the request.");
            return;
        }

        let batch_metadata = batch_metadata.unwrap();

        let mut file_index = None;
        for (i, (file_store_version, _, _)) in batch_metadata.files.iter().enumerate().rev() {
            if *file_store_version <= version {
                file_index = Some(i);
                break;
            }
        }

        let file_index =
            file_index.unwrap_or_else(|| panic!("Must find file_index for version: {version}."));
        let mut end_file_index = batch_metadata.files.len();
        if let Some(max_files) = max_files {
            end_file_index = end_file_index.min(file_index.saturating_add(max_files));
        }

        for i in file_index..end_file_index {
            let current_version = batch_metadata.files[i].0;
            let mut size_bytes = batch_metadata.files[i].2;
            let transactions = self
                .get_transaction_file_at_version(current_version, retries)
                .await;
            if let Ok(mut transactions) = transactions {
                let num_to_skip = version.saturating_sub(current_version) as usize;
                let result = if num_to_skip > 0 {
                    let transactions_to_return = transactions.split_off(num_to_skip);
                    for transaction in transactions {
                        size_bytes -= transaction.encoded_len();
                    }
                    (transactions_to_return, size_bytes)
                } else {
                    (transactions, size_bytes)
                };
                trace!("Got {} transactions from file store to send, size: {size_bytes}, first_version: {:?}", result.0.len(), result.0.first().map(|t| t.version));
                if tx.send(result).await.is_err() {
                    break;
                }
            } else {
                error!("Got error from file store: {:?}.", transactions);
                break;
            }
        }
    }

    /// Returns file store metadata, or None if not found.
    pub async fn get_file_store_metadata(&self) -> Option<FileStoreMetadata> {
        self.reader
            .get_raw_file(PathBuf::from(METADATA_FILE_NAME))
            .await
            .expect("Failed to get file store metadata.")
            .map(|data| serde_json::from_slice(&data).expect("Metadata JSON is invalid."))
    }

    /// Returns the batch matadata for the batch that includes the provided version, or None if not
    /// found.
    pub async fn get_batch_metadata(&self, version: u64) -> Option<BatchMetadata> {
        self.reader
            .get_raw_file(self.get_path_for_batch_metadata(version))
            .await
            .expect("Failed to get batch metadata.")
            .map(|data| serde_json::from_slice(&data).expect("Batch metadata JSON is invalid."))
    }

    /// Returns the latest_version (next_version) that is going to be process by file store, or
    /// None if the metadata file doesn't exist.
    pub async fn get_latest_version(&self) -> Option<u64> {
        let metadata = self.get_file_store_metadata().await;
        let latest_version = metadata.map(|metadata| {
            if metadata.chain_id != self.chain_id {
                panic!("Wrong chain_id.");
            }
            metadata.version
        });

        if let Some(version) = latest_version {
            self.cached_file_store_version
                .fetch_max(version, Ordering::SeqCst);
        }

        latest_version
    }

    pub async fn can_serve(&self, version: u64) -> bool {
        if self.cached_file_store_version.load(Ordering::SeqCst) > version {
            return true;
        }

        self.get_latest_version().await.unwrap() > version
    }

    fn get_folder_name(&self, version: u64) -> PathBuf {
        let mut buf = PathBuf::new();
        buf.push(format!("{}", version / self.num_transactions_per_folder));
        buf
    }

    async fn get_transaction_file_at_version(
        &self,
        version: u64,
        retries: u8,
    ) -> Result<Vec<Transaction>> {
        let mut retries = retries;
        let bytes = loop {
            let path = self.get_path_for_version(version);
            match self.reader.get_raw_file(path.clone()).await {
                Ok(bytes) => break bytes.unwrap_or_else(|| panic!("File should exist: {path:?}.")),
                Err(err) => {
                    TRANSACTION_STORE_FETCH_RETRIES
                        .with_label_values(&[self.reader.tag()])
                        .inc_by(1);

                    if retries == 0 {
                        return Err(err);
                    }
                    retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                },
            }
        };

        let transactions_in_storage = tokio::task::spawn_blocking(move || {
            FileEntry::new(bytes, StorageFormat::Lz4CompressedProto).into_transactions_in_storage()
        })
        .await?;

        Ok(transactions_in_storage.transactions)
    }
}

pub struct FileStoreOperatorV2 {
    max_size_per_file: usize,
    num_txns_per_folder: u64,

    buffer: Vec<Transaction>,
    buffer_size: usize,
    buffer_batch_metadata: BatchMetadata,
    version: u64,
}

impl FileStoreOperatorV2 {
    pub async fn new(
        max_size_per_file: usize,
        num_txns_per_folder: u64,
        version: u64,
        batch_metadata: BatchMetadata,
    ) -> Self {
        Self {
            max_size_per_file,
            num_txns_per_folder,
            buffer: vec![],
            buffer_size: 0,
            buffer_batch_metadata: batch_metadata,
            version,
        }
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub async fn buffer_and_maybe_dump_transactions_to_file(
        &mut self,
        transaction: Transaction,
        tx: Sender<(Vec<Transaction>, BatchMetadata, bool)>,
    ) -> Result<()> {
        let end_batch = (transaction.version + 1) % self.num_txns_per_folder == 0;
        let size = transaction.encoded_len();
        self.buffer.push(transaction);
        self.buffer_size += size;
        self.version += 1;
        if self.buffer_size >= self.max_size_per_file || end_batch {
            self.dump_transactions_to_file(end_batch, tx).await?;
        }

        Ok(())
    }

    async fn dump_transactions_to_file(
        &mut self,
        end_batch: bool,
        tx: Sender<(Vec<Transaction>, BatchMetadata, bool)>,
    ) -> Result<()> {
        let transactions = std::mem::take(&mut self.buffer);
        let first_version = transactions.first().unwrap().version;
        self.buffer_batch_metadata.files.push((
            first_version,
            first_version + transactions.len() as u64,
            self.buffer_size,
        ));
        self.buffer_size = 0;

        tx.send((transactions, self.buffer_batch_metadata.clone(), end_batch))
            .await
            .map_err(anyhow::Error::msg)?;

        if end_batch {
            self.buffer_batch_metadata = BatchMetadata::default();
        }

        Ok(())
    }
}
