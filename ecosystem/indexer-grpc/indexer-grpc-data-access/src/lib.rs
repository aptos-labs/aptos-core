// Copyright © Aptos Foundation

use aptos_protos::transaction::v1::Transaction;
use serde::{Deserialize, Serialize};

pub mod access_trait;
pub mod gcs;
pub mod in_memory;
pub mod in_memory_storage;
pub mod local_file;
pub mod redis;

use crate::access_trait::{
    AccessMetadata, StorageReadError, StorageReadStatus, StorageTransactionRead,
};

#[enum_dispatch::enum_dispatch]
#[derive(Clone)]
pub enum StorageClient {
    InMemory(in_memory::InMemoryStorageClient),
    Redis(redis::RedisClient),
    LocalFile(local_file::LocalFileClient),
    Gcs(gcs::GcsClient),
    MockClient(MockStorageClient),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "storage_type")]
pub enum IndexerStorageClientConfig {
    InMemory(in_memory::InMemoryStorageClientConfig),
    Redis(redis::RedisClientConfig),
    Gcs(gcs::GcsClientConfig),
    LocalFile(local_file::LocalFileClientConfig),
}

impl IndexerStorageClientConfig {
    pub async fn create_client(&self) -> anyhow::Result<StorageClient> {
        match self {
            IndexerStorageClientConfig::InMemory(config) => Ok(StorageClient::InMemory(
                in_memory::InMemoryStorageClient::new(config.clone()).await?,
            )),
            IndexerStorageClientConfig::Redis(config) => Ok(StorageClient::Redis(
                redis::RedisClient::new(config.clone()).await?,
            )),
            IndexerStorageClientConfig::Gcs(config) => Ok(StorageClient::Gcs(
                gcs::GcsClient::new(config.clone()).await?,
            )),
            IndexerStorageClientConfig::LocalFile(config) => Ok(StorageClient::LocalFile(
                local_file::LocalFileClient::new(config.clone()).await?,
            )),
        }
    }
}

const REDIS_ENDING_VERSION_EXCLUSIVE_KEY: &str = "latest_version";
const REDIS_CHAIN_ID: &str = "chain_id";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
struct FileMetadata {
    pub chain_id: u64,
    pub file_folder_size: u64,
    pub version: u64,
}

impl From<Vec<u8>> for FileMetadata {
    fn from(bytes: Vec<u8>) -> Self {
        serde_json::from_slice(bytes.as_slice()).expect("Failed to deserialize FileMetadata.")
    }
}

pub struct MockStorageClient {
    metadata: AccessMetadata,
    transactions: Vec<Transaction>,
}

impl MockStorageClient {
    pub fn new(chain_id: u64, transactions: Vec<Transaction>) -> Self {
        let next_version = transactions.last().unwrap().version + 1;
        Self {
            metadata: AccessMetadata {
                chain_id,
                next_version,
            },
            transactions,
        }
    }
}

impl Clone for MockStorageClient {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            transactions: self.transactions.clone(),
        }
    }
}

#[async_trait::async_trait]
impl StorageTransactionRead for MockStorageClient {
    async fn get_metadata(&self) -> Result<AccessMetadata, StorageReadError> {
        Ok(self.metadata.clone())
    }

    async fn get_transactions(
        &self,
        start_version: u64,
        _limit: Option<usize>,
    ) -> Result<StorageReadStatus, StorageReadError> {
        let current_starting_version = self.transactions.first().unwrap().version;
        if current_starting_version > start_version {
            return Ok(StorageReadStatus::NotFound);
        }

        let current_next_version = self.metadata.next_version;
        if start_version >= current_next_version {
            return Ok(StorageReadStatus::NotAvailableYet);
        }

        return Ok(StorageReadStatus::Ok(
            self.transactions
                .iter()
                .filter(|v| v.version >= start_version)
                .cloned()
                .collect(),
        ));
    }

    async fn is_storage_ready(&self) -> bool {
        true
    }
}
