// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::mock::MockNetwork;
use aptos_config::{
    config::AptosDataClientConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_crypto::HashValue;
use aptos_storage_service_server::network::NetworkRequest;
use aptos_storage_service_types::{
    requests::DataRequest,
    responses::{
        CompleteDataRange, DataResponse, DataSummary, ProtocolMetadata, StorageServerSummary,
        StorageServiceResponse,
    },
};
use aptos_time_service::MockTimeService;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::{TransactionListWithProof, Version},
};
use claims::assert_matches;
use std::time::Duration;

/// Adds a peer to the mock network and returns the peer and network id
pub fn add_peer_to_network(
    poll_priority_peers: bool,
    mock_network: &mut MockNetwork,
) -> (PeerNetworkId, NetworkId) {
    let peer = mock_network.add_peer(poll_priority_peers);
    (peer, peer.network_id())
}

/// Advances time by at least the polling loop interval
pub async fn advance_polling_timer(
    mock_time: &mut MockTimeService,
    data_client_config: &AptosDataClientConfig,
) {
    let poll_loop_interval_ms = data_client_config.data_poller_config.poll_loop_interval_ms;
    for _ in 0..10 {
        mock_time
            .advance_async(Duration::from_millis(poll_loop_interval_ms))
            .await;
    }
}

/// Creates a test ledger info at the given version and timestamp
fn create_ledger_info(version: Version, timestamp_usecs: u64) -> LedgerInfoWithSignatures {
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(
                0,
                0,
                HashValue::zero(),
                HashValue::zero(),
                version,
                timestamp_usecs,
                None,
            ),
            HashValue::zero(),
        ),
        AggregateSignature::empty(),
    )
}

/// Creates a test storage server summary at the given version and timestamp
pub fn create_storage_summary(version: Version) -> StorageServerSummary {
    create_storage_summary_with_timestamp(version, 0)
}

/// Creates a test storage server summary at the given version and timestamp
pub fn create_storage_summary_with_timestamp(
    version: Version,
    timestamp_usecs: u64,
) -> StorageServerSummary {
    StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: 1000,
            max_state_chunk_size: 1000,
            max_transaction_chunk_size: 1000,
            max_transaction_output_chunk_size: 1000,
        },
        data_summary: DataSummary {
            synced_ledger_info: Some(create_ledger_info(version, timestamp_usecs)),
            epoch_ending_ledger_infos: None,
            transactions: Some(CompleteDataRange::new(0, version).unwrap()),
            transaction_outputs: Some(CompleteDataRange::new(0, version).unwrap()),
            states: None,
        },
    }
}

/// Returns the next network request for the given network id
pub async fn get_network_request(
    mock_network: &mut MockNetwork,
    network_id: NetworkId,
) -> NetworkRequest {
    mock_network.next_request(network_id).await.unwrap()
}

/// Handles a storage server summary request by sending the specified storage summary
pub fn handle_storage_summary_request(
    network_request: NetworkRequest,
    storage_server_summary: StorageServerSummary,
) {
    // Verify the request type is valid
    assert_matches!(
        network_request.storage_service_request.data_request,
        DataRequest::GetStorageServerSummary
    );

    // Send the data response
    let data_response = DataResponse::StorageServerSummary(storage_server_summary.clone());
    network_request
        .response_sender
        .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));
}

/// Handles a transactions request by sending an empty transaction list with proof
pub fn handle_transactions_request(network_request: NetworkRequest, use_compression: bool) {
    // Verify the request type is valid
    assert_matches!(
        network_request.storage_service_request.data_request,
        DataRequest::GetTransactionsWithProof(_)
    );

    // Send the data response
    let data_response = DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
    network_request
        .response_sender
        .send(Ok(StorageServiceResponse::new(
            data_response,
            use_compression,
        )
        .unwrap()));
}
