// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use cloud_storage::Object;
use serde::{Deserialize, Serialize};

// The maximum number of transactions to store in a single blob.
pub const BLOB_TRANSACTION_CHUNK_SIZE: u64 = 1_000;
// The name of the folder in the bucket where blobs are stored.
pub(crate) const BLOB_FOLDER_NAME: &str = "blobs";
// Metadata file name for file store.
const METADATA_FILE_NAME: &str = "metadata.json";
const JSON_FILE_TYPE: &str = "application/json";

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct FileStoreMetadata {
    /// The chain_id for the file store.
    pub chain_id: u64,

    /// Current version of the file store.
    pub version: u64,
}

pub async fn get_file_store_metadata(bucket_name: String) -> FileStoreMetadata {
    let metadata = Object::download(&bucket_name, METADATA_FILE_NAME)
        .await
        .expect("[indexer gcs] Failed to get file store metadata.");

    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.")
}

/// Uploads the metadata to the bucket. If the metadata is not updated, the indexer will be restarted.
pub async fn upload_file_store_metadata(bucket_name: String, metadata: FileStoreMetadata) {
    // If the metadata is not updated, the indexer will be restarted.
    Object::create(
        bucket_name.as_str(),
        serde_json::to_vec(&metadata).unwrap(),
        METADATA_FILE_NAME,
        JSON_FILE_TYPE,
    )
    .await
    .unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct TransactionsBlob {
    /// The version of the first transaction in the blob.
    pub starting_version: u64,
    /// The transactions in the blob.
    pub transactions: Vec<String>,
}

#[inline]
pub fn generate_blob_name(starting_version: u64) -> String {
    format!("{}/{}.json", BLOB_FOLDER_NAME, starting_version)
}

#[cfg(test)]
mod tests {
    #[test]
    fn verify_blob_naming() {
        assert_eq!(super::generate_blob_name(0), "blobs/0.json");
        assert_eq!(
            super::generate_blob_name(100_000_000),
            "blobs/100000000.json"
        );
        assert_eq!(
            super::generate_blob_name(1_000_000_000),
            "blobs/1000000000.json"
        );
        assert_eq!(
            super::generate_blob_name(10_000_000_000),
            "blobs/10000000000.json"
        );
        assert_eq!(
            super::generate_blob_name(u64::MAX),
            "blobs/18446744073709551615.json"
        );
    }
}
