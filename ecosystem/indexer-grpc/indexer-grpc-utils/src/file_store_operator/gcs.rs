// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{constants::BLOB_STORAGE_SIZE, file_store_operator::*, EncodedTransactionWithVersion};
use anyhow::bail;
use cloud_storage::{Bucket, Object};
use itertools::Itertools;
use std::env;

const JSON_FILE_TYPE: &str = "application/json";
// The environment variable to set the service account path.
const SERVICE_ACCOUNT_ENV_VAR: &str = "SERVICE_ACCOUNT";
const FILE_STORE_METADATA_TIMEOUT_MILLIS: u128 = 200;

#[derive(Clone)]
pub struct GcsFileStoreOperator {
    bucket_name: String,
    file_store_metadata_last_updated: std::time::Instant,
}

impl GcsFileStoreOperator {
    pub fn new(bucket_name: String, service_account_path: String) -> Self {
        env::set_var(SERVICE_ACCOUNT_ENV_VAR, service_account_path);
        Self {
            bucket_name,
            file_store_metadata_last_updated: std::time::Instant::now(),
        }
    }
}

#[async_trait::async_trait]
impl FileStoreOperator for GcsFileStoreOperator {
    /// Bootstraps the file store operator. This is required before any other operations.
    async fn verify_storage_bucket_existence(&self) {
        tracing::info!(
            bucket_name = self.bucket_name,
            "Before file store operator starts, verify the bucket exists."
        );
        // Verifies the bucket exists.
        Bucket::read(&self.bucket_name)
            .await
            .expect("Failed to read bucket.");
    }

    /// Gets the transactions files from the file store. version has to be a multiple of BLOB_STORAGE_SIZE.
    async fn get_transactions(&self, version: u64) -> anyhow::Result<Vec<String>> {
        let batch_start_version = version / BLOB_STORAGE_SIZE as u64 * BLOB_STORAGE_SIZE as u64;
        let current_file_name = generate_blob_name(batch_start_version);
        match Object::download(&self.bucket_name, current_file_name.as_str()).await {
            Ok(file) => {
                let file: TransactionsFile =
                    serde_json::from_slice(&file).map_err(|e| anyhow::anyhow!(e.to_string()))?;
                Ok(file
                    .transactions
                    .into_iter()
                    .skip((version % BLOB_STORAGE_SIZE as u64) as usize)
                    .collect())
            },
            Err(cloud_storage::Error::Other(err)) => {
                if err.contains("No such object: ") {
                    anyhow::bail!("[Indexer File] Transactions file not found. Gap might happen between cache and file store. {}", err)
                } else {
                    anyhow::bail!(
                        "[Indexer File] Error happens when transaction file. {}",
                        err
                    );
                }
            },
            Err(err) => {
                anyhow::bail!(
                    "[Indexer File] Error happens when transaction file. {}",
                    err
                );
            },
        }
    }

    /// Gets the raw transactions file from the file store. Mainly for verification purpose.
    async fn get_raw_transactions(&self, version: u64) -> anyhow::Result<TransactionsFile> {
        let batch_start_version = version / BLOB_STORAGE_SIZE as u64 * BLOB_STORAGE_SIZE as u64;
        let current_file_name = generate_blob_name(batch_start_version);
        let bytes = Object::download(&self.bucket_name, current_file_name.as_str()).await?;
        serde_json::from_slice(&bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize transactions file: {}", e))
    }

    /// Gets the metadata from the file store. Operator will panic if error happens when accessing the metadata file(except not found).
    async fn get_file_store_metadata(&self) -> Option<FileStoreMetadata> {
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
    async fn update_file_store_metadata_with_timeout(
        &mut self,
        expected_chain_id: u64,
        version: u64,
    ) -> anyhow::Result<()> {
        if let Some(metadata) = self.get_file_store_metadata().await {
            anyhow::ensure!(metadata.chain_id == expected_chain_id, "Chain ID mismatch.");
        }
        if self.file_store_metadata_last_updated.elapsed().as_millis()
            < FILE_STORE_METADATA_TIMEOUT_MILLIS
        {
            bail!("File store metadata is updated too frequently.")
        }
        self.update_file_store_metadata_internal(expected_chain_id, version)
            .await?;
        Ok(())
    }

    /// Updates the file store metadata. This is only performed by the operator when new file transactions are uploaded.
    async fn update_file_store_metadata_internal(
        &mut self,
        chain_id: u64,
        version: u64,
    ) -> anyhow::Result<()> {
        let metadata = FileStoreMetadata::new(chain_id, version);
        // If the metadata is not updated, the indexer will be restarted.
        Object::create(
            self.bucket_name.as_str(),
            serde_json::to_vec(&metadata).unwrap(),
            METADATA_FILE_NAME,
            JSON_FILE_TYPE,
        )
        .await?;
        self.file_store_metadata_last_updated = std::time::Instant::now();
        Ok(())
    }

    /// Uploads the transactions to the file store. The transactions are grouped into batches of BLOB_STORAGE_SIZE.
    /// Updates the file store metadata after the upload.
    async fn upload_transaction_batch(
        &mut self,
        _chain_id: u64,
        transactions: Vec<EncodedTransactionWithVersion>,
    ) -> anyhow::Result<(u64, u64)> {
        let start_version = transactions.first().unwrap().1;
        let end_version = transactions.last().unwrap().1;
        let batch_size = transactions.len();
        anyhow::ensure!(
            start_version % BLOB_STORAGE_SIZE as u64 == 0,
            "Starting version has to be a multiple of BLOB_STORAGE_SIZE."
        );
        anyhow::ensure!(
            batch_size % BLOB_STORAGE_SIZE == 0,
            "The number of transactions to upload has to be multiplier of BLOB_STORAGE_SIZE."
        );

        let bucket_name = self.bucket_name.clone();
        let current_batch = transactions.iter().cloned().collect_vec();
        let transactions_file = build_transactions_file(current_batch).unwrap();
        Object::create(
            bucket_name.clone().as_str(),
            serde_json::to_vec(&transactions_file).unwrap(),
            generate_blob_name(transactions_file.starting_version).as_str(),
            JSON_FILE_TYPE,
        )
        .await?;
        Ok((start_version, end_version))
    }

    fn clone_box(&self) -> Box<dyn FileStoreOperator> {
        Box::new(self.clone())
    }
}
