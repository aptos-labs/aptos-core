// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::compression_util::{
    FileEntry, FileStoreMetadata, StorageFormat, FILE_ENTRY_TRANSACTION_COUNT,
};
use anyhow::{Context, Result};
use velor_protos::transaction::v1::Transaction;

pub mod gcs;
pub use gcs::*;
pub mod local;
use crate::counters::TRANSACTION_STORE_FETCH_RETRIES;
pub use local::*;

const METADATA_FILE_NAME: &str = "metadata.json";
const FILE_STORE_UPDATE_FREQUENCY_SECS: u64 = 5;

#[async_trait::async_trait]
pub trait FileStoreOperator: Send + Sync {
    /// Bootstraps the file store operator. This is required before any other operations.
    async fn verify_storage_bucket_existence(&self);

    fn storage_format(&self) -> StorageFormat;

    /// The name of the store, for logging. Ex: "GCS", "Redis", etc
    fn store_name(&self) -> &str;

    /// Gets the transactions files from the file store. version has to be a multiple of BLOB_STORAGE_SIZE.
    async fn get_transactions(&self, version: u64, retries: u8) -> Result<Vec<Transaction>> {
        let (transactions, _, _) = self
            .get_transactions_with_durations(version, retries)
            .await?;
        Ok(transactions)
    }

    async fn get_raw_file(&self, version: u64) -> Result<Vec<u8>>;

    async fn get_raw_file_with_retries(&self, version: u64, retries: u8) -> Result<Vec<u8>> {
        let mut retries = retries;
        loop {
            match self.get_raw_file(version).await {
                Ok(bytes) => return Ok(bytes),
                Err(err) => {
                    TRANSACTION_STORE_FETCH_RETRIES
                        .with_label_values(&[self.store_name()])
                        .inc_by(1);

                    if retries == 0 {
                        return Err(err);
                    }
                    retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                },
            }
        }
    }

    async fn get_transactions_with_durations(
        &self,
        version: u64,
        retries: u8,
    ) -> Result<(Vec<Transaction>, f64, f64)> {
        let io_start_time = std::time::Instant::now();
        let bytes = self.get_raw_file_with_retries(version, retries).await?;
        let io_duration = io_start_time.elapsed().as_secs_f64();
        let decoding_start_time = std::time::Instant::now();
        let storage_format = self.storage_format();

        let transactions_in_storage = tokio::task::spawn_blocking(move || {
            FileEntry::new(bytes, storage_format).into_transactions_in_storage()
        })
        .await
        .context("Converting storage bytes to FileEntry transactions thread panicked")?;

        let decoding_duration = decoding_start_time.elapsed().as_secs_f64();
        Ok((
            transactions_in_storage
                .transactions
                .into_iter()
                .skip((version % FILE_ENTRY_TRANSACTION_COUNT) as usize)
                .collect(),
            io_duration,
            decoding_duration,
        ))
    }
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

    /// Get a clone for the file store operator.
    fn clone_box(&self) -> Box<dyn FileStoreOperator>;
}
