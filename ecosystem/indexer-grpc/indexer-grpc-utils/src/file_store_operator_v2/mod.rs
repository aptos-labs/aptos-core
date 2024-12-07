// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod gcs;
pub mod local;

use crate::{
    compression_util::{FileEntry, FileStoreMetadata, StorageFormat},
    counters::TRANSACTION_STORE_FETCH_RETRIES,
};
use anyhow::Result;
use aptos_protos::transaction::v1::Transaction;
use serde::{Deserialize, Serialize};
use std::{
    ops::Deref,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::sync::mpsc::Sender;

const METADATA_FILE_NAME: &str = "metadata.json";

#[derive(Serialize, Deserialize, Default)]
pub struct BatchMetadata {
    pub files: Vec<(u64, usize)>,
}

#[async_trait::async_trait]
pub trait FileStore: Sync + Send {
    /// The tag of the store, for logging.
    fn tag(&self) -> &str;

    async fn get_raw_file(&self, file_path: PathBuf) -> Result<Option<Vec<u8>>>;

    async fn save_raw_file(&self, file_path: PathBuf, data: Vec<u8>) -> Result<()>;
}

// TODO(grao): Split out the readonly part.
pub struct FileStoreOperatorV2 {
    chain_id: u64,
    file_store: Box<dyn FileStore>,
    num_transactions_per_folder: u64,
    cached_file_store_version: AtomicU64,
}

impl Deref for FileStoreOperatorV2 {
    type Target = Box<dyn FileStore>;
    fn deref(&self) -> &Box<dyn FileStore> {
        &self.file_store
    }
}

impl FileStoreOperatorV2 {
    pub fn new(
        chain_id: u64,
        file_store: Box<dyn FileStore>,
        num_transactions_per_folder: u64,
    ) -> Self {
        Self {
            chain_id,
            file_store,
            num_transactions_per_folder,
            cached_file_store_version: AtomicU64::new(0),
        }
    }

    pub async fn maybe_init_metadata(&self) -> Result<()> {
        match self.get_file_store_metadata().await {
            Some(_) => Ok(()),
            None => self.update_file_store_metadata(0).await,
        }
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
        tx: Sender<Vec<Transaction>>,
    ) {
        let batch_metadata = self.get_batch_metadata(version).await;
        if batch_metadata.is_none() {
            return;
        }

        let batch_metadata = batch_metadata.unwrap();

        let mut file_index = None;
        for (i, (file_store_version, _)) in batch_metadata.files.iter().enumerate().rev() {
            if *file_store_version <= version {
                file_index = Some(i);
                break;
            }
        }

        let file_index = file_index.expect("Must find file_index.");
        let mut end_file_index = batch_metadata.files.len();
        if let Some(max_files) = max_files {
            end_file_index = end_file_index.min(file_index.saturating_add(max_files));
        }

        for i in file_index..end_file_index {
            let current_version = batch_metadata.files[i].0;
            let transactions = self
                .get_transaction_file_at_version(current_version, retries)
                .await;
            if let Ok(transactions) = transactions {
                let num_to_skip = version.saturating_sub(current_version) as usize;
                let result = if num_to_skip > 0 {
                    transactions.into_iter().skip(num_to_skip).collect()
                } else {
                    transactions
                };
                if tx.send(result).await.is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Returns file store metadata, or None if not found.
    pub async fn get_file_store_metadata(&self) -> Option<FileStoreMetadata> {
        self.file_store
            .get_raw_file(PathBuf::from(METADATA_FILE_NAME))
            .await
            .expect("Failed to get file store metadata.")
            .map(|data| serde_json::from_slice(&data).expect("Metadata JSON is invalid."))
    }

    /// Updates the file store metadata.
    pub async fn update_file_store_metadata(&self, version: u64) -> Result<()> {
        let metadata =
            FileStoreMetadata::new(self.chain_id, version, StorageFormat::Lz4CompressedProto);

        let raw_data = serde_json::to_vec(&metadata).map_err(anyhow::Error::msg)?;
        self.file_store
            .save_raw_file(PathBuf::from(METADATA_FILE_NAME), raw_data)
            .await
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
            match self.file_store.get_raw_file(path.clone()).await {
                Ok(bytes) => break bytes.unwrap_or_else(|| panic!("File should exist: {path:?}.")),
                Err(err) => {
                    TRANSACTION_STORE_FETCH_RETRIES
                        .with_label_values(&[self.file_store.tag()])
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

    async fn get_batch_metadata(&self, version: u64) -> Option<BatchMetadata> {
        self.file_store
            .get_raw_file(self.get_path_for_batch_metadata(version))
            .await
            .expect("Failed to get batch metadata.")
            .map(|data| serde_json::from_slice(&data).expect("Batch metadata JSON is invalid."))
    }
}
