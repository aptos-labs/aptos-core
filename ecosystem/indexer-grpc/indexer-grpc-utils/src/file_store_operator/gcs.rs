// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{constants::BLOB_STORAGE_SIZE, file_store_operator::*, EncodedTransactionWithVersion};
use cloud_storage::{Bucket, Object};
use itertools::{any, Itertools};

const JSON_FILE_TYPE: &str = "application/json";

pub struct GcsFileStoreOperator {
    bucket_name: String,
    /// The timestamp of the latest metadata update; this is to avoid too frequent metadata update.
    latest_metadata_update_timestamp: Option<std::time::Instant>,
}

impl GcsFileStoreOperator {
    pub fn new(bucket_name: String) -> Self {
        Self {
            bucket_name,
            latest_metadata_update_timestamp: None,
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
                    serde_json::from_slice(&file).expect("Expected file to be valid JSON.");
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
                self.latest_metadata_update_timestamp = Some(std::time::Instant::now());
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
        transactions: Vec<EncodedTransactionWithVersion>,
    ) -> anyhow::Result<()> {
        let start_version = transactions.first().unwrap().1;
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

        // Split the transactions into batches of BLOB_STORAGE_SIZE.
        for i in transactions.chunks(BLOB_STORAGE_SIZE) {
            let bucket_name = self.bucket_name.clone();
            let current_batch = i.iter().cloned().collect_vec();
            let transactions_file = build_transactions_file(current_batch).unwrap();
            let task = tokio::spawn(async move {
                match Object::create(
                    bucket_name.clone().as_str(),
                    serde_json::to_vec(&transactions_file).unwrap(),
                    generate_blob_name(transactions_file.starting_version).as_str(),
                    JSON_FILE_TYPE,
                )
                .await
                {
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
}
