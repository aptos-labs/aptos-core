// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::storage_format::{FileEntry, FileEntryBuilder, FileStoreMetadata, StorageFormat};
use anyhow::Result;
use aptos_protos::transaction::v1::Transaction;
use serde::{Deserialize, Serialize};

pub mod gcs;
pub use gcs::*;
pub mod local;
pub use local::*;

pub const FILE_FOLDER_NAME: &str = "files";
const METADATA_FILE_NAME: &str = "metadata.json";
const VERIFICATION_FILE_NAME: &str = "verification.json";
const FILE_STORE_UPDATE_FREQUENCY_SECS: u64 = 5;

/// FileStoreMetadata is the metadata for the file store.
/// It's a JSON file with name: metadata.json.
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct VerificationMetadata {
    pub chain_id: u64,
    // The current version of the file store.
    pub next_version_to_verify: u64,
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
    async fn create_default_file_store_metadata_if_absent(
        &mut self,
        expected_chain_id: u64,
    ) -> anyhow::Result<FileStoreMetadata>;
    /// Updates the file store metadata. This is only performed by the operator when new file transactions are uploaded.
    async fn update_file_store_metadata(
        &mut self,
        chain_id: u64,
        version: u64,
        storage_format: StorageFormat,
    ) -> anyhow::Result<()>;
    /// Uploads the transactions to the file store. The transactions are grouped into batches of BLOB_STORAGE_SIZE.
    /// Updates the file store metadata after the upload.
    async fn upload_transactions(
        &mut self,
        chain_id: u64,
        transactions: Vec<Transaction>,
    ) -> anyhow::Result<()>;

    async fn get_starting_version(&self) -> Option<u64> {
        let metadata = self.get_file_store_metadata().await;
        metadata.map(|metadata| metadata.version)
    }

    /// Fetch the verification metadata file; this is used for bootstrap of the verifier.
    async fn get_or_create_verification_metadata(
        &self,
        chain_id: u64,
    ) -> Result<VerificationMetadata>;

    /// Updates the verification metadata file.
    async fn update_verification_metadata(
        &mut self,
        chain_id: u64,
        next_version_to_verify: u64,
    ) -> Result<()>;
}
