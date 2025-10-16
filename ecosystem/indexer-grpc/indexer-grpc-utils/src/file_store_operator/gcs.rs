// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compression_util::{FileEntry, FileStoreMetadata, StorageFormat, FILE_ENTRY_TRANSACTION_COUNT},
    counters::{log_grpc_step, IndexerGrpcStep},
    file_store_operator::{FileStoreOperator, METADATA_FILE_NAME},
};
use anyhow::bail;
use aptos_protos::transaction::v1::Transaction;
use cloud_storage::{Bucket, Object};
use std::{env, path::PathBuf};

const JSON_FILE_TYPE: &str = "application/json";
// The environment variable to set the service account path.
const SERVICE_ACCOUNT_ENV_VAR: &str = "SERVICE_ACCOUNT";
const FILE_STORE_METADATA_TIMEOUT_MILLIS: u128 = 200;

#[derive(Clone)]
pub struct GcsFileStoreOperator {
    bucket_name: String,
    bucket_sub_dir: Option<PathBuf>,
    file_store_metadata_last_updated: std::time::Instant,
    storage_format: StorageFormat,
    metadata_file_path: PathBuf,
}

impl GcsFileStoreOperator {
    pub fn new(
        bucket_name: String,
        bucket_sub_dir: Option<PathBuf>,
        service_account_path: String,
        enable_compression: bool,
    ) -> Self {
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe {
            env::set_var(SERVICE_ACCOUNT_ENV_VAR, service_account_path);
        }
        let storage_format = if enable_compression {
            StorageFormat::Lz4CompressedProto
        } else {
            StorageFormat::JsonBase64UncompressedProto
        };
        let metadata_file_path = match &bucket_sub_dir {
            Some(sub_dir) => {
                let mut metadata_file_path = sub_dir.clone();
                metadata_file_path.push(METADATA_FILE_NAME);
                metadata_file_path
            },
            None => {
                let mut metadata_file_path = PathBuf::new();
                metadata_file_path.push(METADATA_FILE_NAME);
                metadata_file_path
            },
        };
        Self {
            bucket_name,
            bucket_sub_dir,
            file_store_metadata_last_updated: std::time::Instant::now(),
            storage_format,
            metadata_file_path,
        }
    }

    /// Given a version number, builds the key path for the file entry. This is dependent on the storage format and whether we opt
    /// to use a sub directory, in the case of a shared bucket. The key path can be used directly as a GCS bucket file path.
    fn get_file_entry_key_path(&self, version: u64) -> String {
        let file_entry_key = FileEntry::build_key(version, self.storage_format).to_string();
        // If the sub directory is set, the file entry key will be prefixed with the sub directory.
        match &self.bucket_sub_dir {
            Some(sub_dir) => {
                let mut file_entry_key_path = sub_dir.clone();
                file_entry_key_path.push(file_entry_key);
                file_entry_key_path.to_string_lossy().into_owned()
            },
            None => file_entry_key,
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

    fn storage_format(&self) -> StorageFormat {
        self.storage_format
    }

    fn store_name(&self) -> &str {
        "GCS"
    }

    async fn get_raw_file(&self, version: u64) -> anyhow::Result<Vec<u8>> {
        let file_entry_key_path = self.get_file_entry_key_path(version);
        match Object::download(&self.bucket_name, file_entry_key_path.as_str()).await {
            Ok(file) => Ok(file),
            Err(cloud_storage::Error::Other(err)) => {
                if err.contains("No such object: ") {
                    anyhow::bail!("[Indexer File] Transactions file not found. Gap might happen between cache and file store. {}", err)
                } else {
                    anyhow::bail!(
                        "[Indexer File] Error happens when downloading transaction file. {}",
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
        match Object::download(
            &self.bucket_name,
            self.metadata_file_path
                .to_str()
                .expect("Expected metadata file path to be valid."),
        )
        .await
        {
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
            assert_eq!(metadata.chain_id, expected_chain_id, "Chain ID mismatch.");
            assert_eq!(
                metadata.storage_format, self.storage_format,
                "Storage format mismatch."
            );
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
        let metadata = FileStoreMetadata::new(chain_id, version, self.storage_format);
        // If the metadata is not updated, the indexer will be restarted.
        Object::create(
            self.bucket_name.as_str(),
            serde_json::to_vec(&metadata).unwrap(),
            self.metadata_file_path
                .to_str()
                .expect("Expected metadata file path to be valid."),
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
        transactions: Vec<Transaction>,
    ) -> anyhow::Result<(u64, u64)> {
        let start_version = transactions.first().unwrap().version;
        let end_version = transactions.last().unwrap().version;
        let batch_size = transactions.len();
        anyhow::ensure!(
            start_version % FILE_ENTRY_TRANSACTION_COUNT == 0,
            "Starting version has to be a multiple of BLOB_STORAGE_SIZE."
        );
        anyhow::ensure!(
            batch_size == FILE_ENTRY_TRANSACTION_COUNT as usize,
            "The number of transactions to upload has to be multiplier of BLOB_STORAGE_SIZE."
        );
        let start_time = std::time::Instant::now();
        let bucket_name = self.bucket_name.clone();
        let file_entry = FileEntry::from_transactions(transactions, self.storage_format);
        let file_entry_key_path = self.get_file_entry_key_path(start_version);
        log_grpc_step(
            "file_worker",
            IndexerGrpcStep::FileStoreEncodedTxns,
            Some(start_version as i64),
            Some((start_version + FILE_ENTRY_TRANSACTION_COUNT - 1) as i64),
            None,
            None,
            Some(start_time.elapsed().as_secs_f64()),
            None,
            Some(FILE_ENTRY_TRANSACTION_COUNT as i64),
            None,
        );
        Object::create(
            bucket_name.clone().as_str(),
            file_entry.into_inner(),
            file_entry_key_path.as_str(),
            JSON_FILE_TYPE,
        )
        .await?;
        Ok((start_version, end_version))
    }

    fn clone_box(&self) -> Box<dyn FileStoreOperator> {
        Box::new(self.clone())
    }
}
