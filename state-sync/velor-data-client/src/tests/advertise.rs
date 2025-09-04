// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::VelorDataClient,
    error::Error,
    interface::VelorDataClientInterface,
    peer_states::calculate_optimal_chunk_sizes,
    poller,
    priority::PeerPriority,
    tests::{mock::MockNetwork, utils},
};
use velor_config::{config::VelorDataClientConfig, network_id::PeerNetworkId};
use velor_storage_service_types::{
    requests::{DataRequest, TransactionsWithProofRequest},
    responses::{CompleteDataRange, DataResponse, StorageServerSummary, StorageServiceResponse},
};
use velor_time_service::MockTimeService;
use velor_types::transaction::{TransactionListWithProofV2, Version};
use claims::assert_matches;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn request_works_only_when_data_available() {
    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create a base config for a validator
        let base_config = utils::create_validator_base_config();

        // Create the velor data client config
        let data_client_config = VelorDataClientConfig {
            enable_transaction_data_v2: false,
            ..Default::default()
        };

        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) =
            MockNetwork::new(Some(base_config), Some(data_client_config), None);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Request transactions and verify the request fails (no peers are connected)
        fetch_transactions_and_verify_failure(&data_client_config, &client, 100, true).await;

        // Add a connected peer
        let (peer, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

        // Verify the peer's state has not been updated
        let peer_to_states = client.get_peer_states().get_peer_to_states();
        assert!(peer_to_states.is_empty());

        // Request transactions and verify the request fails (no peers are advertising data)
        fetch_transactions_and_verify_failure(&data_client_config, &client, 100, false).await;

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Get and verify the received network request
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        assert_eq!(network_request.peer_network_id, peer);

        // Handle the request
        let storage_summary = utils::create_storage_summary(200);
        utils::handle_storage_summary_request(network_request, storage_summary.clone());

        // Let the poller finish processing the response
        tokio::task::yield_now().await;

        // Handle the client's transaction request
        tokio::spawn(async move {
            // Verify the received network request
            let network_request = utils::get_network_request(&mut mock_network, network_id).await;
            assert_matches!(
                network_request.storage_service_request.data_request,
                DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    start_version: 0,
                    end_version: 100,
                    proof_version: 100,
                    include_events: false,
                })
            );

            // Fulfill the request
            utils::handle_transactions_request(network_request, true);
        });

        // Verify the peer's state has been updated
        verify_peer_state(&client, peer, storage_summary).await;

        // Request transactions and verify the request succeeds
        let request_timeout = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 0, 100, false, request_timeout)
            .await
            .unwrap();
        assert_eq!(response.payload, TransactionListWithProofV2::new_empty());
    }
}

#[tokio::test]
async fn update_global_data_summary() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the mock network, mock time, client and poller
    let data_client_config = VelorDataClientConfig::default();
    let (mut mock_network, mut mock_time, client, poller) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Start the poller
    tokio::spawn(poller::start_poller(poller));

    // Verify the global data summary is empty
    let global_data_summary = client.get_global_data_summary();
    assert!(global_data_summary.is_empty());

    // Add several peers of different priorities and advertise data for them
    let mut advertised_peer_versions = vec![];
    for (index, peer_priority) in PeerPriority::get_all_ordered_priorities()
        .iter()
        .enumerate()
    {
        // Add the peer
        let (_, network_id) = utils::add_peer_to_network(*peer_priority, &mut mock_network);

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Create the peer's storage summary
        let peer_version = ((index + 1) * 10000) as u64;
        let storage_summary = utils::create_storage_summary(peer_version);
        advertised_peer_versions.push(peer_version);

        // Handle the peer's data summary request
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        let data_response = DataResponse::StorageServerSummary(storage_summary.clone());
        network_request
            .response_sender
            .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));

        // Verify that the advertised data ranges are valid
        verify_advertised_transaction_data(
            &data_client_config,
            &client,
            &mut mock_time,
            peer_version,
            index + 1,
            true,
        )
        .await;
    }

    // Verify that the advertised data ranges are all present
    for (index, peer_version) in advertised_peer_versions.iter().enumerate() {
        let is_highest_version = index == advertised_peer_versions.len() - 1;
        verify_advertised_transaction_data(
            &data_client_config,
            &client,
            &mut mock_time,
            *peer_version,
            advertised_peer_versions.len(),
            is_highest_version,
        )
        .await;
    }
}

#[tokio::test]
async fn update_peer_states() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create the mock network, mock time, client and poller
    let data_client_config = VelorDataClientConfig::default();
    let (mut mock_network, mut mock_time, client, poller) =
        MockNetwork::new(Some(base_config), Some(data_client_config), None);

    // Start the poller
    tokio::spawn(poller::start_poller(poller));

    // Add a high priority peer
    let (high_priority_peer, high_priority_network) =
        utils::add_peer_to_network(PeerPriority::HighPriority, &mut mock_network);

    // Verify that we have no peer states
    let peer_to_states = client.get_peer_states().get_peer_to_states();
    assert!(peer_to_states.is_empty());

    // Advance time so the poller sends a data summary request for the peer
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Handle the high priority peer's data summary request
    let network_request =
        utils::get_network_request(&mut mock_network, high_priority_network).await;
    let high_priority_storage_summary = utils::create_storage_summary(1111);
    utils::handle_storage_summary_request(network_request, high_priority_storage_summary.clone());

    // Let the poller finish processing the responses
    tokio::task::yield_now().await;

    // Verify that the high priority peer's state has been updated
    verify_peer_state(&client, high_priority_peer, high_priority_storage_summary).await;

    // Add a medium priority peer
    let (medium_priority_peer, medium_priority_network) =
        utils::add_peer_to_network(PeerPriority::MediumPriority, &mut mock_network);

    // Advance time so the poller sends a data summary request for both peers
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Handle the high priority peer's data summary request
    let network_request =
        utils::get_network_request(&mut mock_network, high_priority_network).await;
    let high_priority_storage_summary = utils::create_storage_summary(2222);
    utils::handle_storage_summary_request(network_request, high_priority_storage_summary.clone());

    // Handle the medium peer's data summary request
    let network_request =
        utils::get_network_request(&mut mock_network, medium_priority_network).await;
    let medium_priority_storage_summary = utils::create_storage_summary(3333);
    utils::handle_storage_summary_request(network_request, medium_priority_storage_summary.clone());

    // Let the poller finish processing the responses
    tokio::task::yield_now().await;

    // Verify that the peer's states have been set
    verify_peer_state(&client, high_priority_peer, high_priority_storage_summary).await;
    verify_peer_state(
        &client,
        medium_priority_peer,
        medium_priority_storage_summary,
    )
    .await;

    // Add a low priority peer
    let (low_priority_peer, low_priority_network) =
        utils::add_peer_to_network(PeerPriority::LowPriority, &mut mock_network);

    // Advance time so the poller sends a data summary request for all peers
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Handle the high priority peer's data summary request
    let network_request =
        utils::get_network_request(&mut mock_network, high_priority_network).await;
    let high_priority_storage_summary = utils::create_storage_summary(4444);
    utils::handle_storage_summary_request(network_request, high_priority_storage_summary.clone());

    // Handle the medium peer's data summary request
    let network_request =
        utils::get_network_request(&mut mock_network, medium_priority_network).await;
    let medium_priority_storage_summary = utils::create_storage_summary(5555);
    utils::handle_storage_summary_request(network_request, medium_priority_storage_summary.clone());

    // Handle the low priority peer's data summary request
    let network_request = utils::get_network_request(&mut mock_network, low_priority_network).await;
    let low_priority_storage_summary = utils::create_storage_summary(6666);
    utils::handle_storage_summary_request(network_request, low_priority_storage_summary.clone());

    // Let the poller finish processing the responses
    tokio::task::yield_now().await;

    // Verify that the peer's states have been set
    verify_peer_state(&client, high_priority_peer, high_priority_storage_summary).await;
    verify_peer_state(
        &client,
        medium_priority_peer,
        medium_priority_storage_summary,
    )
    .await;
    verify_peer_state(&client, low_priority_peer, low_priority_storage_summary).await;
}

#[tokio::test]
async fn optimal_chunk_size_calculations() {
    // Create a test storage service config
    let max_epoch_chunk_size = 600;
    let max_state_chunk_size = 500;
    let max_transaction_chunk_size = 700;
    let max_transaction_output_chunk_size = 800;
    let data_client_config = VelorDataClientConfig {
        max_epoch_chunk_size,
        max_state_chunk_size,
        max_transaction_chunk_size,
        max_transaction_output_chunk_size,
        ..Default::default()
    };

    // Test median calculations
    let optimal_chunk_sizes = calculate_optimal_chunk_sizes(
        &data_client_config,
        vec![7, 5, 6, 8, 10],
        vec![100, 200, 300, 100],
        vec![900, 700, 500],
        vec![40],
    );
    assert_eq!(200, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(7, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(700, optimal_chunk_sizes.transaction_chunk_size);
    assert_eq!(40, optimal_chunk_sizes.transaction_output_chunk_size);

    // Test no advertised data
    let optimal_chunk_sizes =
        calculate_optimal_chunk_sizes(&data_client_config, vec![], vec![], vec![], vec![]);
    assert_eq!(max_state_chunk_size, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(max_epoch_chunk_size, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(
        max_transaction_chunk_size,
        optimal_chunk_sizes.transaction_chunk_size
    );
    assert_eq!(
        max_transaction_output_chunk_size,
        optimal_chunk_sizes.transaction_output_chunk_size
    );

    // Verify the config caps the amount of chunks
    let optimal_chunk_sizes = calculate_optimal_chunk_sizes(
        &data_client_config,
        vec![70, 50, 60, 80, 100],
        vec![1000, 1000, 2000, 3000],
        vec![9000, 7000, 5000],
        vec![400],
    );
    assert_eq!(max_state_chunk_size, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(70, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(
        max_transaction_chunk_size,
        optimal_chunk_sizes.transaction_chunk_size
    );
    assert_eq!(400, optimal_chunk_sizes.transaction_output_chunk_size);
}

/// Requests transactions up to the specified version and verifies the request fails
async fn fetch_transactions_and_verify_failure(
    data_client_config: &VelorDataClientConfig,
    data_client: &VelorDataClient,
    version: u64,
    no_connected_peers: bool,
) {
    // Request the transactions with proof
    let request_timeout = data_client_config.response_timeout_ms;
    let error = data_client
        .get_transactions_with_proof(version, 0, version, false, request_timeout)
        .await
        .unwrap_err();

    // Verify the error is correct
    if no_connected_peers {
        assert_matches!(error, Error::NoConnectedPeers(_));
    } else {
        assert_matches!(error, Error::DataIsUnavailable(_));
    }
}

/// Verifies that the advertised transaction data is valid
async fn verify_advertised_transaction_data(
    data_client_config: &VelorDataClientConfig,
    client: &VelorDataClient,
    mock_time: &mut MockTimeService,
    advertised_version: Version,
    expected_num_advertisements: usize,
    is_highest_version: bool,
) {
    // Wait for the advertised data to be updated
    timeout(Duration::from_secs(10), async {
        loop {
            // Advance time so the poller updates the global data summary
            utils::advance_polling_timer(mock_time, data_client_config).await;

            // Sleep for a while before retrying
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Get the advertised data
            let global_data_summary = client.get_global_data_summary();
            let advertised_data = global_data_summary.advertised_data;

            // Verify the number of advertised entries
            if advertised_data.transactions.len() != expected_num_advertisements {
                continue; // The advertised data has not been updated yet
            }

            // Verify that the advertised transaction data contains an entry for the given version
            let transaction_range = CompleteDataRange::new(0, advertised_version).unwrap();
            if !advertised_data.transactions.contains(&transaction_range) {
                continue; // The advertised data has not been updated yet
            }

            // Verify that the highest synced ledger info is valid (if this is the highest advertised version)
            if is_highest_version {
                let highest_synced_ledger_info =
                    advertised_data.highest_synced_ledger_info().unwrap();
                if highest_synced_ledger_info.ledger_info().version() != advertised_version {
                    continue; // The advertised data has not been updated yet
                }
            }

            // All checks passed
            return;
        }
    })
    .await
    .expect("The advertised data was not updated correctly! Timed out!");
}

/// Verifies that the peer's state is updated to the correct value
async fn verify_peer_state(
    client: &VelorDataClient,
    peer: PeerNetworkId,
    expected_storage_summary: StorageServerSummary,
) {
    // Wait for the peer's state to be updated to the expected storage summary
    timeout(Duration::from_secs(10), async {
        loop {
            // Check if the peer's state has been updated
            let peer_to_states = client.get_peer_states().get_peer_to_states();
            if let Some(peer_state) = peer_to_states.get(&peer) {
                if let Some(storage_summary) = peer_state.get_storage_summary_if_not_ignored() {
                    if storage_summary == &expected_storage_summary {
                        return; // The peer's state has been updated correctly
                    }
                }
            }

            // Sleep for a while before retrying
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("The peer state was not updated to the expected storage summary! Timed out!");
}
