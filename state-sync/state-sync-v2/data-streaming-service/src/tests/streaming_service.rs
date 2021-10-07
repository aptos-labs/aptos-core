// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    streaming_client::{
        new_streaming_service_client_listener_pair, DataStreamingClient, PayloadRefetchReason,
        StreamingServiceClient,
    },
    streaming_service::DataStreamingService,
};
use async_trait::async_trait;
use claim::{assert_matches, assert_ok};
use diem_crypto::HashValue;
use diem_data_client::{
    AdvertisedData, DataClientPayload, DataClientResponse, DiemDataClient, GlobalDataSummary,
    OptimalChunkSizes, ResponseError,
};
use futures::executor::block_on;
use storage_service_types::CompleteDataRange;
use tokio::runtime::{Builder, Runtime};

#[test]
fn test_epoch_ending_stream() {
    // Create a new streaming client and service
    let (streaming_client, streaming_service) = create_new_streaming_client_and_service();
    let _runtime = spawn_service_on_runtime(streaming_service);

    // Request an epoch ending stream and verify we get a data stream listener
    let streaming_client_clone = streaming_client.clone();
    block_on(async move {
        let result = streaming_client_clone
            .get_all_epoch_ending_ledger_infos(100)
            .await;
        assert_ok!(result);
    });

    // Try to request a stream where epoch data is missing (all data was pruned at version 100)
    let streaming_client_clone = streaming_client.clone();
    block_on(async move {
        let result = streaming_client_clone
            .get_all_epoch_ending_ledger_infos(0)
            .await;
        assert_matches!(result, Err(Error::DataIsUnavailable(_)));
    });

    // Try to request a stream where epoch data is missing (we are higher than anything advertised)
    block_on(async move {
        let result = streaming_client
            .get_all_epoch_ending_ledger_infos(10000)
            .await;
        assert_matches!(result, Err(Error::DataIsUnavailable(_)));
    });
}

#[test]
fn test_unsupported_streams() {
    // Create a new streaming client and service
    let (streaming_client, streaming_service) = create_new_streaming_client_and_service();
    let _runtime = spawn_service_on_runtime(streaming_service);

    // Request an account stream and verify it's unsupported
    let streaming_client_clone = streaming_client.clone();
    block_on(async move {
        let result = streaming_client_clone.get_all_accounts(0).await;
        assert_matches!(result, Err(Error::UnsupportedRequestEncountered(_)));
    });

    // Request a transaction stream and verify it's unsupported
    let streaming_client_clone = streaming_client.clone();
    block_on(async move {
        let result = streaming_client_clone
            .get_all_transactions(0, 100, 200, true)
            .await;
        assert_matches!(result, Err(Error::UnsupportedRequestEncountered(_)));
    });

    // Request a transaction output stream and verify it's unsupported
    let streaming_client_clone = streaming_client.clone();
    block_on(async move {
        let result = streaming_client_clone
            .get_all_transaction_outputs(0, 100, 200)
            .await;
        assert_matches!(result, Err(Error::UnsupportedRequestEncountered(_)));
    });

    // Request a continuous transaction stream and verify it's unsupported
    let streaming_client_clone = streaming_client.clone();
    block_on(async move {
        let result = streaming_client_clone
            .continuously_stream_transactions(0, 0, true)
            .await;
        assert_matches!(result, Err(Error::UnsupportedRequestEncountered(_)));
    });

    // Request a continuous transaction output stream and verify it's unsupported
    let streaming_client_clone = streaming_client.clone();
    block_on(async move {
        let result = streaming_client_clone
            .continuously_stream_transaction_outputs(0, 0)
            .await;
        assert_matches!(result, Err(Error::UnsupportedRequestEncountered(_)));
    });

    // Request a refetch notification payload stream and verify it's unsupported
    block_on(async move {
        let result = streaming_client
            .refetch_notification_payload(0, PayloadRefetchReason::InvalidPayloadData)
            .await;
        assert_matches!(result, Err(Error::UnsupportedRequestEncountered(_)));
    });
}

fn create_new_streaming_client_and_service() -> (
    StreamingServiceClient,
    DataStreamingService<MockDiemDataClient>,
) {
    // Create a new streaming client and listener
    let (streaming_client, streaming_service_listener) =
        new_streaming_service_client_listener_pair();

    // Create the streaming service and connect it to the listener
    let diem_data_client = MockDiemDataClient {};
    let streaming_service = DataStreamingService::new(diem_data_client, streaming_service_listener);

    (streaming_client, streaming_service)
}

fn spawn_service_on_runtime(
    streaming_service: DataStreamingService<MockDiemDataClient>,
) -> Runtime {
    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    runtime.spawn(streaming_service.start_service());
    runtime
}

/// A simple mock of the Diem Data Client (used for tests).
struct MockDiemDataClient {}

#[async_trait]
impl DiemDataClient for MockDiemDataClient {
    async fn get_account_states_with_proof(
        &self,
        _version: u64,
        _start_index: HashValue,
        _end_index: HashValue,
    ) -> Result<DataClientResponse, diem_data_client::Error> {
        unimplemented!();
    }

    async fn get_epoch_ending_ledger_infos(
        &self,
        _start_epoch: u64,
        _end_epoch: u64,
    ) -> Result<DataClientResponse, diem_data_client::Error> {
        unimplemented!();
    }

    fn get_global_data_summary(&self) -> Result<DataClientResponse, diem_data_client::Error> {
        // Return a global data summary containing only epochs 100 -> 1000 (inclusive)
        let advertised_data = AdvertisedData {
            account_states: vec![],
            epoch_ending_ledger_infos: vec![CompleteDataRange::new(100, 1000)],
            synced_ledger_infos: vec![],
            transactions: vec![],
            transaction_outputs: vec![],
        };
        let optimal_chunk_sizes = OptimalChunkSizes {
            account_states_chunk_size: 0,
            epoch_chunk_size: 0,
            transaction_chunk_size: 0,
            transaction_output_chunk_size: 0,
        };

        Ok(DataClientResponse {
            response_id: 0,
            response_payload: DataClientPayload::GlobalDataSummary(GlobalDataSummary {
                advertised_data,
                optimal_chunk_sizes,
            }),
        })
    }

    async fn get_number_of_account_states(
        &self,
        _version: u64,
    ) -> Result<DataClientResponse, diem_data_client::Error> {
        unimplemented!();
    }

    async fn get_transaction_outputs_with_proof(
        &self,
        _proof_version: u64,
        _start_version: u64,
        _end_version: u64,
    ) -> Result<DataClientResponse, diem_data_client::Error> {
        unimplemented!();
    }

    async fn get_transactions_with_proof(
        &self,
        _proof_version: u64,
        _start_version: u64,
        _end_version: u64,
        _include_events: bool,
    ) -> Result<DataClientResponse, diem_data_client::Error> {
        unimplemented!();
    }

    async fn notify_bad_response(
        &self,
        _response_id: u64,
        _response_error: ResponseError,
    ) -> Result<(), diem_data_client::Error> {
        unimplemented!();
    }
}
