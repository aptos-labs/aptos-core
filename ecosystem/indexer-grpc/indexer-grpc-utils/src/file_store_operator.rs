// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::constants::BLOB_STORAGE_SIZE;
use cloud_storage::{Bucket, Object};
use itertools::{any, Itertools};
use serde::{Deserialize, Serialize};

const FILE_FOLDER_NAME: &str = "files";
const METADATA_FILE_NAME: &str = "metadata.json";
const JSON_FILE_TYPE: &str = "application/json";

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
    /// Each transaction is a encoded string for Transaction protobuf.
    /// Expected size of each vector is BLOB_STORAGE_SIZE, i.e., 1_000.
    pub transactions: Vec<String>,
}

/// FileStoreMetadata is the metadata for the file store.
/// It's a JSON file with name: metadata.json.
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct FileStoreMetadata {
    pub chain_id: u64,
    /// The size of each file folder, BLOB_STORAGE_SIZE, i.e., 1_000.
    pub file_folder_size: usize,
    /// The current version of the file store.
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

/// The file store operator is stateless and the state should be handled by the caller, e.g., current version.
/// The only state it maintains is the latest metadata update timestamp.
/// The file store operator is not thread safe and is intended to be used in a single thread.
pub struct FileStoreOperator {
    bucket_name: String,
    /// The timestamp of the latest metadata update; this is to avoid too frequent metadata update.
    latest_metadata_update_timestamp: std::time::Instant,
}

impl FileStoreOperator {
    pub fn new(bucket_name: String) -> Self {
        Self {
            bucket_name,
            latest_metadata_update_timestamp: std::time::Instant::now(),
        }
    }

    /// Bootstraps the file store operator. This is required before any other operations.
    pub async fn bootstrap(&self) {
        // Verifies the bucket exists.
        Bucket::read(&self.bucket_name)
            .await
            .expect("Failed to read bucket.");
    }

    /// Gets the transactions files from the file store. version has to be a multiple of BLOB_STORAGE_SIZE.
    pub async fn get_transactions_file(&self, version: u64) -> anyhow::Result<TransactionsFile> {
        anyhow::ensure!(
            version % BLOB_STORAGE_SIZE as u64 == 0,
            "Version has to be a multiple of BLOB_STORAGE_SIZE."
        );

        let current_file_name = generate_blob_name(version);
        match Object::download(&self.bucket_name, current_file_name.as_str()).await {
            Ok(file) => {
                let file: TransactionsFile =
                    serde_json::from_slice(&file).expect("Expected file to be valid JSON.");
                Ok(file)
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }

    /// Gets the metadata from the file store. Operator will panic if error happens when accessing the metadata file(except not found).
    pub async fn get_file_store_metadata(&self) -> Option<FileStoreMetadata> {
        match Object::download(&self.bucket_name, METADATA_FILE_NAME).await {
            Ok(metadata) => {
                let metadata: FileStoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                Some(metadata)
            },
            Err(cloud_storage::Error::Other(err)) => {
                if err.contains("No such object: ") {
                    // Metadata is not found.
                    None
                } else {
                    panic!(
                        "[Indexer File] Error happens when accessing metadata file. {}",
                        err
                    );
                }
            },
            Err(e) => {
                panic!(
                    "[Indexer File] Error happens when accessing metadata file. {}",
                    e
                );
            },
        }
    }

    /// If the file store is empty, the metadata will be created; otherwise, return the existing metadata.
    pub async fn create_default_file_store_metadata_if_absent(
        &mut self,
        expected_chain_id: u64,
    ) -> anyhow::Result<FileStoreMetadata> {
        match Object::download(&self.bucket_name, METADATA_FILE_NAME).await {
            Ok(metadata) => {
                let metadata: FileStoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                anyhow::ensure!(metadata.chain_id == expected_chain_id, "Chain ID mismatch.");
                Ok(metadata)
            },
            Err(cloud_storage::Error::Other(err)) => {
                let is_file_missing = err.contains("No such object: ");
                if is_file_missing {
                    // If the metadata is not found, it means the file store is empty.
                    self.update_file_store_metadata(expected_chain_id, 0)
                        .await
                        .expect("[Indexer File] Update metadata failed.");
                    Ok(FileStoreMetadata::new(expected_chain_id, 0))
                } else {
                    // If not in write mode, the metadata must exist.
                    Err(anyhow::Error::msg(format!(
                        "Metadata not found or file store operator is not in write mode. {}",
                        err
                    )))
                }
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }

    /// Updates the file store metadata. This is only performed by the operator when new file transactions are uploaded.
    async fn update_file_store_metadata(
        &mut self,
        chain_id: u64,
        version: u64,
    ) -> anyhow::Result<()> {
        if (std::time::Instant::now() - self.latest_metadata_update_timestamp).as_secs() < 5 {
            return Ok(());
        }

        let metadata = FileStoreMetadata::new(chain_id, version);
        // If the metadata is not updated, the indexer will be restarted.
        match Object::create(
            self.bucket_name.as_str(),
            serde_json::to_vec(&metadata).unwrap(),
            METADATA_FILE_NAME,
            JSON_FILE_TYPE,
        )
        .await
        {
            Ok(_) => {
                self.latest_metadata_update_timestamp = std::time::Instant::now();
                Ok(())
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }

    /// Uploads the transactions to the file store. The transactions are grouped into batches of BLOB_STORAGE_SIZE.
    /// Updates the file store metadata after the upload.
    pub async fn upload_transactions(
        &mut self,
        chain_id: u64,
        starting_version: u64,
        transactions: Vec<String>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            starting_version % BLOB_STORAGE_SIZE as u64 == 0,
            "Starting version has to be a multiple of BLOB_STORAGE_SIZE."
        );

        let mut tasks = vec![];
        let mut current_version = starting_version;

        // Split the transactions into batches of BLOB_STORAGE_SIZE.
        for i in transactions.chunks(BLOB_STORAGE_SIZE) {
            let bucket_name = self.bucket_name.clone();
            let current_batch = i.iter().cloned().collect_vec();
            let task = tokio::spawn(async move {
                let batch_version = current_version;
                match Object::create(
                    bucket_name.clone().as_str(),
                    serde_json::to_vec(&TransactionsFile {
                        starting_version: batch_version,
                        transactions: current_batch,
                    })
                    .unwrap(),
                    generate_blob_name(batch_version).as_str(),
                    JSON_FILE_TYPE,
                )
                .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(anyhow::Error::from(err)),
                }
            });
            current_version += BLOB_STORAGE_SIZE as u64;
            tasks.push(task);
        }
        let results = match futures::future::try_join_all(tasks).await {
            Ok(res) => res,
            Err(err) => panic!("Error processing transaction batches: {:?}", err),
        };
        // If any uploading fails, retry.
        if any(results, |x| x.is_err()) {
            anyhow::bail!("Uploading transactions failed.");
        }

        self.update_file_store_metadata(chain_id, current_version)
            .await
    }
}

#[cfg(test)]
mod tests {
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
}
