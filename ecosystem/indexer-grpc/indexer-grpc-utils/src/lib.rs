// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod cache_operator;
pub mod config;
pub mod constants;
pub mod file_store_operator;

use aptos_protos::{
    internal::fullnode::v1::fullnode_data_client::FullnodeDataClient, transaction::v1::Transaction,
    util::timestamp::Timestamp,
};
use prost::Message;

pub type GrpcClientType = FullnodeDataClient<tonic::transport::Channel>;

/// Create a gRPC client with exponential backoff.
pub async fn create_grpc_client(address: String) -> GrpcClientType {
    backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
        match FullnodeDataClient::connect(address.clone()).await {
            Ok(client) => {
                tracing::info!(
                    address = address.clone(),
                    "[Indexer Cache] Connected to indexer gRPC server."
                );
                Ok(client)
            },
            Err(e) => {
                tracing::error!(
                    address = address.clone(),
                    "[Indexer Cache] Failed to connect to indexer gRPC server: {}",
                    e
                );
                Err(backoff::Error::transient(e))
            },
        }
    })
    .await
    .unwrap()
}

// (Protobuf encoded transaction, version)
pub type EncodedTransactionWithVersion = (String, u64);
/// Build the EncodedTransactionWithVersion from the encoded transactions and starting version.
#[inline]
pub fn build_protobuf_encoded_transaction_wrappers(
    encoded_transactions: Vec<String>,
    starting_version: u64,
) -> Vec<EncodedTransactionWithVersion> {
    encoded_transactions
        .into_iter()
        .enumerate()
        .map(|(ind, encoded_transaction)| (encoded_transaction, starting_version + ind as u64))
        .collect()
}

pub fn time_diff_since_pb_timestamp_in_secs(timestamp: &Timestamp) -> f64 {
    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs_f64();
    let transaction_time = timestamp.seconds as f64 + timestamp.nanos as f64 * 1e-9;
    current_timestamp - transaction_time
}

/// Chunk transactions into chunks with chunk size less than or equal to chunk_size.
/// If a single transaction is larger than chunk_size, it will be put into a chunk by itself.
pub fn chunk_transactions(
    transactions: Vec<Transaction>,
    chunk_size: usize,
) -> Vec<Vec<Transaction>> {
    let mut chunked_transactions = vec![];
    let mut chunk = vec![];
    let mut current_size = 0;

    for transaction in transactions {
        // Only add the chunk when it's empty.
        if !chunk.is_empty() && current_size + transaction.encoded_len() > chunk_size {
            chunked_transactions.push(chunk);
            chunk = vec![];
            current_size = 0;
        }
        current_size += transaction.encoded_len();
        chunk.push(transaction);
    }
    if !chunk.is_empty() {
        chunked_transactions.push(chunk);
    }
    chunked_transactions
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_the_transactions_correctly_with_large_transaction() {
        let t = Transaction {
            version: 2,
            timestamp: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
            ..Transaction::default()
        };
        // Create a vec with 10 transactions.
        let transactions = vec![t.clone(); 10];
        assert!(t.encoded_len() > 1);
        let chunked_transactions = chunk_transactions(transactions, 1);
        assert_eq!(chunked_transactions.len(), 10);
    }

    #[test]
    fn test_chunk_the_transactions_correctly() {
        let t = Transaction {
            version: 2,
            timestamp: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
            ..Transaction::default()
        };
        // Create a vec with 10 transactions.
        let transactions = vec![t.clone(); 10];
        assert!(t.encoded_len() == 6);
        let chunked_transactions = chunk_transactions(transactions, 20);
        assert_eq!(chunked_transactions.len(), 4);
        let total_count = chunked_transactions
            .iter()
            .map(|chunk| chunk.len())
            .sum::<usize>();
        assert!(total_count == 10);
    }
}
