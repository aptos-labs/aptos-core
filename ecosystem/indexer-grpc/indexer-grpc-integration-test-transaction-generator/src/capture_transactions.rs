// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_protos::indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest};
use std::path::PathBuf;

const LOCAL_INDEXER_GRPC_URL: &str = "http://localhost:50051";
const TRANSACTION_FILE_PREFIX: &str = "txn_";
const TRANSACTION_STREAM_TIMEOUT_IN_SECS: u64 = 30;

/// Captures the transactions with the given versions and writes them to the output directory.
/// It requires the indexer gRPC service to be running locally.
/// The transactions are written to the output directory in JSON format.
pub async fn capture_transactions(
    transaction_versions: Vec<u64>,
    output_dir: PathBuf,
) -> anyhow::Result<()> {
    if transaction_versions.is_empty() {
        return Err(anyhow::anyhow!("No transaction versions provided."));
    }
    // Make sure the transactions are sorted.
    let mut transaction_versions = transaction_versions;
    transaction_versions.sort();
    // Build the request.
    let first_version = *transaction_versions.first().unwrap();
    let last_version = *transaction_versions.last().unwrap();
    let transactions_count = last_version - first_version + 1;
    let request = tonic::Request::new(aptos_protos::indexer::v1::GetTransactionsRequest {
        starting_version: Some(first_version),
        transactions_count: Some(transactions_count),
        ..GetTransactionsRequest::default()
    });

    // Create a client and send the request.
    let mut client = RawDataClient::connect(LOCAL_INDEXER_GRPC_URL).await?;
    let response = client.get_transactions(request).await?;
    let mut response = response.into_inner();
    let mut transactions = Vec::new();
    if (tokio::time::timeout(
        std::time::Duration::from_secs(TRANSACTION_STREAM_TIMEOUT_IN_SECS),
        async {
            while let Ok(Some(resp_item)) = response.message().await {
                for transaction in resp_item.transactions {
                    transactions.push(transaction);
                }
            }
        },
    )
    .await)
        .is_err()
    {
        return Err(anyhow::anyhow!("Timeout while fetching transactions."));
    }

    // Filter the transactions.
    let transactions = transactions
        .into_iter()
        .filter(|transaction| transaction_versions.contains(&transaction.version))
        .collect::<Vec<_>>();

    // If the number of transactions fetched is not equal to the number of requested transactions, return an error.
    if transactions.len() != transaction_versions.len() {
        return Err(anyhow::anyhow!(
            "Failed to fetch all requested transactions."
        ));
    }

    // Change the transactions versions to be 1, 2, 3... This is to make sure diff is stable.
    let transactions = transactions
        .into_iter()
        .enumerate()
        .map(|(i, mut transaction)| {
            transaction.version = i as u64 + 1;
            transaction
        })
        .collect::<Vec<_>>();

    // If the output directory does not exist, create it.
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
    }
    // Write the transactions to the output directory.
    for transaction in transactions {
        let file_name = format!("{}{}.json", TRANSACTION_FILE_PREFIX, transaction.version);
        let file_path = output_dir.join(file_name);
        std::fs::write(file_path, serde_json::to_string_pretty(&transaction)?)
            .expect("Failed to write transaction to file.");
    }

    Ok(())
}
