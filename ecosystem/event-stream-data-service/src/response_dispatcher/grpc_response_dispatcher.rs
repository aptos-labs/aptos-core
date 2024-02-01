// Copyright © Aptos Foundation

use crate::response_dispatcher::ResponseDispatcher;
use aptos_indexer_grpc_data_access::{
    access_trait::{StorageReadError, StorageReadStatus, StorageTransactionRead},
    StorageClient,
};
use aptos_indexer_grpc_utils::{chunk_transactions, constants::MESSAGE_SIZE_LIMIT};
use aptos_logger::prelude::{sample, SampleRate};
use aptos_protos::indexer::v1::TransactionsResponse;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tonic::Status;

// The server will retry to send the response to the client and give up after RESPONSE_CHANNEL_SEND_TIMEOUT.
// This is to prevent the server from being occupied by a slow client.
const RESPONSE_CHANNEL_SEND_TIMEOUT: Duration = Duration::from_secs(120);
// Number of retries for fetching responses from upstream.
const FETCH_RETRY_COUNT: usize = 100;
const RETRY_BACKOFF_IN_MS: u64 = 500;
const NOT_AVAILABLE_RETRY_BACKOFF_IN_MS: u64 = 10;
const WAIT_TIME_BEFORE_CLOUSING_IN_MS: u64 = 60_000;
const RESPONSE_DISPATCH_NAME: &str = "GrpcResponseDispatcher";

pub struct GrpcResponseDispatcher {
    next_version_to_process: u64,
    transaction_count: Option<u64>,
    sender: Sender<Result<TransactionsResponse, Status>>,
    storages: Vec<StorageClient>,
    sender_capacity: usize,
}

impl GrpcResponseDispatcher {
    // Fetches the next batch of responses from storage.
    // This is a stateless function that only fetches from storage based on current state.
    async fn fetch_from_storages(&self) -> Result<Vec<TransactionsResponse>, StorageReadError> {
        if let Some(transaction_count) = self.transaction_count {
            if transaction_count == 0 {
                return Ok(vec![]);
            }
        }
        // Loop to wait for the next storage to be available.
        let mut previous_storage_not_found = false;
        loop {
            if self.sender.is_closed() {
                return Err(StorageReadError::PermenantError(
                    RESPONSE_DISPATCH_NAME,
                    anyhow::anyhow!("Sender is closed."),
                ));
            }
            for storage in self.storages.as_slice() {
                let metadata = storage.get_metadata().await?;
                match storage
                    .get_transactions(self.next_version_to_process, None)
                    .await
                {
                    Ok(StorageReadStatus::Ok(transactions)) => {
                        let responses = chunk_transactions(transactions, MESSAGE_SIZE_LIMIT);
                        return Ok(responses
                            .into_iter()
                            .map(|transactions| TransactionsResponse {
                                transactions,
                                chain_id: Some(metadata.chain_id),
                            })
                            .collect());
                    },
                    Ok(StorageReadStatus::NotAvailableYet) => {
                        // This is fatal; it means previous storage evicts the data before the current storage has it.
                        if previous_storage_not_found {
                            return Err(StorageReadError::PermenantError(
                                RESPONSE_DISPATCH_NAME,
                                anyhow::anyhow!("Gap detected between storages."),
                            ));
                        }
                        // If the storage is not available yet, retry the storages.
                        tokio::time::sleep(Duration::from_millis(
                            NOT_AVAILABLE_RETRY_BACKOFF_IN_MS,
                        ))
                        .await;
                        break;
                    },
                    Ok(StorageReadStatus::NotFound) => {
                        // Continue to the next storage.
                        previous_storage_not_found = true;
                        continue;
                    },
                    Err(e) => {
                        return Err(e);
                    },
                }
            }

            if previous_storage_not_found {
                return Err(StorageReadError::PermenantError(
                    RESPONSE_DISPATCH_NAME,
                    anyhow::anyhow!("Gap detected between storages."),
                ));
            }
        }
    }

    // Based on the response from fetch_from_storages, verify and dispatch the response, and update the state.
    async fn fetch_internal(&mut self) -> Result<Vec<TransactionsResponse>, StorageReadError> {
        // TODO: add retry to TransientError.
        let responses = self.fetch_from_storages().await?;
        // Verify no empty response.
        if responses.iter().any(|v| v.transactions.is_empty()) {
            return Err(StorageReadError::TransientError(
                RESPONSE_DISPATCH_NAME,
                anyhow::anyhow!("Empty responses from storages."),
            ));
        }

        // Verify responses are consecutive and sequential.
        let mut version = self.next_version_to_process;
        for response in responses.iter() {
            for transaction in response.transactions.iter() {
                if transaction.version != version {
                    return Err(StorageReadError::TransientError(
                        RESPONSE_DISPATCH_NAME,
                        anyhow::anyhow!("Version mismatch in response."),
                    ));
                }
                // move to the next version.
                version += 1;
            }
        }
        let mut processed_responses = vec![];
        if let Some(transaction_count) = self.transaction_count {
            // If transactions_count is specified, truncate if necessary.
            let mut current_transaction_count = 0;
            for response in responses.into_iter() {
                if current_transaction_count == transaction_count {
                    break;
                }
                let current_response_size = response.transactions.len() as u64;
                if current_transaction_count + current_response_size > transaction_count {
                    let remaining_transaction_count = transaction_count - current_transaction_count;
                    let truncated_transactions = response
                        .transactions
                        .into_iter()
                        .take(remaining_transaction_count as usize)
                        .collect();
                    processed_responses.push(TransactionsResponse {
                        transactions: truncated_transactions,
                        chain_id: response.chain_id,
                    });
                    current_transaction_count += remaining_transaction_count;
                } else {
                    processed_responses.push(response);
                    current_transaction_count += current_response_size;
                }
            }
            self.transaction_count = Some(transaction_count - current_transaction_count);
        } else {
            // If not, continue to fetch.
            processed_responses = responses;
        }
        let processed_transactions_count = processed_responses
            .iter()
            .map(|v| v.transactions.len())
            .sum::<usize>() as u64;
        self.next_version_to_process += processed_transactions_count;
        Ok(processed_responses)
    }
}

#[async_trait::async_trait]
impl ResponseDispatcher for GrpcResponseDispatcher {
    fn new(
        starting_version: u64,
        transaction_count: Option<u64>,
        sender: Sender<Result<TransactionsResponse, Status>>,
        storages: &[StorageClient],
    ) -> Self {
        let sender_capacity = sender.capacity();
        Self {
            next_version_to_process: starting_version,
            transaction_count,
            sender,
            sender_capacity,
            storages: storages.to_vec(),
        }
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.fetch_with_retries().await {
                Ok(responses) => {
                    if responses.is_empty() {
                        break;
                    }
                    for response in responses {
                        self.dispatch(Ok(response)).await?;
                    }
                },
                Err(status) => {
                    self.dispatch(Err(status)).await?;
                    anyhow::bail!("Failed to fetch transactions from storages.");
                },
            }
        }
        if self.transaction_count.is_some() {
            let start_time = std::time::Instant::now();
            loop {
                if start_time.elapsed().as_millis() > WAIT_TIME_BEFORE_CLOUSING_IN_MS as u128 {
                    break;
                }
                if self.sender.capacity() == self.sender_capacity {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
        Ok(())
    }

    async fn fetch_with_retries(&mut self) -> anyhow::Result<Vec<TransactionsResponse>, Status> {
        for _ in 0..FETCH_RETRY_COUNT {
            match self.fetch_internal().await {
                Ok(responses) => {
                    return Ok(responses);
                },
                Err(StorageReadError::TransientError(s, _e)) => {
                    tracing::warn!("Failed to fetch transactions from storage: {:#}", s);
                    tokio::time::sleep(Duration::from_millis(RETRY_BACKOFF_IN_MS)).await;
                    continue;
                },
                Err(StorageReadError::PermenantError(s, _e)) => Err(Status::internal(format!(
                    "Failed to fetch transactions from storages, {:}",
                    s
                )))?,
            }
        }
        Err(Status::internal(
            "Failed to fetch transactions from storages.",
        ))
    }

    async fn dispatch(
        &mut self,
        response: Result<TransactionsResponse, Status>,
    ) -> anyhow::Result<()> {
        let start_time = std::time::Instant::now();
        match self
            .sender
            .send_timeout(response, RESPONSE_CHANNEL_SEND_TIMEOUT)
            .await
        {
            Ok(_) => {},
            Err(e) => {
                tracing::warn!("Failed to send response to downstream: {:#}", e);
                return Err(anyhow::anyhow!("Failed to send response to downstream."));
            },
        };
        sample!(
            SampleRate::Duration(Duration::from_secs(60)),
            tracing::info!(
                "[GrpcResponseDispatch] response waiting time in seconds: {}",
                start_time.elapsed().as_secs_f64()
            );
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_indexer_grpc_data_access::MockStorageClient;
    use aptos_protos::transaction::v1::Transaction;
    fn create_transactions(starting_version: u64, size: usize) -> Vec<Transaction> {
        let mut transactions = vec![];
        for i in 0..size {
            transactions.push(Transaction {
                version: starting_version + i as u64,
                ..Default::default()
            });
        }
        transactions
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_finite_stream() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            let first_storage_transactions = create_transactions(20, 100);
            let second_storage_transactions = create_transactions(10, 20);
            let third_storage_transactions = create_transactions(0, 15);
            let storages = vec![
                StorageClient::MockClient(MockStorageClient::new(1, first_storage_transactions)),
                StorageClient::MockClient(MockStorageClient::new(2, second_storage_transactions)),
                StorageClient::MockClient(MockStorageClient::new(3, third_storage_transactions)),
            ];
            let mut dispatcher =
                GrpcResponseDispatcher::new(0, Some(40), sender, storages.as_slice());
            let run_result = dispatcher.run().await;
            assert!(run_result.is_ok());
        });

        let mut transactions = vec![];
        while let Some(response) = receiver.recv().await {
            for transaction in response.unwrap().transactions {
                transactions.push(transaction);
            }
        }
        assert_eq!(transactions.len(), 40);
        for (current_version, t) in transactions.into_iter().enumerate() {
            assert_eq!(t.version, current_version as u64);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_storages_gap() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            let first_storage_transactions = create_transactions(30, 100);
            let second_storage_transactions = create_transactions(10, 10);
            let storages = vec![
                StorageClient::MockClient(MockStorageClient::new(1, first_storage_transactions)),
                StorageClient::MockClient(MockStorageClient::new(2, second_storage_transactions)),
            ];
            let mut dispatcher =
                GrpcResponseDispatcher::new(15, Some(30), sender, storages.as_slice());
            let run_result = dispatcher.run().await;
            assert!(run_result.is_err());
        });

        let first_response = receiver.recv().await.unwrap();
        assert!(first_response.is_ok());
        let transactions_response = first_response.unwrap();
        assert!(transactions_response.transactions.len() == 5);
        let second_response = receiver.recv().await.unwrap();
        // Gap is detected.
        assert!(second_response.is_err());
    }

    // This test is to make sure dispatch doesn't leak memory.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_infinite_stream_with_client_closure() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
        let task_result = tokio::spawn(async move {
            let first_storage_transactions = create_transactions(20, 20);
            let second_storage_transactions = create_transactions(10, 30);
            let third_storage_transactions = create_transactions(0, 15);
            let storages = vec![
                StorageClient::MockClient(MockStorageClient::new(1, first_storage_transactions)),
                StorageClient::MockClient(MockStorageClient::new(2, second_storage_transactions)),
                StorageClient::MockClient(MockStorageClient::new(3, third_storage_transactions)),
            ];
            let mut dispatcher = GrpcResponseDispatcher::new(0, None, sender, storages.as_slice());
            dispatcher.run().await
        });
        // Let the dispatcher run for 1 second.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let first_peek = receiver.try_recv();
        // transactions 0 - 15
        assert!(first_peek.is_ok());
        let first_response = first_peek.unwrap();
        assert!(first_response.is_ok());
        let transactions_response = first_response.unwrap();
        assert!(transactions_response.transactions.len() == 15);
        let second_peek = receiver.try_recv();
        // transactions 15 - 40
        assert!(second_peek.is_ok());
        let second_response = second_peek.unwrap();
        assert!(second_response.is_ok());
        let transactions_response = second_response.unwrap();
        assert!(transactions_response.transactions.len() == 25);
        let third_peek = receiver.try_recv();
        match third_peek {
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {},
            _ => unreachable!("This is not possible."),
        }
        // Drop the receiver to close the channel.
        drop(receiver);
        let task_result = task_result.await;

        // The task should finish successfully.
        assert!(task_result.is_ok());
        let task_result = task_result.unwrap();
        // The dispatcher thread should exit with error.
        assert!(task_result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_not_found_in_all_storages() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            let first_storage_transactions = create_transactions(20, 100);
            let storages = vec![StorageClient::MockClient(MockStorageClient::new(
                1,
                first_storage_transactions,
            ))];
            let mut dispatcher =
                GrpcResponseDispatcher::new(0, Some(40), sender, storages.as_slice());
            let run_result = dispatcher.run().await;
            assert!(run_result.is_err());
        });

        let first_response = receiver.recv().await.unwrap();
        assert!(first_response.is_err());
    }
}
