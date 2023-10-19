// Copyright © Aptos Foundation

use crate::{
    access_trait::{AccessMetadata, StorageReadError, StorageReadStatus, StorageTransactionRead},
    in_memory_storage::storage::{InMemoryStorageInternal, IN_MEMORY_STORAGE_SIZE_SOFT_LIMIT},
};
use anyhow::Context;
use aptos_protos::transaction::v1::Transaction;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const IN_MEMORY_STORAGE_NAME: &str = "In Memory";
const IN_MEMORY_STORAGE_READ_SIZE: usize = 1000;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InMemoryStorageClientConfig {
    // The source of the transactions.
    redis_address: String,
}

#[derive(Clone)]
pub struct InMemoryStorageClient {
    internal: Arc<InMemoryStorageInternal>,
}

impl InMemoryStorageClient {
    // For each process, to avoid memory explosion, only create the client once and copy the reference
    // to other threads.
    pub async fn new(redis_address: String) -> anyhow::Result<Self> {
        let internal = InMemoryStorageInternal::new(redis_address)
            .await
            .context("Internal storage initialization failed.")?;
        Ok(Self {
            internal: Arc::new(internal),
        })
    }
}

#[async_trait::async_trait]
impl StorageTransactionRead for InMemoryStorageClient {
    async fn get_transactions(
        &self,
        batch_starting_version: u64,
        _size_hint: Option<usize>,
    ) -> Result<StorageReadStatus, StorageReadError> {
        let current_metadata = self.get_metadata().await?;

        let lowest_available_version = current_metadata
            .next_version
            .saturating_sub(IN_MEMORY_STORAGE_SIZE_SOFT_LIMIT as u64);
        if batch_starting_version < lowest_available_version {
            // The requested version is too low.
            return Ok(StorageReadStatus::NotFound);
        }
        let highest_version = std::cmp::min(
            current_metadata.next_version,
            batch_starting_version + IN_MEMORY_STORAGE_READ_SIZE as u64,
        );

        let mut transaction_refs = Vec::new();
        for version in batch_starting_version..highest_version {
            let read_result = self.internal.transactions_map.get(&version);
            match read_result {
                Some(transaction_ref) => {
                    let transaction = transaction_ref.clone();
                    transaction_refs.push(transaction);
                },
                None => break,
            }
        }
        let transactions: Vec<Transaction> = transaction_refs
            .into_iter()
            .map(|transaction_ref| (*transaction_ref).clone())
            .collect();
        match transactions.len() {
            0 => Ok(StorageReadStatus::NotFound),
            _ => Ok(StorageReadStatus::Ok(transactions)),
        }
    }

    async fn get_metadata(&self) -> Result<AccessMetadata, StorageReadError> {
        match self.internal.metadata.read() {
            Ok(metadata) => {
                match *metadata {
                    Some(ref metadata) => Ok(metadata.clone()),
                    // Metadata is not ready yet; needs retry.
                    None => Err(StorageReadError::TransientError(
                        IN_MEMORY_STORAGE_NAME,
                        anyhow::anyhow!("No metadata".to_string()),
                    )),
                }
            },
            Err(err) => Err(StorageReadError::PermenantError(
                IN_MEMORY_STORAGE_NAME,
                anyhow::anyhow!("Failed to read metadata: {:#}", err),
            )),
        }
    }
}
