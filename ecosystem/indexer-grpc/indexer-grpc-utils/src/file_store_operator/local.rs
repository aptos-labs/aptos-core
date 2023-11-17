// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    constants::BLOB_STORAGE_SIZE,
    file_store_operator::*,
    storage::{FileEntry, FileEntryBuilder, FileEntryKey, StorageFormat},
};
use aptos_protos::indexer::v1::TransactionsInStorage;
use itertools::{any, Itertools};
use std::path::PathBuf;
use tracing::info;

pub struct LocalFileStoreOperator {
    path: PathBuf,
    /// The timestamp of the latest metadata update; this is to avoid too frequent metadata update.
    latest_metadata_update_timestamp: Option<std::time::Instant>,
    storage_format: StorageFormat,
}

impl LocalFileStoreOperator {
    pub fn new(path: PathBuf, storage_format: StorageFormat) -> Self {
        Self {
            path,
            latest_metadata_update_timestamp: None,
            storage_format,
        }
    }
}

#[async_trait::async_trait]
impl FileStoreOperator for LocalFileStoreOperator {
    async fn verify_storage_bucket_existence(&self) {
        tracing::info!(
            bucket_name = self.path.to_str().unwrap(),
            "Before file store operator starts, verify the bucket exists."
        );
        if !self.path.exists() {
            panic!("File store path does not exist.");
        }
    }

    async fn get_transactions(&self, version: u64) -> anyhow::Result<Vec<Transaction>> {
        let current_file_name = FileEntryKey::new(version, self.storage_format).to_string();
        let file_path = self.path.join(current_file_name);
        match tokio::fs::read(file_path).await {
            Ok(file) => {
                let file_entry = FileEntry::from_bytes(file, self.storage_format);
                let transactions_in_storage = TransactionsInStorage::try_from(file_entry)?;
                let transactions = transactions_in_storage
                    .transactions
                    .into_iter()
                    .filter(|t| t.version >= version)
                    .collect::<Vec<Transaction>>();
                Ok(transactions)
            },
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    anyhow::bail!("[Indexer File] Transactions file not found. Gap might happen between cache and file store. {}", err)
                } else {
                    anyhow::bail!(
                        "[Indexer File] Error happens when transaction file. {}",
                        err
                    );
                }
            },
        }
    }

    async fn get_file_store_metadata(&self) -> Option<FileStoreMetadata> {
        let metadata_path = self.path.join(METADATA_FILE_NAME);
        match tokio::fs::read(metadata_path).await {
            Ok(metadata) => {
                let metadata: FileStoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                Some(metadata)
            },
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    // Metadata is not found.
                    None
                } else {
                    panic!(
                        "[Indexer File] Error happens when accessing metadata file. {}",
                        err
                    );
                }
            },
        }
    }

    async fn create_default_file_store_metadata_if_absent(
        &mut self,
        expected_chain_id: u64,
    ) -> anyhow::Result<FileStoreMetadata> {
        let metadata_path = self.path.join(METADATA_FILE_NAME);
        match tokio::fs::read(metadata_path).await {
            Ok(metadata) => {
                let metadata: FileStoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                anyhow::ensure!(metadata.chain_id == expected_chain_id, "Chain ID mismatch.");
                Ok(metadata)
            },
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    // If the metadata is not found, it means the file store is empty.
                    info!("File store is empty. Creating metadata file.");
                    self.update_file_store_metadata(expected_chain_id, 0)
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
        }
    }

    async fn update_file_store_metadata(
        &mut self,
        chain_id: u64,
        version: u64,
    ) -> anyhow::Result<()> {
        let metadata = FileStoreMetadata::new(chain_id, version, self.storage_format);
        // If the metadata is not updated, the indexer will be restarted.
        let metadata_path = self.path.join(METADATA_FILE_NAME);
        info!(
            "Updating metadata file {} @ version {}",
            metadata_path.display(),
            version
        );
        match tokio::fs::write(metadata_path, serde_json::to_vec(&metadata).unwrap()).await {
            Ok(_) => {
                self.latest_metadata_update_timestamp = Some(std::time::Instant::now());
                Ok(())
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }

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
        let mut tasks = vec![];

        // create files directory
        let files_dir = self.path.join(FILE_FOLDER_NAME);
        if !files_dir.exists() {
            tracing::info!("Creating files directory {:?}", files_dir.clone());
            tokio::fs::create_dir(files_dir.clone()).await?;
        }

        // Split the transactions into batches of BLOB_STORAGE_SIZE.
        for i in transactions.chunks(BLOB_STORAGE_SIZE) {
            let current_batch = i.iter().cloned().collect_vec();
            let first_version = current_batch.first().unwrap().version;
            let file_entry_builder = FileEntryBuilder::new(current_batch, self.storage_format);
            let file_entry = FileEntry::try_from(file_entry_builder)?;
            let file_name = FileEntryKey::new(first_version, self.storage_format).to_string();
            let file_path = self.path.join(file_name);
            let task = tokio::spawn(async move {
                match tokio::fs::write(file_path, file_entry.into_inner()).await {
                    Ok(_) => Ok(()),
                    Err(err) => Err(anyhow::Error::from(err)),
                }
            });
            tasks.push(task);
        }
        let results = match futures::future::try_join_all(tasks).await {
            Ok(res) => res,
            Err(err) => panic!("Error processing transaction batches: {:?}", err),
        };
        // If any uploading fails, retry.
        for result in &results {
            if result.is_err() {
                tracing::error!("Error happens when uploading transactions. {:?}", result);
            }
        }
        if any(results, |x| x.is_err()) {
            anyhow::bail!("Uploading transactions failed.");
        }

        if let Some(ts) = self.latest_metadata_update_timestamp {
            // a periodic metadata update
            if (std::time::Instant::now() - ts).as_secs() > FILE_STORE_UPDATE_FREQUENCY_SECS {
                self.update_file_store_metadata(chain_id, start_version + batch_size as u64)
                    .await?;
            }
        } else {
            // the first metadata update
            self.update_file_store_metadata(chain_id, start_version + batch_size as u64)
                .await?;
        }

        Ok(())
    }

    async fn get_or_create_verification_metadata(
        &self,
        _chain_id: u64,
    ) -> Result<VerificationMetadata> {
        anyhow::bail!("Verification is not impelemented for local file store.")
    }

    async fn update_verification_metadata(
        &mut self,
        _chain_id: u64,
        _next_version_to_verify: u64,
    ) -> Result<()> {
        anyhow::bail!("Verification is not impelemented for local file store.")
    }
}
