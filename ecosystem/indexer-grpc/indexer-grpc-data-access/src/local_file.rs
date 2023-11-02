// Copyright © Aptos Foundation

use crate::{
    access_trait::{AccessMetadata, StorageReadError, StorageReadStatus, StorageTransactionRead},
    get_transactions_file_name, FileMetadata, TransactionsFile,
};
use aptos_protos::transaction::v1::Transaction;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const LOCAL_FILE_STORAGE_NAME: &str = "Local File";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFileClientConfig {
    // The absolute path to the folder that contains the transactions files.
    path: String,
}

#[derive(Clone)]
pub struct LocalFileClient {
    pub file_path: PathBuf,
}

impl LocalFileClient {
    pub fn new(config: LocalFileClientConfig) -> anyhow::Result<Self> {
        Ok(Self {
            file_path: PathBuf::from(config.path),
        })
    }
}

impl From<std::io::Error> for StorageReadError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            // Fetch an entry that is not set yet.
            std::io::ErrorKind::NotFound => {
                StorageReadError::PermenantError(LOCAL_FILE_STORAGE_NAME, anyhow::Error::new(err))
            },
            // Other errors are transient; let it retry.
            _ => StorageReadError::TransientError(LOCAL_FILE_STORAGE_NAME, anyhow::Error::new(err)),
        }
    }
}

#[async_trait::async_trait]
impl StorageTransactionRead for LocalFileClient {
    async fn get_transactions(
        &self,
        batch_starting_version: u64,
        _size_hint: Option<usize>,
    ) -> Result<StorageReadStatus, StorageReadError> {
        let file_path = self
            .file_path
            .clone()
            .join(get_transactions_file_name(batch_starting_version));
        let file = match tokio::fs::read(file_path.clone()).await {
            Ok(file) => file,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        // The file is not found. This is not an error.
                        return Ok(StorageReadStatus::NotFound);
                    },
                    _ => {
                        return Err(StorageReadError::PermenantError(
                            LOCAL_FILE_STORAGE_NAME,
                            anyhow::anyhow!(
                                "Failed to find txns file '{}': {}",
                                file_path.display(),
                                e
                            ),
                        ));
                    },
                }
            },
        };
        let transactions_file = TransactionsFile::from(file);
        let all_transactions: Vec<Transaction> = transactions_file.into();
        let transactions = all_transactions
            .into_iter()
            .skip((batch_starting_version % 1000) as usize)
            .collect::<Vec<Transaction>>();
        Ok(StorageReadStatus::Ok(transactions))
    }

    async fn get_metadata(&self) -> Result<AccessMetadata, StorageReadError> {
        let file_path = self.file_path.clone().join("metadata.json");
        let metadata = FileMetadata::from(tokio::fs::read(file_path.clone()).await?);
        Ok(AccessMetadata {
            chain_id: metadata.chain_id,
            next_version: metadata.version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_protos::transaction::v1::Transaction;
    use prost::Message;
    use std::{
        fs::{create_dir, File},
        io::Write,
    };
    fn create_transactions(starting_version: u64) -> Vec<Transaction> {
        (starting_version..starting_version + 1000)
            .map(|version| Transaction {
                version,
                ..Default::default()
            })
            .collect()
    }

    fn create_transactions_file(starting_version: u64) -> TransactionsFile {
        TransactionsFile {
            transactions: create_transactions(starting_version)
                .into_iter()
                .map(|transaction| {
                    let mut buf = Vec::new();
                    transaction.encode(&mut buf).unwrap();
                    base64::encode(buf)
                })
                .collect(),
            starting_version,
        }
    }
    #[tokio::test]
    async fn test_local_file_read_full_batch_successful() {
        // Create a temp file.
        let dir = tempfile::tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.json");
        create_dir(dir.path().join("files")).unwrap();
        let transactions_file_path = dir.path().join("files/0.json");
        // Write some data to the file.
        {
            let mut metadata_file = File::create(&metadata_path).unwrap();
            let file_metadata = FileMetadata {
                chain_id: 1,
                file_folder_size: 1000,
                version: 1000,
            };
            write!(
                metadata_file,
                "{}",
                serde_json::to_string(&file_metadata).unwrap()
            )
            .unwrap();
            let mut transactions_file = File::create(&transactions_file_path).unwrap();
            let transactions_file_obj = create_transactions_file(0);
            write!(
                transactions_file,
                "{}",
                serde_json::to_string(&transactions_file_obj).unwrap()
            )
            .unwrap();
        }

        let local_file_client = LocalFileClient::new(LocalFileClientConfig {
            path: dir.path().to_path_buf().to_str().unwrap().to_string(),
        })
        .unwrap();
        let transactions = local_file_client.get_transactions(0, None).await.unwrap();
        let access_metadata = local_file_client.get_metadata().await.unwrap();
        assert_eq!(access_metadata.chain_id, 1);
        assert_eq!(access_metadata.next_version, 1000);
        assert_eq!(transactions, StorageReadStatus::Ok(create_transactions(0)));
    }

    #[tokio::test]
    async fn test_local_file_read_partial_batch_successful() {
        // Create a temp file.
        let dir = tempfile::tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.json");
        create_dir(dir.path().join("files")).unwrap();
        let transactions_file_path = dir.path().join("files/0.json");
        // Write some data to the file.
        {
            let mut metadata_file = File::create(&metadata_path).unwrap();
            let file_metadata = FileMetadata {
                chain_id: 1,
                file_folder_size: 1000,
                version: 1000,
            };
            write!(
                metadata_file,
                "{}",
                serde_json::to_string(&file_metadata).unwrap()
            )
            .unwrap();
            let mut transactions_file = File::create(&transactions_file_path).unwrap();
            let transactions_file_obj = create_transactions_file(0);
            write!(
                transactions_file,
                "{}",
                serde_json::to_string(&transactions_file_obj).unwrap()
            )
            .unwrap();
        }

        let local_file_client = LocalFileClient::new(LocalFileClientConfig {
            path: dir.path().to_path_buf().to_str().unwrap().to_string(),
        })
        .unwrap();
        let transactions = local_file_client.get_transactions(500, None).await.unwrap();
        let access_metadata = local_file_client.get_metadata().await.unwrap();
        assert_eq!(access_metadata.chain_id, 1);
        assert_eq!(access_metadata.next_version, 1000);
        let partial_transactions_file = (500..1000)
            .map(|version| Transaction {
                version,
                ..Default::default()
            })
            .collect::<Vec<Transaction>>();
        assert_eq!(
            transactions,
            StorageReadStatus::Ok(partial_transactions_file)
        );
    }

    #[tokio::test]
    async fn test_local_file_metadata_missing() {
        // Create a temp file.
        let dir = tempfile::tempdir().unwrap();
        let local_file_client = LocalFileClient::new(LocalFileClientConfig {
            path: dir.path().to_path_buf().to_str().unwrap().to_string(),
        })
        .unwrap();
        let access_metadata = local_file_client.get_metadata().await;
        assert!(access_metadata.is_err());
        assert!(matches!(
            access_metadata.unwrap_err(),
            StorageReadError::PermenantError(LOCAL_FILE_STORAGE_NAME, _)
        ));
    }

    #[tokio::test]
    async fn test_local_file_transactions_file_not_found() {
        // Create a temp file.
        let dir = tempfile::tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.json");
        // Write some data to the file.
        {
            let mut metadata_file = File::create(&metadata_path).unwrap();
            let file_metadata = FileMetadata {
                chain_id: 1,
                file_folder_size: 1000,
                // No transactions yet.
                version: 0,
            };
            write!(
                metadata_file,
                "{}",
                serde_json::to_string(&file_metadata).unwrap()
            )
            .unwrap();
        }

        let local_file_client = LocalFileClient::new(LocalFileClientConfig {
            path: dir.path().to_path_buf().to_str().unwrap().to_string(),
        })
        .unwrap();
        let transactions = local_file_client.get_transactions(0, None).await;
        let access_metadata = local_file_client.get_metadata().await.unwrap();

        assert_eq!(access_metadata.chain_id, 1);
        assert_eq!(access_metadata.next_version, 0);
        assert!(transactions.is_ok());
        assert!(transactions.unwrap() == StorageReadStatus::NotFound);
    }
}
