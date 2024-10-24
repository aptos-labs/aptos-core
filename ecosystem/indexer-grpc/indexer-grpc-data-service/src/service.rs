// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_indexer_grpc_utils::{
    chunk_transactions,
    compression_util::{CacheEntry, StorageFormat},
    constants::MESSAGE_SIZE_LIMIT,
    file_store_operator::FileStoreOperator,
};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    indexer::v1::{raw_data_server::RawData, GetTransactionsRequest, TransactionsResponse},
    transaction::v1::Transaction,
};
use futures::Stream;
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::sync::mpsc::{channel, error::SendTimeoutError, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{error, warn};

type ResponseStream = Pin<Box<dyn Stream<Item = Result<TransactionsResponse, Status>> + Send>>;

const MOVING_AVERAGE_WINDOW_SIZE: u64 = 10_000;
// When trying to fetch beyond the current head of cache, the server will retry after this duration.
const AHEAD_OF_CACHE_RETRY_SLEEP_DURATION_MS: u64 = 50;
// When error happens when fetching data from cache and file store, the server will retry after this duration.
const TRANSIENT_DATA_ERROR_RETRY_SLEEP_DURATION_MS: u64 = 1000;
// This is the time we wait for the file store to be ready. It should only be
// kicked off when there's no metadata in the file store.
const FILE_STORE_METADATA_WAIT_MS: u64 = 2000;

// The server will retry to send the response to the client and give up after RESPONSE_CHANNEL_SEND_TIMEOUT.
// This is to prevent the server from being occupied by a slow client.
const RESPONSE_CHANNEL_SEND_TIMEOUT: Duration = Duration::from_secs(120);

// Number of times to retry fetching a given txn block from the stores
pub const NUM_DATA_FETCH_RETRIES: u8 = 5;

// Max number of tasks to reach out to TXN stores with
const MAX_FETCH_TASKS_PER_REQUEST: u64 = 5;
// The number of transactions we store per txn block; this is used to determine max num of tasks
const TRANSACTIONS_PER_STORAGE_BLOCK: u64 = 1000;

pub struct RawDataServerWrapper {
    handler_tx: Sender<(
        GetTransactionsRequest,
        Sender<Result<TransactionsResponse, Status>>,
    )>,
    pub data_service_response_channel_size: usize,
}

impl RawDataServerWrapper {
    pub fn new(
        handler_tx: Sender<(
            GetTransactionsRequest,
            Sender<Result<TransactionsResponse, Status>>,
        )>,
        data_service_response_channel_size: usize,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            handler_tx,
            data_service_response_channel_size,
        })
    }
}

#[tonic::async_trait]
impl RawData for RawDataServerWrapper {
    type GetTransactionsStream = ResponseStream;

    async fn get_transactions(
        &self,
        req: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        // Get request identity. The request is already authenticated by the interceptor.
        let request = req.into_inner();

        tracing::info!("Request: {request:?}.");

        // Response channel to stream the data to the client.
        let (tx, rx) = channel(self.data_service_response_channel_size);
        self.handler_tx.send((request, tx)).await.unwrap();

        let output_stream = ReceiverStream::new(rx);
        let response = Response::new(Box::pin(output_stream) as Self::GetTransactionsStream);

        Ok(response)
    }
}

fn get_transactions_responses_builder(
    transactions: Vec<Transaction>,
    chain_id: u32,
) -> Vec<TransactionsResponse> {
    let chunks = chunk_transactions(transactions, MESSAGE_SIZE_LIMIT);
    let responses = chunks
        .into_iter()
        .map(|chunk| TransactionsResponse {
            chain_id: Some(chain_id as u64),
            transactions: chunk,
        })
        .collect();
    responses
}

// This is a CPU bound operation, so we spawn_blocking
async fn deserialize_cached_transactions(
    transactions: Vec<Vec<u8>>,
    storage_format: StorageFormat,
) -> anyhow::Result<Vec<Transaction>> {
    let task = tokio::task::spawn_blocking(move || {
        transactions
            .into_iter()
            .map(|transaction| {
                let cache_entry = CacheEntry::new(transaction, storage_format);
                cache_entry.into_transaction()
            })
            .collect::<Vec<Transaction>>()
    })
    .await;
    task.context("Transaction bytes to CacheEntry deserialization task failed")
}

async fn channel_send_multiple_with_timeout(
    resp_items: Vec<TransactionsResponse>,
    tx: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
) -> Result<(), SendTimeoutError<Result<TransactionsResponse, Status>>> {
    for resp_item in resp_items {
        tx.send_timeout(
            Result::<TransactionsResponse, Status>::Ok(resp_item.clone()),
            RESPONSE_CHANNEL_SEND_TIMEOUT,
        )
        .await?;
    }

    Ok(())
}
