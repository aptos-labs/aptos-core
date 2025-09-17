// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    interface::AptosDataClientInterface,
    poller,
    priority::PeerPriority,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::{config::AptosDataClientConfig, network_id::NetworkId};
use aptos_network::protocols::wire::handshake::v1::ProtocolId;
use aptos_storage_service_types::{
    requests::{DataRequest, TransactionsWithProofRequest},
    responses::{CompleteDataRange, DataResponse, StorageServiceResponse},
};
use aptos_types::transaction::TransactionListWithProofV2;
use claims::assert_matches;

#[tokio::test]
async fn compression_mismatch_disabled() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client config that disables compression
    let data_client_config = AptosDataClientConfig {
        use_compression: false,
        ..Default::default()
    };

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) =
            MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Add a connected peer
        let (_, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Receive their request and respond
        let highest_synced_version = 100;
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        let data_response = DataResponse::StorageServerSummary(utils::create_storage_summary(
            highest_synced_version,
        ));
        network_request.response_sender.send(Ok(
            StorageServiceResponse::new(data_response, false).unwrap()
        ));

        // Wait for the poller to process the response
        let transaction_range = CompleteDataRange::new(0, highest_synced_version).unwrap();
        utils::wait_for_transaction_advertisement(
            &client,
            &mut mock_time,
            &data_client_config,
            transaction_range,
        )
        .await;

        // Handle the client's transactions request using compression
        tokio::spawn(async move {
            loop {
                // Verify the received network request
                let network_request =
                    utils::get_network_request(&mut mock_network, network_id).await;
                assert!(!network_request.storage_service_request.use_compression);

                // Fulfill the request if it is for transactions
                if matches!(
                    network_request.storage_service_request.data_request,
                    DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                        start_version: 50,
                        end_version: 100,
                        proof_version: 100,
                        include_events: false,
                    })
                ) {
                    // Compress the response
                    utils::handle_transactions_request(network_request, true);
                }
            }
        });

        // The client should receive a compressed response and return an error
        let request_timeout = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 50, 100, false, request_timeout)
            .await
            .unwrap_err();
        assert_matches!(response, Error::DataIsUnavailable(_));
    }
}

#[tokio::test]
async fn compression_mismatch_enabled() {
    // Create a base config for a validator
    let base_config = utils::create_validator_base_config();

    // Create a data client config that enables compression
    let data_client_config = AptosDataClientConfig {
        use_compression: true,
        ..Default::default()
    };

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) =
            MockNetwork::new(Some(base_config.clone()), Some(data_client_config), None);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Add a connected peer
        let (_, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Receive their request and respond
        let highest_synced_version = 200;
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        utils::handle_storage_summary_request(
            network_request,
            utils::create_storage_summary(highest_synced_version),
        );

        // Wait for the poller to process the response
        let transaction_range = CompleteDataRange::new(0, highest_synced_version).unwrap();
        utils::wait_for_transaction_advertisement(
            &client,
            &mut mock_time,
            &data_client_config,
            transaction_range,
        )
        .await;

        // Handle the client's transactions request without compression
        tokio::spawn(async move {
            loop {
                // Verify the received network request
                let network_request =
                    utils::get_network_request(&mut mock_network, network_id).await;
                assert!(network_request.storage_service_request.use_compression);

                // Fulfill the request if it is for transactions
                if matches!(
                    network_request.storage_service_request.data_request,
                    DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                        start_version: 50,
                        end_version: 100,
                        proof_version: 100,
                        include_events: false,
                    })
                ) {
                    // Don't compress the response
                    utils::handle_transactions_request(network_request, false);
                }
            }
        });

        // The client should receive a compressed response and return an error
        let request_timeout = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 50, 100, false, request_timeout)
            .await
            .unwrap_err();
        assert_matches!(response, Error::DataIsUnavailable(_));
    }
}

#[tokio::test]
async fn disable_compression() {
    // Create a base config for a VFN
    let base_config = utils::create_fullnode_base_config();
    let networks = vec![NetworkId::Vfn, NetworkId::Public];

    // Create a data client config that disables compression
    let data_client_config = AptosDataClientConfig {
        enable_transaction_data_v2: false,
        use_compression: false,
        ..Default::default()
    };

    // Ensure the properties hold for all peer priorities
    for peer_priority in PeerPriority::get_all_ordered_priorities() {
        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) = MockNetwork::new(
            Some(base_config.clone()),
            Some(data_client_config),
            Some(networks.clone()),
        );

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Add a connected peer
        let (peer, network_id) = utils::add_peer_to_network(peer_priority, &mut mock_network);

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Verify the received network request
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        assert_eq!(network_request.peer_network_id, peer);
        assert_eq!(network_request.protocol_id, ProtocolId::StorageServiceRpc);
        assert!(!network_request.storage_service_request.use_compression);
        assert_matches!(
            network_request.storage_service_request.data_request,
            DataRequest::GetStorageServerSummary
        );

        // Fulfill their request
        let highest_synced_version = 200;
        let data_response = DataResponse::StorageServerSummary(utils::create_storage_summary(
            highest_synced_version,
        ));
        network_request.response_sender.send(Ok(
            StorageServiceResponse::new(data_response, false).unwrap()
        ));

        // Wait for the poller to process the response
        let transaction_range = CompleteDataRange::new(0, highest_synced_version).unwrap();
        utils::wait_for_transaction_advertisement(
            &client,
            &mut mock_time,
            &data_client_config,
            transaction_range,
        )
        .await;

        // Handle the client's requests
        tokio::spawn(async move {
            loop {
                // Verify the received network request
                let network_request =
                    utils::get_network_request(&mut mock_network, network_id).await;
                assert_eq!(network_request.peer_network_id, peer);
                assert_eq!(network_request.protocol_id, ProtocolId::StorageServiceRpc);
                assert!(!network_request.storage_service_request.use_compression);

                // Fulfill the request if it is for transactions
                if matches!(
                    network_request.storage_service_request.data_request,
                    DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                        start_version: 50,
                        end_version: 100,
                        proof_version: 100,
                        include_events: false,
                    })
                ) {
                    utils::handle_transactions_request(network_request, false);
                }
            }
        });

        // The request should succeed since a peer has advertised the data
        let request_timeout = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 50, 100, false, request_timeout)
            .await
            .unwrap();
        assert_eq!(response.payload, TransactionListWithProofV2::new_empty());
    }
}
