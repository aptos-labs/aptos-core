// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compression_util::{FileEntry, FileStoreMetadata, StorageFormat, FILE_ENTRY_TRANSACTION_COUNT},
    file_store_operator::{
        FileStoreOperator, FILE_STORE_UPDATE_FREQUENCY_SECS, METADATA_FILE_NAME,
    },
};
use velor_protos::transaction::v1::Transaction;
use itertools::{any, Itertools};
use std::path::PathBuf;
use tracing::info;

#[derive(Clone)]
pub struct LocalFileStoreOperator {
    path: PathBuf,
    /// The timestamp of the latest metadata update; this is to avoid too frequent metadata update.
    latest_metadata_update_timestamp: Option<std::time::Instant>,
    storage_format: StorageFormat,
}

impl LocalFileStoreOperator {
    pub fn new(path: PathBuf, enable_compression: bool) -> Self {
        let storage_format = if enable_compression {
            StorageFormat::Lz4CompressedProto
        } else {
            StorageFormat::JsonBase64UncompressedProto
        };
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

    fn storage_format(&self) -> StorageFormat {
        self.storage_format
    }

    fn store_name(&self) -> &str {
        "local"
    }

    async fn get_raw_file(&self, version: u64) -> anyhow::Result<Vec<u8>> {
        let file_entry_key = FileEntry::build_key(version, self.storage_format).to_string();
        let file_path = self.path.join(file_entry_key);
        match tokio::fs::read(file_path).await {
            Ok(file) => Ok(file),
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
            Ok(metadata) => Some(FileStoreMetadata::from_bytes(metadata)),
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

    async fn update_file_store_metadata_with_timeout(
        &mut self,
        expected_chain_id: u64,
        _version: u64,
    ) -> anyhow::Result<()> {
        let metadata_path = self.path.join(METADATA_FILE_NAME);
        match tokio::fs::read(metadata_path).await {
            Ok(metadata) => {
                let metadata: FileStoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                anyhow::ensure!(metadata.chain_id == expected_chain_id, "Chain ID mismatch.");
                Ok(())
            },
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    // If the metadata is not found, it means the file store is empty.
                    info!("File store is empty. Creating metadata file.");
                    self.update_file_store_metadata_internal(expected_chain_id, 0)
                        .await
                        .expect("[Indexer File] Update metadata failed.");
                    Ok(())
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

    async fn update_file_store_metadata_internal(
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

    /// TODO: rewrite this function to be similar to the general version
    async fn upload_transaction_batch(
        &mut self,
        chain_id: u64,
        transactions: Vec<Transaction>,
    ) -> anyhow::Result<(u64, u64)> {
        let start_version = transactions.first().unwrap().version;
        let batch_size = transactions.len();
        anyhow::ensure!(
            start_version % FILE_ENTRY_TRANSACTION_COUNT == 0,
            "Starting version has to be a multiple of BLOB_STORAGE_SIZE."
        );
        anyhow::ensure!(
            batch_size % FILE_ENTRY_TRANSACTION_COUNT as usize == 0,
            "The number of transactions to upload has to be multiplier of BLOB_STORAGE_SIZE."
        );
        let mut tasks = vec![];

        // Split the transactions into batches of BLOB_STORAGE_SIZE.
        for i in transactions.chunks(FILE_ENTRY_TRANSACTION_COUNT as usize) {
            let current_batch = i.iter().cloned().collect_vec();
            let starting_version = current_batch.first().unwrap().version;
            let file_entry = FileEntry::from_transactions(current_batch, self.storage_format);
            let file_entry_key =
                FileEntry::build_key(starting_version, self.storage_format).to_string();
            let txns_path = self.path.join(file_entry_key.as_str());
            let parent_dir = txns_path.parent().unwrap();
            if !parent_dir.exists() {
                tracing::debug!("Creating parent dir: {parent_dir:?}.");
                tokio::fs::create_dir_all(parent_dir).await?;
            }

            tracing::debug!(
                "Uploading transactions to {:?}",
                txns_path.to_str().unwrap()
            );
            let task = tokio::spawn(async move {
                match tokio::fs::write(txns_path, file_entry.into_inner()).await {
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
                self.update_file_store_metadata_internal(
                    chain_id,
                    start_version + batch_size as u64,
                )
                .await?;
            }
        } else {
            // the first metadata update
            self.update_file_store_metadata_internal(chain_id, start_version + batch_size as u64)
                .await?;
        }

        Ok((start_version, start_version + batch_size as u64 - 1))
    }

    fn clone_box(&self) -> Box<dyn FileStoreOperator> {
        Box::new(self.clone())
    }
}
