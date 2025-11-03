// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::AptosDataClient, error::Error, interface::AptosDataClientInterface,
    priority::PeerPriority, tests::mock::MockNetwork,
};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig, RoleType},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_crypto::HashValue;
use aptos_peer_monitoring_service_types::PeerMonitoringMetadata;
use aptos_storage_service_server::network::NetworkRequest;
use aptos_storage_service_types::{
    requests::{
        DataRequest, NewTransactionOutputsWithProofRequest,
        NewTransactionsOrOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StorageServiceRequest, SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        SubscriptionStreamMetadata,
    },
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
use maplit::hashset;
use rand::{rngs::OsRng, Rng};
use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    time::Duration,
};
use tokio::time::timeout;

// Useful test constants
pub const NUM_SELECTION_ITERATIONS: u64 = 15_000;

/// Adds a peer to the mock network and returns the peer and network id
pub fn add_peer_to_network(
    peer_priority: PeerPriority,
    mock_network: &mut MockNetwork,
) -> (PeerNetworkId, NetworkId) {
    let peer = mock_network.add_peer(peer_priority);
    (peer, peer.network_id())
}

/// Adds several peers to the mock network and returns the set of peers
pub fn add_several_peers(
    mock_network: &mut MockNetwork,
    num_peers: u64,
    peer_priority: PeerPriority,
) -> HashSet<PeerNetworkId> {
    let mut peers = hashset![];

    // Add the peers
    for _ in 0..num_peers {
        let peer = mock_network.add_peer(peer_priority);
        peers.insert(peer);
    }

    peers
}

/// Adds several peers with metadata to the mock network and returns the set of peers
pub fn add_several_peers_with_metadata(
    mock_network: &mut MockNetwork,
    client: &AptosDataClient,
    num_peers: u64,
    min_validator_distance: u64,
    max_validator_distance: u64,
    peer_priority: PeerPriority,
) -> HashSet<PeerNetworkId> {
    let mut peers = hashset![];

    // Add the peers and metadata
    for _ in 0..num_peers {
        // Add a peer
        let peer = mock_network.add_peer(peer_priority);
        peers.insert(peer);

        // Generate a random distance for the peer and update the peer's distance metadata
        let distance_from_validator =
            rand::thread_rng().gen_range(min_validator_distance..=max_validator_distance);
        update_distance_metadata(client, peer, distance_from_validator as u64);
    }

    peers
}

/// Advances time by at least the polling loop interval
pub async fn advance_polling_timer(
    mock_time: &mut MockTimeService,
    data_client_config: &AptosDataClientConfig,
) {
    let poll_loop_interval_ms = data_client_config.data_poller_config.poll_loop_interval_ms;
    for _ in 0..10 {
        tokio::task::yield_now().await;
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

/// Creates and returns a base config for a fullnode
pub fn create_fullnode_base_config() -> BaseConfig {
    BaseConfig {
        role: RoleType::FullNode,
        ..Default::default()
    }
}

/// Creates and returns a base config for a validator node
pub fn create_validator_base_config() -> BaseConfig {
    BaseConfig {
        role: RoleType::Validator,
        ..Default::default()
    }
}

/// Disconnect all peers from the network
pub fn disconnect_all_peers(mock_network: &mut MockNetwork, peers: &HashSet<PeerNetworkId>) {
    for peer in peers.iter() {
        mock_network.disconnect_peer(*peer);
    }
}

/// Enumerates all optimistic fetch request types
pub fn enumerate_optimistic_fetch_requests(
    known_version: u64,
    known_epoch: u64,
) -> Vec<DataRequest> {
    // Create all optimistic fetch requests
    let new_transactions_request =
        DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version,
            known_epoch,
            include_events: false,
        });
    let new_outputs_requests =
        DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
            known_version,
            known_epoch,
        });
    let new_transactions_or_outputs_request = DataRequest::GetNewTransactionsOrOutputsWithProof(
        NewTransactionsOrOutputsWithProofRequest {
            known_version,
            known_epoch,
            include_events: false,
            max_num_output_reductions: 0,
        },
    );

    // Return all optimistic fetch requests
    vec![
        new_transactions_request,
        new_outputs_requests,
        new_transactions_or_outputs_request,
    ]
}

/// Enumerates all subscription request types
pub fn enumerate_subscription_requests(known_version: u64, known_epoch: u64) -> Vec<DataRequest> {
    // Create all subscription requests
    let subscribe_transactions_request =
        DataRequest::SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
            subscription_stream_metadata: SubscriptionStreamMetadata {
                known_version_at_stream_start: known_version,
                known_epoch_at_stream_start: known_epoch,
                subscription_stream_id: 100,
            },
            subscription_stream_index: 0,
            include_events: false,
        });
    let subscribe_outputs_request = DataRequest::SubscribeTransactionOutputsWithProof(
        SubscribeTransactionOutputsWithProofRequest {
            subscription_stream_metadata: SubscriptionStreamMetadata {
                known_version_at_stream_start: known_version,
                known_epoch_at_stream_start: known_epoch,
                subscription_stream_id: 200,
            },
            subscription_stream_index: 0,
        },
    );
    let subscribe_transactions_or_outputs_request =
        DataRequest::SubscribeTransactionsOrOutputsWithProof(
            SubscribeTransactionsOrOutputsWithProofRequest {
                subscription_stream_metadata: SubscriptionStreamMetadata {
                    known_version_at_stream_start: known_version,
                    known_epoch_at_stream_start: known_epoch,
                    subscription_stream_id: 300,
                },
                subscription_stream_index: 0,
                include_events: false,
                max_num_output_reductions: 0,
            },
        );

    // Return all subscription requests
    vec![
        subscribe_transactions_request,
        subscribe_outputs_request,
        subscribe_transactions_or_outputs_request,
    ]
}

/// Returns the next network request for the given network id
pub async fn get_network_request(
    mock_network: &mut MockNetwork,
    network_id: NetworkId,
) -> NetworkRequest {
    timeout(Duration::from_secs(10), async {
        mock_network.next_request(network_id).await.unwrap()
    })
    .await
    .expect("Failed to get network request! Timed out!")
}

/// Returns the peer distance from validators for the given peer
pub fn get_peer_distance_from_validators(
    mock_network: &mut MockNetwork,
    peer: PeerNetworkId,
) -> u64 {
    let peer_monitoring_metadata = get_peer_monitoring_metadata(mock_network, peer);
    peer_monitoring_metadata
        .latest_network_info_response
        .unwrap()
        .distance_from_validators
}

/// Returns the ping latency for the given peer
pub fn get_peer_ping_latency(mock_network: &mut MockNetwork, peer: PeerNetworkId) -> f64 {
    let peer_monitoring_metadata = get_peer_monitoring_metadata(mock_network, peer);
    peer_monitoring_metadata.average_ping_latency_secs.unwrap()
}

/// Returns the peer monitoring metadata for the given peer
pub fn get_peer_monitoring_metadata(
    mock_network: &mut MockNetwork,
    peer: PeerNetworkId,
) -> PeerMonitoringMetadata {
    // Get the peer metadata
    let peers_and_metadata = mock_network.get_peers_and_metadata();
    let peer_metadata = peers_and_metadata.get_metadata_for_peer(peer).unwrap();

    // Return the peer monitoring metadata
    peer_metadata.get_peer_monitoring_metadata().clone()
}

/// Returns the peer priority for polling based on if `poll_priority_peers` is true
pub fn get_peer_priority_for_polling(poll_priority_peers: bool) -> PeerPriority {
    if poll_priority_peers {
        PeerPriority::HighPriority
    } else {
        // Generate a random u64
        let random_number: u64 = OsRng.r#gen();

        // If the random number is even, return medium priority.
        // Otherwise, return low priority.
        if random_number % 2 == 0 {
            PeerPriority::MediumPriority
        } else {
            PeerPriority::LowPriority
        }
    }
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

/// Removes the distance metadata for the specified peer
pub fn remove_distance_metadata(client: &AptosDataClient, peer: PeerNetworkId) {
    // Get the peer monitoring metadata
    let peers_and_metadata = client.get_peers_and_metadata();
    let peer_metadata = peers_and_metadata.get_metadata_for_peer(peer).unwrap();
    let mut peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata().clone();

    // Remove the network info response (containing the distance metadata)
    peer_monitoring_metadata.latest_network_info_response = None;

    // Update the peer monitoring metadata
    peers_and_metadata
        .update_peer_monitoring_metadata(peer, peer_monitoring_metadata)
        .unwrap();
}

/// Removes the latency metadata for the specified peer
pub fn remove_latency_metadata(client: &AptosDataClient, peer: PeerNetworkId) {
    // Get the peer monitoring metadata
    let peers_and_metadata = client.get_peers_and_metadata();
    let peer_metadata = peers_and_metadata.get_metadata_for_peer(peer).unwrap();
    let mut peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata().clone();

    // Remove the latency metadata
    peer_monitoring_metadata.average_ping_latency_secs = None;

    // Update the peer monitoring metadata
    peers_and_metadata
        .update_peer_monitoring_metadata(peer, peer_monitoring_metadata)
        .unwrap();
}

/// Chooses peers to service the given request multiple times and
/// returns a map of the peers and their selection counts.
pub fn select_peers_multiple_times(
    client: &AptosDataClient,
    expected_num_peers_for_request: usize,
    storage_request: &StorageServiceRequest,
) -> HashMap<PeerNetworkId, i32> {
    let mut peers_and_selection_counts = HashMap::new();

    // Select the peers multiple times and collect the selection counts
    for _ in 0..NUM_SELECTION_ITERATIONS {
        // Select peers to service the request
        let selected_peers = client.choose_peers_for_request(storage_request).unwrap();
        assert_eq!(selected_peers.len(), expected_num_peers_for_request);

        // Update the peer selection counts
        for selected_peer in selected_peers {
            *peers_and_selection_counts.entry(selected_peer).or_insert(0) += 1;
        }
    }

    peers_and_selection_counts
}

/// Updates the distance metadata for the specified peer
pub fn update_distance_metadata(
    client: &AptosDataClient,
    peer: PeerNetworkId,
    distance_from_validators: u64,
) {
    // Get the peer monitoring metadata
    let peers_and_metadata = client.get_peers_and_metadata();
    let peer_metadata = peers_and_metadata.get_metadata_for_peer(peer).unwrap();
    let mut peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata().clone();

    // Update the distance metadata
    let mut latest_network_info_response = peer_monitoring_metadata
        .latest_network_info_response
        .unwrap();
    latest_network_info_response.distance_from_validators = distance_from_validators;
    peer_monitoring_metadata.latest_network_info_response = Some(latest_network_info_response);

    // Update the peer monitoring metadata
    peers_and_metadata
        .update_peer_monitoring_metadata(peer, peer_monitoring_metadata)
        .unwrap();
}

/// Updates the storage summaries for the given peers using the specified
/// version and timestamp.
pub fn update_storage_summaries_for_peers(
    client: &AptosDataClient,
    peers: &HashSet<PeerNetworkId>,
    known_version: u64,
    timestamp_usecs: u128,
) {
    for peer in peers.iter() {
        client.update_peer_storage_summary(
            *peer,
            create_storage_summary_with_timestamp(known_version, timestamp_usecs as u64),
        );
    }
}

/// Updates the subscription request ID in the given storage request
/// and returns the updated storage request.
pub fn update_subscription_request_id(
    storage_service_request: &StorageServiceRequest,
) -> StorageServiceRequest {
    let mut storage_service_request = storage_service_request.clone();

    // Update the subscription's request ID
    match &mut storage_service_request.data_request {
        DataRequest::SubscribeTransactionsWithProof(request) => {
            request.subscription_stream_metadata.subscription_stream_id += 1
        },
        DataRequest::SubscribeTransactionOutputsWithProof(request) => {
            request.subscription_stream_metadata.subscription_stream_id += 1
        },
        DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
            request.subscription_stream_metadata.subscription_stream_id += 1
        },
        _ => panic!(
            "Unexpected subscription request type! {:?}",
            storage_service_request
        ),
    }

    storage_service_request
}

/// Verifies the top 10% of selected peers are the lowest latency peers
pub fn verify_highest_peer_selection_latencies(
    mock_network: &mut MockNetwork,
    peers_and_selection_counts: &mut HashMap<PeerNetworkId, i32>,
) {
    // Build a max-heap of all peers by their selection counts
    let mut max_heap_selection_counts = build_selection_count_max_heap(peers_and_selection_counts);

    // Verify the top 10% of polled peers are the lowest latency peers
    let peers_to_verify = peers_and_selection_counts.len() / 10;
    let mut highest_seen_latency = 0.0;
    for _ in 0..peers_to_verify {
        // Get the peer
        let (_, peer) = max_heap_selection_counts.pop().unwrap();

        // Get the peer's ping latency
        let ping_latency = get_peer_ping_latency(mock_network, peer);

        // Verify that the ping latencies are increasing
        if ping_latency <= highest_seen_latency {
            // The ping latencies did not increase. This should only be
            // possible if the latencies are very close (i.e., within 10%).
            if (highest_seen_latency - ping_latency) > 0.1 {
                panic!("The ping latencies are not increasing! Are peers weighted by latency?");
            }
        }

        // Update the highest seen latency
        highest_seen_latency = ping_latency;
    }
}

/// Builds and returns a max-heap of all peers by their selection counts
pub fn build_selection_count_max_heap(
    peers_and_selection_counts: &HashMap<PeerNetworkId, i32>,
) -> BinaryHeap<(i32, PeerNetworkId)> {
    let mut max_heap_selection_counts = BinaryHeap::new();
    for (peer, selection_count) in peers_and_selection_counts.clone() {
        max_heap_selection_counts.push((selection_count, peer));
    }
    max_heap_selection_counts
}

/// Verifies that the selected peers for the given request match the expected peers
pub fn verify_selected_peers_match(
    client: &AptosDataClient,
    expected_peers: HashSet<PeerNetworkId>,
    request: &StorageServiceRequest,
) {
    let selected_peers = client.choose_peers_for_request(request).unwrap();
    assert_eq!(selected_peers, expected_peers);
}

/// Verifies that the given request is unserviceable
pub fn verify_request_is_unserviceable(
    client: &AptosDataClient,
    request: &StorageServiceRequest,
    no_connected_peers: bool,
) {
    let result = client.choose_peers_for_request(request);
    if no_connected_peers {
        assert_matches!(result, Err(Error::NoConnectedPeers(_)));
    } else {
        assert_matches!(result, Err(Error::DataIsUnavailable(_)));
    }
}

/// Selects peers to service the given request and verifies
/// that: (i) only a single peer is selected; and (ii) that
/// peer is contained in the broader set.
pub fn verify_selected_peer_from_set(
    client: &AptosDataClient,
    storage_request: &StorageServiceRequest,
    peers: &HashSet<PeerNetworkId>,
) {
    verify_selected_peers_from_set(client, storage_request, 1, peers)
}

/// Selects peers to service the given request and verifies that:
/// (i) the correct number of peers are selection; and
/// (ii) the peers are contained in the broader set.
pub fn verify_selected_peers_from_set(
    client: &AptosDataClient,
    storage_request: &StorageServiceRequest,
    num_expected_peers: usize,
    peers: &HashSet<PeerNetworkId>,
) {
    // Select peers to service the request
    let selected_peers = client.choose_peers_for_request(storage_request).unwrap();

    // Verify the selected peers
    assert_eq!(selected_peers.len(), num_expected_peers);
    assert!(peers.is_superset(&selected_peers));
}

/// Waits until the transaction range is advertised by the peers
pub async fn wait_for_transaction_advertisement(
    client: &AptosDataClient,
    mock_time: &mut MockTimeService,
    data_client_config: &AptosDataClientConfig,
    transaction_range: CompleteDataRange<u64>,
) {
    timeout(Duration::from_secs(10), async {
        loop {
            // Check if the transaction range is serviceable
            let advertised_data = client.get_global_data_summary().advertised_data;
            if advertised_data.transactions.contains(&transaction_range) {
                return; // The request range is serviceable
            }

            // Advance time so the poller sends a data summary request and gets the response
            advance_polling_timer(mock_time, data_client_config).await;

            // Sleep for a while before retrying
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("The transaction range is not advertised! Timed out!");
}
