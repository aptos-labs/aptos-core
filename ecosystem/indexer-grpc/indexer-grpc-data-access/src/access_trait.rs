// Copyright © Aptos Foundation

use aptos_protos::transaction::v1::Transaction;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum StorageReadStatus {
    // Requested version is available for the given storage.
    Ok(Vec<Transaction>),
    // Requested version is not available yet for the given storage.
    NotAvailableYet,
    // Requested version is not available anymore for the given storage.
    NotFound,
}

#[derive(Error, Debug)]
pub enum StorageReadError {
    // Storage is not available; but you can try again later.
    #[error("[{0}] Storage access transient error: {1:#}")]
    TransientError(&'static str, #[source] anyhow::Error),
    // Storage is not available; and you should not try again.
    #[error("[{0}] Storage access permanent error: {1:#}")]
    PermenantError(&'static str, #[source] anyhow::Error),
}

#[derive(Clone, Debug, Default)]
pub struct AccessMetadata {
    // The chain id of the transactions; this is used to check if the transactions are from the same chain.
    pub chain_id: u64,
    // The next version in the storage to process.
    pub next_version: u64,
}

impl PartialEq for AccessMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.chain_id == other.chain_id
    }
}

/// StorageTransactionRead is the interface for reading transactions from storage. It's expected to be implemented by all storages and
/// cloning the trait object should be cheap.
#[async_trait::async_trait]
#[enum_dispatch::enum_dispatch(StorageClient)]
pub trait StorageTransactionRead: Send + Sync + Clone {
    // Fetches the transactions from storage starting from the given version.
    // The response returned has the following semantics:
    // - If the requested version is available, the response will contain the transactions starting from the requested version.
    // - If the requested version is not available yet, NotAvailableYet will be returned.
    // - If the requested version is not available anymore, NotFound will be returned.
    async fn get_transactions(
        &self,
        batch_starting_version: u64,
        size_hint: Option<usize>,
    ) -> Result<StorageReadStatus, StorageReadError>;

    // Fetches the metadata from storage and check against the other storages.
    // E.g., redis metadata == gcs metadata == in-memory metadata.
    async fn get_metadata(&self) -> Result<AccessMetadata, StorageReadError>;

    async fn is_storage_ready(&self) -> bool {
        self.get_metadata().await.is_ok()
    }
}

// TODO: Add write interface for cache worker + file storage.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_access_metadata_different_chain() {
        let mainnet_metadata = AccessMetadata {
            chain_id: 1,
            next_version: 100,
        };
        let testnet_metadata = AccessMetadata {
            chain_id: 2,
            next_version: 100,
        };
        assert_ne!(mainnet_metadata, testnet_metadata);
    }

    #[tokio::test]
    async fn test_access_metadata_same_chain() {
        let mainnet_metadata = AccessMetadata {
            chain_id: 1,
            next_version: 100,
        };
        let testnet_metadata = AccessMetadata {
            chain_id: 1,
            next_version: 101,
        };
        assert_eq!(mainnet_metadata, testnet_metadata);
    }
}
