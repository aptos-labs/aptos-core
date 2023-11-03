// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::grpc_response_stream::GrpcResponseStream;
use aptos_indexer_grpc_data_access::StorageClient;
use aptos_protos::indexer::v1::{
    raw_data_server::RawData, GetTransactionsRequest, TransactionsResponse,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::{pin::Pin, time::Duration};
use tonic::{Request, Response, Status};
use tracing::error;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<TransactionsResponse, Status>> + Send>>;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct RequestMetadata {
    pub processor_name: String,
    pub request_email: String,
    pub request_user_classification: String,
    pub request_api_key_name: String,
    // Token is no longer needed behind api gateway.
    #[deprecated]
    pub request_token: String,
}
#[allow(dead_code)]
const MOVING_AVERAGE_WINDOW_SIZE: u64 = 10_000;
// When trying to fetch beyond the current head of cache, the server will retry after this duration.
#[allow(dead_code)]
const AHEAD_OF_CACHE_RETRY_SLEEP_DURATION_MS: u64 = 50;
// When error happens when fetching data from cache and file store, the server will retry after this duration.
// TODO(larry): fix all errors treated as transient errors.
#[allow(dead_code)]
const TRANSIENT_DATA_ERROR_RETRY_SLEEP_DURATION_MS: u64 = 1000;

// The server will retry to send the response to the client and give up after RESPONSE_CHANNEL_SEND_TIMEOUT.
// This is to prevent the server from being occupied by a slow client.
#[allow(dead_code)]
const RESPONSE_CHANNEL_SEND_TIMEOUT: Duration = Duration::from_secs(120);
#[allow(dead_code)]
const SHORT_CONNECTION_DURATION_IN_SECS: u64 = 10;
#[allow(dead_code)]
const REQUEST_HEADER_APTOS_EMAIL_HEADER: &str = "x-aptos-email";
#[allow(dead_code)]
const REQUEST_HEADER_APTOS_USER_CLASSIFICATION_HEADER: &str = "x-aptos-user-classification";
#[allow(dead_code)]
const REQUEST_HEADER_APTOS_API_KEY_NAME: &str = "x-aptos-api-key-name";

pub struct RawDataServerWrapper {
    pub storages: Vec<StorageClient>,
    pub data_service_response_channel_size: usize,
}

impl RawDataServerWrapper {
    pub fn new(
        storages: &[StorageClient],
        data_service_response_channel_size: usize,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            data_service_response_channel_size,
            storages: storages.to_vec(),
        })
    }
}

/// RawDataServerWrapper handles the get transactions requests from cache and file store.
#[tonic::async_trait]
impl RawData for RawDataServerWrapper {
    type GetTransactionsStream = ResponseStream;

    /// GetTransactionsStream is a streaming GRPC endpoint:
    /// 1. Fetches data from cache and file store.
    ///    1.1. If the data is beyond the current head of cache, retry after a short sleep.
    ///    1.2. If the data is not in cache, fetch the data from file store.
    ///    1.3. If the data is not in file store, stream connection will break.
    ///    1.4  If error happens, retry after a short sleep.
    /// 2. Push data into channel to stream to the client.
    ///    2.1. If the channel is full, do not fetch and retry after a short sleep.
    async fn get_transactions(
        &self,
        req: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        let req = req.into_inner();
        let starting_version = req.starting_version.unwrap_or(0);
        let transactions_count = req.transactions_count;
        let grpc_response_stream = match GrpcResponseStream::new(
            starting_version,
            transactions_count,
            Some(self.data_service_response_channel_size),
            self.storages.as_slice(),
        ) {
            Ok(grpc_response_stream) => grpc_response_stream,
            Err(e) => {
                error!("Failed to create response stream: {}", e);
                return Err(Status::internal("Failed to create response stream"));
            },
        };
        Ok(Response::new(
            Box::pin(grpc_response_stream) as Self::GetTransactionsStream
        ))
    }
}
