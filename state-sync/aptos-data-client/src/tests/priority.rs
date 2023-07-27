// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig, RoleType},
    network_id::NetworkId,
};
use aptos_storage_service_types::requests::{
    DataRequest, NewTransactionOutputsWithProofRequest, NewTransactionsWithProofRequest,
    StorageServiceRequest, TransactionOutputsWithProofRequest,
};
use claims::assert_matches;

#[tokio::test]
async fn all_peer_request_selection() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure no peers can service the given request (we have no connections)
    let server_version_request =
        StorageServiceRequest::new(DataRequest::GetServerProtocolVersion, true);
    assert_matches!(
        client.choose_peer_for_request(&server_version_request),
        Err(Error::DataIsUnavailable(_))
    );

    // Add a regular peer and verify the peer is selected as the recipient
    let regular_peer_1 = mock_network.add_peer(false);
    assert_eq!(
        client.choose_peer_for_request(&server_version_request),
        Ok(regular_peer_1)
    );

    // Add two prioritized peers
    let priority_peer_1 = mock_network.add_peer(true);
    let priority_peer_2 = mock_network.add_peer(true);

    // Request data that is not being advertised and verify we get an error
    let output_data_request =
        DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version: 100,
            start_version: 0,
            end_version: 100,
        });
    let storage_request = StorageServiceRequest::new(output_data_request, false);
    assert_matches!(
        client.choose_peer_for_request(&storage_request),
        Err(Error::DataIsUnavailable(_))
    );

    // Advertise the data for the regular peer and verify it is now selected
    client.update_summary(regular_peer_1, utils::create_storage_summary(100));
    assert_eq!(
        client.choose_peer_for_request(&storage_request),
        Ok(regular_peer_1)
    );

    // Advertise the data for the priority peer and verify the priority peer is selected
    client.update_summary(priority_peer_2, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert_eq!(peer_for_request, priority_peer_2);

    // Reconnect priority peer 1 and remove the advertised data for priority peer 2
    mock_network.reconnect_peer(priority_peer_1);
    client.update_summary(priority_peer_2, utils::create_storage_summary(0));

    // Request the data again and verify the regular peer is chosen
    assert_eq!(
        client.choose_peer_for_request(&storage_request),
        Ok(regular_peer_1)
    );

    // Advertise the data for priority peer 1 and verify the priority peer is selected
    client.update_summary(priority_peer_1, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert_eq!(peer_for_request, priority_peer_1);

    // Advertise the data for priority peer 2 and verify either priority peer is selected
    client.update_summary(priority_peer_2, utils::create_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert!(peer_for_request == priority_peer_1 || peer_for_request == priority_peer_2);
}

#[tokio::test]
async fn prioritized_peer_request_selection() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for storage summary and version requests
    let storage_summary_request = DataRequest::GetStorageServerSummary;
    let get_version_request = DataRequest::GetServerProtocolVersion;
    for data_request in [storage_summary_request, get_version_request] {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Add a regular peer and verify the peer is selected as the recipient
        let regular_peer_1 = mock_network.add_peer(false);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Add a priority peer and verify the peer is selected as the recipient
        let priority_peer_1 = mock_network.add_peer(true);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Disconnect the priority peer and verify the regular peer is now chosen
        mock_network.disconnect_peer(priority_peer_1);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Connect a new priority peer and verify it is now selected
        let priority_peer_2 = mock_network.add_peer(true);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_2)
        );

        // Disconnect the priority peer and verify the regular peer is again chosen
        mock_network.disconnect_peer(priority_peer_2);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Disconnect the regular peer so that we no longer have any connections
        mock_network.disconnect_peer(regular_peer_1);
    }
}

#[tokio::test]
async fn prioritized_peer_optimistic_fetch_selection() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a data client with a max version lag of 100
    let max_optimistic_fetch_version_lag = 100;
    let data_client_config = AptosDataClientConfig {
        max_optimistic_fetch_version_lag,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(None, Some(data_client_config), None);

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;

    // Ensure the properties hold for both optimistic fetch requests
    let new_transactions_request =
        DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version,
            known_epoch,
            include_events: false,
        });
    let new_outputs_request =
        DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
            known_version,
            known_epoch,
        });
    for data_request in [new_transactions_request, new_outputs_request] {
        let storage_request = StorageServiceRequest::new(data_request, true);

        // Ensure no peers can service the request (we have no connections)
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Add a regular peer and verify the peer cannot support the request
        let regular_peer_1 = mock_network.add_peer(false);
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Advertise the data for the regular peer and verify it is now selected
        client.update_summary(regular_peer_1, utils::create_storage_summary(known_version));
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Add a priority peer and verify the regular peer is selected
        let priority_peer_1 = mock_network.add_peer(true);
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Advertise the data for the priority peer and verify it is now selected
        client.update_summary(
            priority_peer_1,
            utils::create_storage_summary(known_version),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Update the priority peer to be too far behind and verify it is not selected
        client.update_summary(
            priority_peer_1,
            utils::create_storage_summary(known_version - max_optimistic_fetch_version_lag),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Update the regular peer to be too far behind and verify neither is selected
        client.update_summary(
            regular_peer_1,
            utils::create_storage_summary(known_version - (max_optimistic_fetch_version_lag * 2)),
        );
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Disconnect the regular peer and verify neither is selected
        mock_network.disconnect_peer(regular_peer_1);
        assert_matches!(
            client.choose_peer_for_request(&storage_request),
            Err(Error::DataIsUnavailable(_))
        );

        // Advertise the data for the priority peer and verify it is now selected again
        client.update_summary(
            priority_peer_1,
            utils::create_storage_summary(known_version + 1000),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Disconnect the priority peer so that we no longer have any connections
        mock_network.disconnect_peer(priority_peer_1);
    }
}

#[tokio::test]
async fn validator_peer_prioritization() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a validator node
    let base_config = BaseConfig {
        role: RoleType::Validator,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(Some(base_config), None, None);

    // Add a validator peer and ensure it's prioritized
    let validator_peer = mock_network.add_peer_with_network_id(NetworkId::Validator, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![validator_peer]);
    assert_eq!(regular_peers, vec![]);

    // Add a vfn peer and ensure it's not prioritized
    let vfn_peer = mock_network.add_peer_with_network_id(NetworkId::Vfn, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![validator_peer]);
    assert_eq!(regular_peers, vec![vfn_peer]);
}

#[tokio::test]
async fn vfn_peer_prioritization() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a validator fullnode
    let base_config = BaseConfig {
        role: RoleType::FullNode,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(Some(base_config), None, None);

    // Add a validator peer and ensure it's prioritized
    let validator_peer = mock_network.add_peer_with_network_id(NetworkId::Vfn, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![validator_peer]);
    assert_eq!(regular_peers, vec![]);

    // Add a pfn peer and ensure it's not prioritized
    let pfn_peer = mock_network.add_peer_with_network_id(NetworkId::Public, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![validator_peer]);
    assert_eq!(regular_peers, vec![pfn_peer]);
}

#[tokio::test]
async fn pfn_peer_prioritization() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a public fullnode
    let base_config = BaseConfig {
        role: RoleType::FullNode,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) =
        MockNetwork::new(Some(base_config), None, Some(vec![NetworkId::Public]));

    // Add an inbound pfn peer and ensure it's not prioritized
    let inbound_peer = mock_network.add_peer_with_network_id(NetworkId::Public, false);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![]);
    assert_eq!(regular_peers, vec![inbound_peer]);

    // Add an outbound pfn peer and ensure it's prioritized
    let outbound_peer = mock_network.add_peer_with_network_id(NetworkId::Public, true);
    let (priority_peers, regular_peers) = client.get_priority_and_regular_peers().unwrap();
    assert_eq!(priority_peers, vec![outbound_peer]);
    assert_eq!(regular_peers, vec![inbound_peer]);
}
