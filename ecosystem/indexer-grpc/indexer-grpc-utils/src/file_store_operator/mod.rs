// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{constants::BLOB_STORAGE_SIZE, EncodedTransactionWithVersion};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod gcs;
pub use gcs::*;
pub mod local;
pub use local::*;

pub const FILE_FOLDER_NAME: &str = "files";
const METADATA_FILE_NAME: &str = "metadata.json";
const VERIFICATION_FILE_NAME: &str = "verification.json";
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

/// FileStoreMetadata is the metadata for the file store.
/// It's a JSON file with name: metadata.json.
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct VerificationMetadata {
    pub chain_id: u64,
    // The current version of the file store.
    pub next_version_to_verify: u64,
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
    async fn get_transactions(&self, version: u64) -> Result<Vec<String>>;
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
    ) -> anyhow::Result<()>;
    /// Uploads the transactions to the file store. The transactions are grouped into batches of BLOB_STORAGE_SIZE.
    /// Updates the file store metadata after the upload.
    async fn upload_transactions(
        &mut self,
        chain_id: u64,
        transactions: Vec<EncodedTransactionWithVersion>,
    ) -> anyhow::Result<()>;

    async fn get_starting_version(&self) -> Option<u64> {
        let metadata = self.get_file_store_metadata().await;
        metadata.map(|metadata| metadata.version)
    }
    /// Gets the raw transaction file; mainly for verification purpose.
    async fn get_raw_transactions(&self, version: u64) -> Result<TransactionsFile>;

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

pub(crate) fn build_transactions_file(
    transactions: Vec<EncodedTransactionWithVersion>,
) -> anyhow::Result<TransactionsFile> {
    let starting_version = transactions.first().unwrap().1;
    anyhow::ensure!(
        starting_version % BLOB_STORAGE_SIZE as u64 == 0,
        "Starting version has to be a multiple of BLOB_STORAGE_SIZE."
    );
    anyhow::ensure!(
        transactions.len() == BLOB_STORAGE_SIZE,
        "The number of transactions to upload has to be BLOB_STORAGE_SIZE."
    );
    anyhow::ensure!(
        transactions
            .iter()
            .enumerate()
            .any(|(ind, (_, version))| ind + starting_version as usize == *version as usize),
        "Transactions are in order."
    );

    Ok(TransactionsFile {
        starting_version,
        transactions: transactions.into_iter().map(|(tx, _)| tx).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_blob_naming() {
        assert_eq!(super::generate_blob_name(0), "files/0.json");
        assert_eq!(
            super::generate_blob_name(100_000_000),
            "files/100000000.json"
        );
        assert_eq!(
            super::generate_blob_name(1_000_000_000),
            "files/1000000000.json"
        );
        assert_eq!(
            super::generate_blob_name(10_000_000_000),
            "files/10000000000.json"
        );
        assert_eq!(
            super::generate_blob_name(u64::MAX),
            "files/18446744073709551615.json"
        );
    }

    #[test]
    fn verify_build_transactions_file() {
        // 1000 txns with starting version 0 succeeds.
        let mut transactions = vec![];
        for i in 0..BLOB_STORAGE_SIZE {
            transactions.push(("".to_string(), i as u64));
        }
        assert!(build_transactions_file(transactions).is_ok());

        // 1001 txns fails.
        let mut transactions = vec![];
        for i in 0..BLOB_STORAGE_SIZE + 1 {
            transactions.push(("".to_string(), i as u64));
        }
        assert!(build_transactions_file(transactions).is_err());
        // 1000 txns with starting version 1 fails.
        let mut transactions = vec![];
        for i in 1..BLOB_STORAGE_SIZE + 1 {
            transactions.push(("".to_string(), i as u64));
        }

        assert!(build_transactions_file(transactions).is_err());
    }
}
