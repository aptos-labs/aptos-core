// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    constants::BLOB_STORAGE_SIZE,
    file_store_operator::*,
    storage_format::{FileEntryKey, StorageFormat},
};
use aptos_protos::indexer::v1::TransactionsInStorage;
use cloud_storage::{Bucket, Object};
use itertools::{any, Itertools};
use std::env;

const JSON_FILE_TYPE: &str = "application/json";
const BINARY_FILE_TYPE: &str = "application/octet-stream";
// The environment variable to set the service account path.
const SERVICE_ACCOUNT_ENV_VAR: &str = "SERVICE_ACCOUNT";
const SERVICE_TYPE: &str = "file_worker";

pub struct GcsFileStoreOperator {
    bucket_name: String,
    /// The timestamp of the latest metadata update; this is to avoid too frequent metadata update.
    latest_metadata_update_timestamp: Option<std::time::Instant>,

    /// The timestamp of the latest verification metadata update; this is to avoid too frequent metadata update.
    latest_verification_metadata_update_timestamp: Option<std::time::Instant>,
    storage_format: StorageFormat,
}

impl GcsFileStoreOperator {
    pub fn new(
        bucket_name: String,
        service_account_path: String,
        storage_format: StorageFormat,
    ) -> Self {
        env::set_var(SERVICE_ACCOUNT_ENV_VAR, service_account_path);
        Self {
            bucket_name,
            latest_metadata_update_timestamp: None,
            latest_verification_metadata_update_timestamp: None,
            storage_format,
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
    async fn get_transactions(&self, version: u64) -> anyhow::Result<Vec<Transaction>> {
        let start_time = std::time::Instant::now();
        let file_entry_key = FileEntryKey::new(version, self.storage_format);
        let key_name = file_entry_key.to_string();
        let downloaded_file = Object::download(&self.bucket_name, key_name.as_str()).await;
        tracing::info!(
            start_version = version,
            duration_in_secs = start_time.elapsed().as_secs_f64(),
            service_type = SERVICE_TYPE,
            "{}",
            "Fetched data from GCS."
        );
        match downloaded_file {
            Ok(file) => {
                let file_entry: FileEntry = FileEntry::from_bytes(file, self.storage_format);
                let transactions_in_storage: TransactionsInStorage = file_entry.try_into()?;
                tracing::info!(
                    start_version = version,
                    duration_in_secs = start_time.elapsed().as_secs_f64(),
                    service_type = SERVICE_TYPE,
                    "{}",
                    "Deserialized data from GCS."
                );
                Ok(transactions_in_storage.transactions)
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
    async fn create_default_file_store_metadata_if_absent(
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
                    self.update_file_store_metadata(expected_chain_id, 0, self.storage_format)
                        .await
                        .expect("[Indexer File] Update metadata failed.");
                    Ok(FileStoreMetadata::new(
                        expected_chain_id,
                        0,
                        self.storage_format,
                    ))
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
        storage_format: StorageFormat,
    ) -> anyhow::Result<()> {
        let metadata = FileStoreMetadata::new(chain_id, version, storage_format);
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
                self.latest_metadata_update_timestamp = Some(std::time::Instant::now());
                Ok(())
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }

    /// Updates the verification metadata file.
    async fn update_verification_metadata(
        &mut self,
        chain_id: u64,
        next_version_to_verify: u64,
    ) -> Result<()> {
        let verification_metadata = VerificationMetadata {
            chain_id,
            next_version_to_verify,
        };
        let time_now = std::time::Instant::now();
        if let Some(last_update_time) = self.latest_verification_metadata_update_timestamp {
            if time_now.duration_since(last_update_time) < std::time::Duration::from_secs(20) {
                return Ok(());
            }
        }
        // If the metadata is not updated, the indexer will be restarted.
        match Object::create(
            self.bucket_name.as_str(),
            serde_json::to_vec(&verification_metadata).unwrap(),
            VERIFICATION_FILE_NAME,
            JSON_FILE_TYPE,
        )
        .await
        {
            Ok(_) => {
                self.latest_verification_metadata_update_timestamp =
                    Some(std::time::Instant::now());
                Ok(())
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }

    /// Uploads the transactions to the file store. The transactions are grouped into batches of BLOB_STORAGE_SIZE.
    /// Updates the file store metadata after the upload.
    async fn upload_transactions(
        &mut self,
        chain_id: u64,
        transactions: Vec<Transaction>,
    ) -> anyhow::Result<()> {
        let start_version = transactions.first().unwrap().version;
        let batch_size = transactions.len();
        anyhow::ensure!(
            start_version % BLOB_STORAGE_SIZE as u64 == 0,
            "Starting version has to be a multiple of BLOB_STORAGE_SIZE."
        );
        anyhow::ensure!(
            batch_size % BLOB_STORAGE_SIZE == 0,
            "The number of transactions to upload has to be multiplier of BLOB_STORAGE_SIZE."
        );

        let files_to_upload = transactions
            .into_iter()
            .chunks(BLOB_STORAGE_SIZE)
            .into_iter()
            .map(|i| {
                let transactions = i.collect_vec();
                let file_key =
                    FileEntryKey::new(transactions.first().unwrap().version, self.storage_format);
                let file_key_string = file_key.to_string();

                let file_entry_builder = FileEntryBuilder::new(transactions, self.storage_format);
                let file_entry: FileEntry = file_entry_builder
                    .try_into()
                    .expect("Failed to serialize file entry");
                (file_key_string, file_entry.into_inner())
            })
            .collect_vec();

        let tasks = files_to_upload
            .into_iter()
            .map(|(file_key_string, file_entry)| {
                let bucket_name = self.bucket_name.clone();
                tokio::spawn(async move {
                    match Object::create(
                        bucket_name.as_str(),
                        file_entry,
                        file_key_string.as_str(),
                        // Upload json as binary as well.
                        BINARY_FILE_TYPE,
                    )
                    .await
                    {
                        Ok(_) => Ok(()),
                        Err(err) => Err(anyhow::Error::from(err)),
                    }
                })
            })
            .collect_vec();

        let results = match futures::future::try_join_all(tasks).await {
            Ok(res) => res,
            Err(err) => panic!("Error processing transaction batches: {:?}", err),
        };
        // If any uploading fails, retry.
        if any(results, |x| x.is_err()) {
            anyhow::bail!("Uploading transactions failed.");
        }

        if let Some(ts) = self.latest_metadata_update_timestamp {
            // a periodic metadata update
            if ts.elapsed().as_secs() > FILE_STORE_UPDATE_FREQUENCY_SECS {
                self.update_file_store_metadata(
                    chain_id,
                    start_version + batch_size as u64,
                    self.storage_format,
                )
                .await?;
            }
        } else {
            // the first metadata update
            self.update_file_store_metadata(
                chain_id,
                start_version + batch_size as u64,
                self.storage_format,
            )
            .await?;
        }

        Ok(())
    }

    async fn get_or_create_verification_metadata(
        &self,
        chain_id: u64,
    ) -> Result<VerificationMetadata> {
        let file_metadata = self
            .get_file_store_metadata()
            .await
            .ok_or(anyhow::anyhow!("No file store metadata found"))?;
        anyhow::ensure!(file_metadata.chain_id == chain_id, "Chain ID mismatch");

        match Object::download(&self.bucket_name, VERIFICATION_FILE_NAME).await {
            Ok(verification_metadata) => {
                let metadata: VerificationMetadata = serde_json::from_slice(&verification_metadata)
                    .expect("Expected metadata to be valid JSON.");
                anyhow::ensure!(metadata.chain_id == chain_id, "Chain ID mismatch.");
                Ok(metadata)
            },
            Err(cloud_storage::Error::Other(err)) => {
                if err.contains("No such object: ") {
                    // Metadata is not found.
                    let metadata = VerificationMetadata {
                        chain_id,
                        next_version_to_verify: 0,
                    };
                    match Object::create(
                        self.bucket_name.as_str(),
                        serde_json::to_vec(&metadata).unwrap(),
                        VERIFICATION_FILE_NAME,
                        JSON_FILE_TYPE,
                    )
                    .await
                    {
                        Ok(_) => Ok(metadata),
                        Err(err) => Err(anyhow::Error::from(err)),
                    }
                } else {
                    Err(anyhow::anyhow!("{:?}", err))
                }
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }
}
