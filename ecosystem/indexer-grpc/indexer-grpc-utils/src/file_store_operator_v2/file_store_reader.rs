// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compression_util::{FileEntry, StorageFormat},
    file_store_operator_v2::common::{
        BatchMetadata, FileStoreMetadata, IFileStore, METADATA_FILE_NAME,
    },
};
use anyhow::Result;
use aptos_protos::{transaction::v1::Transaction, util::timestamp::Timestamp};
use aptos_transaction_filter::{BooleanTransactionFilter, Filterable};
use prost::Message;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::sync::mpsc::Sender;
use tracing::{error, trace};

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

    /// Returns the file path for the given version. Requires the version to be the first version
    /// in the file.
    pub fn get_path_for_version(&self, version: u64, suffix: Option<u64>) -> PathBuf {
        let mut buf = self.get_folder_name(version);
        if let Some(suffix) = suffix {
            buf.push(format!("{version}_{suffix}"));
        } else {
            buf.push(format!("{version}"));
        }
        buf
    }

    /// Returns the metadata file path for the given version.
    pub fn get_path_for_batch_metadata(&self, version: u64) -> PathBuf {
        let folder = self.get_folder_name(version);
        let mut batch_metadata_path = PathBuf::new();
        batch_metadata_path.push(folder);
        batch_metadata_path.push(METADATA_FILE_NAME);
        batch_metadata_path
    }

    /// Returns transactions starting from the version, up to the end of the batch. Only
    /// `max_files` will be read if provided.
    pub async fn get_transaction_batch(
        &self,
        version: u64,
        retries: u8,
        max_files: Option<usize>,
        filter: Option<BooleanTransactionFilter>,
        ending_version: Option<u64>,
        tx: Sender<(Vec<Transaction>, usize, Timestamp, (u64, u64))>,
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
        for (i, file_metadata) in batch_metadata.files.iter().enumerate().rev() {
            let file_first_version = file_metadata.first_version;
            if file_first_version <= version {
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
            let current_version = batch_metadata.files[i].first_version;
            if let Some(ending_version) = ending_version {
                if current_version >= ending_version {
                    break;
                }
            }
            let transactions = self
                .get_transaction_file_at_version(current_version, batch_metadata.suffix, retries)
                .await;
            if let Ok(mut transactions) = transactions {
                let timestamp = transactions.last().unwrap().timestamp.unwrap();
                let num_to_skip = version.saturating_sub(current_version) as usize;
                if num_to_skip > 0 {
                    transactions = transactions.split_off(num_to_skip);
                }
                let mut processed_range = (
                    transactions.first().unwrap().version,
                    transactions.last().unwrap().version,
                );
                if let Some(ending_version) = ending_version {
                    transactions
                        .truncate(transactions.partition_point(|t| t.version < ending_version));
                    processed_range.1 = processed_range.1.min(ending_version - 1);
                }
                if let Some(ref filter) = filter {
                    transactions.retain(|t| filter.matches(t));
                }
                let size_bytes = transactions.iter().map(|t| t.encoded_len()).sum();
                trace!("Got {} transactions from file store to send, size: {size_bytes}, processed_range: [{}, {}]", transactions.len(), processed_range.0, processed_range.1);
                if tx
                    .send((transactions, size_bytes, timestamp, processed_range))
                    .await
                    .is_err()
                {
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

    /// Returns true iff the transaction at version can be served (i.e. less than file store
    /// version).
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
        suffix: Option<u64>,
        retries: u8,
    ) -> Result<Vec<Transaction>> {
        let mut retries = retries;
        let bytes = loop {
            let path = self.get_path_for_version(version, suffix);
            match self.reader.get_raw_file(path.clone()).await {
                Ok(bytes) => break bytes.unwrap_or_else(|| panic!("File should exist: {path:?}.")),
                Err(err) => {
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
