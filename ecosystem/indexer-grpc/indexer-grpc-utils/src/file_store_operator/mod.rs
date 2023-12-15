// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::constants::BLOB_STORAGE_SIZE;
use anyhow::Result;
use aptos_protos::transaction::v1::Transaction;
use serde::{Deserialize, Serialize};

pub mod gcs;
pub use gcs::*;
pub mod local;
pub use local::*;

pub const FILE_FOLDER_NAME: &str = "files";
const METADATA_FILE_NAME: &str = "metadata.json";
const FILE_STORE_UPDATE_FREQUENCY_SECS: u64 = 5;

#[inline]
pub fn generate_blob_name(starting_version: u64) -> String {
    format!("{}/{}.json", FILE_FOLDER_NAME, starting_version)
}

/// TransactionsFile is the file format for storing transactions.
/// It's a JSON file with name: ${starting_version}.json.
#[derive(Serialize, Deserialize)]
pub struct TransactionsFile {
    // The version of the first transaction in the file.
    // It must be the same as the starting_version in the file name.
    pub starting_version: u64,
    // Each transaction is a encoded string for Transaction protobuf.
    // Expected size of each vector is BLOB_STORAGE_SIZE, i.e., 1_000.
    pub transactions: Vec<String>,
}

/// FileStoreMetadata is the metadata for the file store.
/// It's a JSON file with name: metadata.json.
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct FileStoreMetadata {
    pub chain_id: u64,
    // The size of each file folder, BLOB_STORAGE_SIZE, i.e., 1_000.
    pub file_folder_size: usize,
    // The current version of the file store.
    pub version: u64,
}

impl FileStoreMetadata {
    pub fn new(chain_id: u64, version: u64) -> Self {
        Self {
            chain_id,
            file_folder_size: BLOB_STORAGE_SIZE,
            version,
        }
    }
}

#[async_trait::async_trait]
pub trait FileStoreOperator: Send + Sync {
    /// Bootstraps the file store operator. This is required before any other operations.
    async fn verify_storage_bucket_existence(&self);
    /// Gets the transactions files from the file store. version has to be a multiple of BLOB_STORAGE_SIZE.
    async fn get_transactions(&self, version: u64) -> Result<Vec<Transaction>>;
    /// Gets the metadata from the file store. Operator will panic if error happens when accessing the metadata file(except not found).
    async fn get_file_store_metadata(&self) -> Option<FileStoreMetadata>;
    /// If the file store is empty, the metadata will be created; otherwise, return the existing metadata.
    async fn update_file_store_metadata_with_timeout(
        &mut self,
        expected_chain_id: u64,
        version: u64,
    ) -> anyhow::Result<()>;
    /// Updates the file store metadata. This is only performed by the operator when new file transactions are uploaded.
    async fn update_file_store_metadata_internal(
        &mut self,
        chain_id: u64,
        version: u64,
    ) -> anyhow::Result<()>;
    /// Uploads the transactions to the file store. Single batch of 1000
    /// Returns start and end version of the batch, inclusive
    async fn upload_transaction_batch(
        &mut self,
        chain_id: u64,
        batch: Vec<Transaction>,
    ) -> anyhow::Result<(u64, u64)>;

    /// This is updated by the filestore worker whenever it updates the filestore metadata
    async fn get_latest_version(&self) -> Option<u64> {
        let metadata = self.get_file_store_metadata().await;
        metadata.map(|metadata| metadata.version)
    }
    /// Gets the raw transaction file; mainly for verification purpose.
    async fn get_raw_transactions(&self, version: u64) -> Result<TransactionsFile>;

    /// Get a clone for the file store operator.
    fn clone_box(&self) -> Box<dyn FileStoreOperator>;
}
