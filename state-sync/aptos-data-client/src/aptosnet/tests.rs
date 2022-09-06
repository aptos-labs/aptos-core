// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{AptosDataClient, AptosNetDataClient, DataSummaryPoller, Error};
use crate::aptosnet::{poll_peer, state::calculate_optimal_chunk_sizes};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig, RoleType, StorageServiceConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_crypto::HashValue;
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::{TransactionListWithProof, Version},
    PeerId,
};
use channel::{aptos_channel, message_queues::QueueStyle};
use claims::{assert_err, assert_matches, assert_none};
use futures::StreamExt;
use maplit::hashmap;
use netcore::transport::ConnectionOrigin;
use network::{
    application::{interface::MultiNetworkSender, storage::PeerMetadataStorage, types::PeerState},
    peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
    protocols::{network::NewNetworkSender, wire::handshake::v1::ProtocolId},
    transport::ConnectionMetadata,
};
use std::{collections::hash_map::Entry, sync::Arc, time::Duration};
use storage_service_client::{StorageServiceClient, StorageServiceNetworkSender};
use storage_service_server::network::{NetworkRequest, ResponseSender};
use storage_service_types::{
    requests::{
        DataRequest, NewTransactionOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StorageServiceRequest, TransactionOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{
        CompleteDataRange, DataResponse, DataSummary, ProtocolMetadata, StorageServerSummary,
        StorageServiceResponse, OPTIMISTIC_FETCH_VERSION_DELTA,
    },
    StorageServiceError, StorageServiceMessage,
};

fn mock_ledger_info(version: Version) -> LedgerInfoWithSignatures {
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(0, 0, HashValue::zero(), HashValue::zero(), version, 0, None),
            HashValue::zero(),
        ),
        AggregateSignature::empty(),
    )
}

fn mock_storage_summary(version: Version) -> StorageServerSummary {
    StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: 1000,
            max_state_chunk_size: 1000,
            max_transaction_chunk_size: 1000,
            max_transaction_output_chunk_size: 1000,
        },
        data_summary: DataSummary {
            synced_ledger_info: Some(mock_ledger_info(version)),
            epoch_ending_ledger_infos: None,
            transactions: Some(CompleteDataRange::new(0, version).unwrap()),
            transaction_outputs: Some(CompleteDataRange::new(0, version).unwrap()),
            states: None,
        },
    }
}

struct MockNetwork {
    peer_mgr_reqs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    peer_infos: Arc<PeerMetadataStorage>,
}

impl MockNetwork {
    fn new(
        base_config: Option<BaseConfig>,
        data_client_config: Option<AptosDataClientConfig>,
        networks: Option<Vec<NetworkId>>,
    ) -> (Self, MockTimeService, AptosNetDataClient, DataSummaryPoller) {
        let queue_cfg = aptos_channel::Config::new(10).queue_style(QueueStyle::FIFO);
        let (peer_mgr_reqs_tx, peer_mgr_reqs_rx) = queue_cfg.build();
        let (connection_reqs_tx, _connection_reqs_rx) = queue_cfg.build();

        let network_sender = MultiNetworkSender::new(hashmap! {
            NetworkId::Validator => StorageServiceNetworkSender::new(
                PeerManagerRequestSender::new(peer_mgr_reqs_tx),
                ConnectionRequestSender::new(connection_reqs_tx),
            )
        });

        let networks = networks
            .unwrap_or_else(|| vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public]);
        let peer_infos = PeerMetadataStorage::new(&networks);
        let network_client = StorageServiceClient::new(network_sender, peer_infos.clone());

        let mock_time = TimeService::mock();
        let base_config = base_config.unwrap_or_default();
        let data_client_config = data_client_config.unwrap_or_default();
        let (client, poller) = AptosNetDataClient::new(
            data_client_config,
            base_config,
            StorageServiceConfig::default(),
            mock_time.clone(),
            network_client,
            None,
        );

        let mock_network = Self {
            peer_mgr_reqs_rx,
            peer_infos,
        };
        (mock_network, mock_time.into_mock(), client, poller)
    }

    /// Add a new peer to the network peer DB
    fn add_peer(&mut self, priority: bool) -> PeerNetworkId {
        // Get the network id
        let network_id = if priority {
            NetworkId::Validator
        } else {
            NetworkId::Public
        };
        self.add_peer_with_network_id(network_id, false)
    }

    /// Add a new peer to the network peer DB with the specified network
    fn add_peer_with_network_id(
        &mut self,
        network_id: NetworkId,
        outbound_connection: bool,
    ) -> PeerNetworkId {
        // Create a new peer
        let peer_id = PeerId::random();
        let peer_network_id = PeerNetworkId::new(network_id, peer_id);

        // Create and save a new connection metadata
        let mut connection_metadata = ConnectionMetadata::mock(peer_id);
        connection_metadata.origin = if outbound_connection {
            ConnectionOrigin::Outbound
        } else {
            ConnectionOrigin::Inbound
        };
        connection_metadata
            .application_protocols
            .insert(ProtocolId::StorageServiceRpc);
        self.peer_infos
            .insert_connection(network_id, connection_metadata);

        // Return the new peer
        peer_network_id
    }

    /// Disconnects the peer in the network peer DB
    fn disconnect_peer(&mut self, peer: PeerNetworkId) {
        self.update_peer_state(peer, PeerState::Disconnected);
    }

    /// Reconnects the peer in the network peer DB
    fn reconnect_peer(&mut self, peer: PeerNetworkId) {
        self.update_peer_state(peer, PeerState::Connected);
    }

    /// Updates the state of the given peer
    fn update_peer_state(&mut self, peer: PeerNetworkId, state: PeerState) {
        self.peer_infos
            .write(peer, |entry| match entry {
                Entry::Vacant(..) => panic!("Peer must exist!"),
                Entry::Occupied(inner) => {
                    inner.get_mut().status = state;
                    Ok(())
                }
            })
            .unwrap();
    }

    /// Get the next request sent from the client.
    async fn next_request(&mut self) -> Option<NetworkRequest> {
        match self.peer_mgr_reqs_rx.next().await {
            Some(PeerManagerRequest::SendRpc(peer_id, network_request)) => {
                let protocol = network_request.protocol_id;
                let data = network_request.data;
                let res_tx = network_request.res_tx;

                let message: StorageServiceMessage = bcs::from_bytes(data.as_ref()).unwrap();
                let request = match message {
                    StorageServiceMessage::Request(request) => request,
                    _ => panic!("unexpected: {:?}", message),
                };
                let response_sender = ResponseSender::new(res_tx);

                Some((peer_id, protocol, request, response_sender))
            }
            Some(PeerManagerRequest::SendDirectSend(_, _)) => panic!("Unexpected direct send msg"),
            None => None,
        }
    }
}

#[tokio::test]
async fn request_works_only_when_data_available() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, mock_time, client, poller) = MockNetwork::new(None, None, None);

    tokio::spawn(poller.start_poller());

    // This request should fail because no peers are currently connected
    let error = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataIsUnavailable(_));

    // Add a connected peer
    let expected_peer = mock_network.add_peer(true);

    // Requesting some txns now will still fail since no peers are advertising
    // availability for the desired range.
    let error = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataIsUnavailable(_));

    // Advance time so the poller sends a data summary request
    tokio::task::yield_now().await;
    mock_time.advance_async(Duration::from_millis(1_000)).await;

    // Receive their request and fulfill it
    let (peer, protocol, request, response_sender) = mock_network.next_request().await.unwrap();
    assert_eq!(peer, expected_peer.peer_id());
    assert_eq!(protocol, ProtocolId::StorageServiceRpc);
    assert!(request.use_compression);
    assert_matches!(request.data_request, DataRequest::GetStorageServerSummary);

    let summary = mock_storage_summary(200);
    let data_response = DataResponse::StorageServerSummary(summary);
    response_sender.send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));

    // Let the poller finish processing the response
    tokio::task::yield_now().await;

    // Handle the client's transactions request
    tokio::spawn(async move {
        let (peer, protocol, request, response_sender) = mock_network.next_request().await.unwrap();

        assert_eq!(peer, expected_peer.peer_id());
        assert_eq!(protocol, ProtocolId::StorageServiceRpc);
        assert!(request.use_compression);
        assert_matches!(
            request.data_request,
            DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                start_version: 50,
                end_version: 100,
                proof_version: 100,
                include_events: false,
            })
        );

        let data_response =
            DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
        response_sender.send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));
    });

    // The client's request should succeed since a peer finally has advertised
    // data for this range.
    let response = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap();
    assert_eq!(response.payload, TransactionListWithProof::new_empty());
}

#[tokio::test]
async fn fetch_peers_frequency() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, poller) = MockNetwork::new(None, None, None);

    // Add regular peer 1 and 2
    let _regular_peer_1 = mock_network.add_peer(false);
    let _regular_peer_2 = mock_network.add_peer(false);

    // Set `always_poll` to true and fetch the regular peers multiple times. Ensure
    // that for each fetch we receive a peer.
    let num_fetches = 20;
    for _ in 0..num_fetches {
        let peer = poller.fetch_regular_peer(true).unwrap();
        client.in_flight_request_complete(&peer);
    }

    // Set `always_poll` to false and fetch the regular peers multiple times
    let mut regular_peer_count = 0;
    for _ in 0..num_fetches {
        if let Some(peer) = poller.fetch_regular_peer(false) {
            regular_peer_count += 1;
            client.in_flight_request_complete(&peer);
        }
    }

    // Verify we received regular peers at a reduced frequency
    assert!(regular_peer_count < num_fetches);

    // Add priority peer 1 and 2
    let _priority_peer_1 = mock_network.add_peer(true);
    let _priority_peer_2 = mock_network.add_peer(true);

    // Fetch the prioritized peers multiple times. Ensure that for
    // each fetch we receive a peer.
    for _ in 0..num_fetches {
        let peer = poller.try_fetch_peer(true).unwrap();
        client.in_flight_request_complete(&peer);
    }
}

#[tokio::test]
async fn fetch_peers_ordering() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for is_priority_peer in [true, false] {
        // Add peer 1
        let peer_1 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify that we get peer 1
        for _ in 0..3 {
            let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
                .unwrap()
                .unwrap();
            assert_eq!(peer_to_poll, peer_1);
            client.in_flight_request_complete(&peer_to_poll);
        }

        // Add peer 2
        let peer_2 = mock_network.add_peer(is_priority_peer);

        // Request the next peer and verify we get either peer
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert!(peer_to_poll == peer_1 || peer_to_poll == peer_2);
        client.in_flight_request_complete(&peer_to_poll);

        // Request the next peer again, but don't mark the poll as complete
        let peer_to_poll_1 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();

        // Request another peer again and verify that it's different to the previous peer
        let peer_to_poll_2 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_ne!(peer_to_poll_1, peer_to_poll_2);

        // Neither poll has completed (they're both in-flight), so make another request
        // and verify we get no peers.
        assert_none!(fetch_peer_to_poll(client.clone(), is_priority_peer).unwrap());

        // Add peer 3
        let peer_3 = mock_network.add_peer(is_priority_peer);

        // Request another peer again and verify it's peer_3
        let peer_to_poll_3 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll_3, peer_3);

        // Mark the second poll as completed
        client.in_flight_request_complete(&peer_to_poll_2);

        // Make another request and verify we get peer 2 now (as it was ready)
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_to_poll_2);

        // Mark the first poll as completed
        client.in_flight_request_complete(&peer_to_poll_1);

        // Make another request and verify we get peer 1 now
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_to_poll_1);

        // Mark the third poll as completed
        client.in_flight_request_complete(&peer_to_poll_3);

        // Make another request and verify we get peer 3 now
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_to_poll_3);
        client.in_flight_request_complete(&peer_to_poll_3);
    }
}

#[tokio::test]
async fn fetch_peers_disconnect() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for is_priority_peer in [true, false] {
        // Request the next peer to poll and verify we have no peers
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );

        // Add peer 1
        let peer_1 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 1
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_1);
        client.in_flight_request_complete(&peer_to_poll);

        // Add peer 2 and disconnect peer 1
        let peer_2 = mock_network.add_peer(is_priority_peer);
        mock_network.disconnect_peer(peer_1);

        // Request the next peer to poll and verify it's peer 2
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_2);
        client.in_flight_request_complete(&peer_to_poll);

        // Disconnect peer 2
        mock_network.disconnect_peer(peer_2);

        // Request the next peer to poll and verify an error is returned because
        // there are no connected peers.
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );

        // Add peer 3
        let peer_3 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 3
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_3);
        client.in_flight_request_complete(&peer_to_poll);

        // Disconnect peer 3
        mock_network.disconnect_peer(peer_3);

        // Request the next peer to poll and verify an error is returned because
        // there are no connected peers.
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );
    }
}

#[tokio::test]
async fn fetch_peers_reconnect() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Ensure the properties hold for both priority and non-priority peers
    for is_priority_peer in [true, false] {
        // Request the next peer to poll and verify we have no peers
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );

        // Add peer 1
        let peer_1 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 1
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_1);
        client.in_flight_request_complete(&peer_to_poll);

        // Add peer 2 and disconnect peer 1
        let peer_2 = mock_network.add_peer(is_priority_peer);
        mock_network.disconnect_peer(peer_1);

        // Request the next peer to poll and verify it's peer 2
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_2);
        client.in_flight_request_complete(&peer_to_poll);

        // Disconnect peer 2 and reconnect peer 1
        mock_network.disconnect_peer(peer_2);
        mock_network.reconnect_peer(peer_1);

        // Request the next peer to poll and verify it's peer 1
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_1);

        // Reconnect peer 2
        mock_network.reconnect_peer(peer_2);

        // Request the next peer to poll several times and verify it's peer 2
        // (the in-flight request for peer 1 has yet to complete).
        for _ in 0..3 {
            let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
                .unwrap()
                .unwrap();
            assert_eq!(peer_to_poll, peer_2);
            client.in_flight_request_complete(&peer_to_poll);
        }

        // Disconnect peer 2 and mark peer 1's in-flight request as complete
        mock_network.disconnect_peer(peer_2);
        client.in_flight_request_complete(&peer_1);

        // Request the next peer to poll several times and verify it's peer 1
        for _ in 0..3 {
            let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
                .unwrap()
                .unwrap();
            assert_eq!(peer_to_poll, peer_1);
            client.in_flight_request_complete(&peer_to_poll);
        }

        // Disconnect peer 1
        mock_network.disconnect_peer(peer_1);

        // Request the next peer to poll and verify an error is returned because
        // there are no connected peers.
        assert_matches!(
            fetch_peer_to_poll(client.clone(), is_priority_peer),
            Err(Error::DataIsUnavailable(_))
        );
    }
}

#[tokio::test]
async fn fetch_peers_max_in_flight() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a data client with max in-flight requests of 2
    let data_client_config = AptosDataClientConfig {
        max_num_in_flight_priority_polls: 2,
        max_num_in_flight_regular_polls: 2,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(None, Some(data_client_config), None);

    // Ensure the properties hold for both priority and non-priority peers
    for is_priority_peer in [true, false] {
        // Add peer 1
        let peer_1 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 1
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_1);

        // Add peer 2
        let peer_2 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's peer 2 (peer 1's in-flight
        // request has not yet completed).
        let peer_to_poll = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_eq!(peer_to_poll, peer_2);

        // Add peer 3
        let peer_3 = mock_network.add_peer(is_priority_peer);

        // Request the next peer to poll and verify it's empty (we already have
        // the maximum number of in-flight requests).
        assert_none!(fetch_peer_to_poll(client.clone(), is_priority_peer).unwrap());

        // Mark peer 2's in-flight request as complete
        client.in_flight_request_complete(&peer_2);

        // Request the next peer to poll and verify it's either peer 2 or peer 3
        let peer_to_poll_1 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert!(peer_to_poll_1 == peer_2 || peer_to_poll_1 == peer_3);

        // Request the next peer to poll and verify it's empty (we already have
        // the maximum number of in-flight requests).
        assert_none!(fetch_peer_to_poll(client.clone(), is_priority_peer).unwrap());

        // Mark peer 1's in-flight request as complete
        client.in_flight_request_complete(&peer_1);

        // Request the next peer to poll and verify it's not the peer that already
        // has an in-flight request.
        let peer_to_poll_2 = fetch_peer_to_poll(client.clone(), is_priority_peer)
            .unwrap()
            .unwrap();
        assert_ne!(peer_to_poll_1, peer_to_poll_2);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn in_flight_error_handling() {
    ::aptos_logger::Logger::init_for_testing();

    // Create a data client with max in-flight requests of 1
    let data_client_config = AptosDataClientConfig {
        max_num_in_flight_priority_polls: 1,
        max_num_in_flight_regular_polls: 1,
        ..Default::default()
    };
    let (mut mock_network, _, client, _) = MockNetwork::new(None, Some(data_client_config), None);

    // Verify we have no in-flight polls
    let num_in_flight_polls = get_num_in_flight_polls(client.clone(), true);
    assert_eq!(num_in_flight_polls, 0);

    // Add a peer
    let peer = mock_network.add_peer(true);

    // Poll the peer
    client.in_flight_request_started(&peer);
    let handle = poll_peer(client.clone(), peer, None);

    // Respond to the peer poll with an error
    if let Some((_, _, _, response_sender)) = mock_network.next_request().await {
        response_sender.send(Err(StorageServiceError::InternalError(
            "An unexpected error occurred!".into(),
        )));
    }

    // Wait for the poller to complete
    handle.await.unwrap();

    // Verify we have no in-flight polls
    let num_in_flight_polls = get_num_in_flight_polls(client.clone(), true);
    assert_eq!(num_in_flight_polls, 0);
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
async fn prioritized_peer_subscription_selection() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    // Create test data
    let known_version = 10000000;
    let known_epoch = 10;

    // Ensure the properties hold for both subscription requests
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
        client.update_summary(regular_peer_1, mock_storage_summary(known_version));
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
        client.update_summary(priority_peer_1, mock_storage_summary(known_version));
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Update the priority peer to be too far behind and verify it is not selected
        client.update_summary(
            priority_peer_1,
            mock_storage_summary(known_version - OPTIMISTIC_FETCH_VERSION_DELTA),
        );
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(regular_peer_1)
        );

        // Update the regular peer to be too far behind and verify neither is selected
        client.update_summary(
            regular_peer_1,
            mock_storage_summary(known_version - (OPTIMISTIC_FETCH_VERSION_DELTA * 2)),
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
        client.update_summary(priority_peer_1, mock_storage_summary(known_version + 1000));
        assert_eq!(
            client.choose_peer_for_request(&storage_request),
            Ok(priority_peer_1)
        );

        // Disconnect the priority peer so that we no longer have any connections
        mock_network.disconnect_peer(priority_peer_1);
    }
}

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
    client.update_summary(regular_peer_1, mock_storage_summary(100));
    assert_eq!(
        client.choose_peer_for_request(&storage_request),
        Ok(regular_peer_1)
    );

    // Advertise the data for the priority peer and verify the priority peer is selected
    client.update_summary(priority_peer_2, mock_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert_eq!(peer_for_request, priority_peer_2);

    // Reconnect priority peer 1 and remove the advertised data for priority peer 2
    mock_network.reconnect_peer(priority_peer_1);
    client.update_summary(priority_peer_2, mock_storage_summary(0));

    // Request the data again and verify the regular peer is chosen
    assert_eq!(
        client.choose_peer_for_request(&storage_request),
        Ok(regular_peer_1)
    );

    // Advertise the data for priority peer 1 and verify the priority peer is selected
    client.update_summary(priority_peer_1, mock_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert_eq!(peer_for_request, priority_peer_1);

    // Advertise the data for priority peer 2 and verify either priority peer is selected
    client.update_summary(priority_peer_2, mock_storage_summary(100));
    let peer_for_request = client.choose_peer_for_request(&storage_request).unwrap();
    assert!(peer_for_request == priority_peer_1 || peer_for_request == priority_peer_2);
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

// 1. 2 peers
// 2. one advertises bad range, one advertises honest range
// 3. sending a bunch of requests to the bad range (which will always go to the
//    bad peer) should lower bad peer's score
// 4. eventually bad peer score should hit threshold and we err with no available
#[tokio::test]
async fn bad_peer_is_eventually_banned_internal() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    let good_peer = mock_network.add_peer(true);
    let bad_peer = mock_network.add_peer(true);

    // Bypass poller and just add the storage summaries directly.

    // Good peer advertises txns 0 -> 100.
    client.update_summary(good_peer, mock_storage_summary(100));
    // Bad peer advertises txns 0 -> 200 (but can't actually service).
    client.update_summary(bad_peer, mock_storage_summary(200));
    client.update_global_summary_cache();

    // The global summary should contain the bad peer's advertisement.
    let global_summary = client.get_global_data_summary();
    assert!(global_summary
        .advertised_data
        .transactions
        .contains(&CompleteDataRange::new(0, 200).unwrap()));

    // Spawn a handler for both peers.
    tokio::spawn(async move {
        while let Some((peer, _, _, response_sender)) = mock_network.next_request().await {
            if peer == good_peer.peer_id() {
                // Good peer responds with good response.
                let data_response =
                    DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
                response_sender.send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));
            } else if peer == bad_peer.peer_id() {
                // Bad peer responds with error.
                response_sender.send(Err(StorageServiceError::InternalError("".to_string())));
            }
        }
    });

    let mut seen_data_unavailable_err = false;

    // Sending a bunch of requests to the bad peer's upper range will fail.
    for _ in 0..20 {
        let result = client
            .get_transactions_with_proof(200, 200, 200, false)
            .await;

        // While the score is still decreasing, we should see a bunch of
        // InternalError's. Once we see a `DataIsUnavailable` error, we should
        // only see that error.
        if !seen_data_unavailable_err {
            assert_err!(&result);
            if let Err(Error::DataIsUnavailable(_)) = result {
                seen_data_unavailable_err = true;
            }
        } else {
            assert_matches!(result, Err(Error::DataIsUnavailable(_)));
        }
    }

    // Peer should eventually get ignored and we should consider this request
    // range unserviceable.
    assert!(seen_data_unavailable_err);

    // The global summary should no longer contain the bad peer's advertisement.
    client.update_global_summary_cache();
    let global_summary = client.get_global_data_summary();
    assert!(!global_summary
        .advertised_data
        .transactions
        .contains(&CompleteDataRange::new(0, 200).unwrap()));

    // We should still be able to send the good peer a request.
    let response = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap();
    assert_eq!(response.payload, TransactionListWithProof::new_empty());
}

#[tokio::test]
async fn bad_peer_is_eventually_banned_callback() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, _, client, _) = MockNetwork::new(None, None, None);

    let bad_peer = mock_network.add_peer(true);

    // Bypass poller and just add the storage summaries directly.
    // Bad peer advertises txns 0 -> 200 (but can't actually service).
    client.update_summary(bad_peer, mock_storage_summary(200));
    client.update_global_summary_cache();

    // Spawn a handler for both peers.
    tokio::spawn(async move {
        while let Some((_, _, _, response_sender)) = mock_network.next_request().await {
            let data_response =
                DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
            response_sender.send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));
        }
    });

    let mut seen_data_unavailable_err = false;

    // Sending a bunch of requests to the bad peer (that we later decide are bad).
    for _ in 0..20 {
        let result = client
            .get_transactions_with_proof(200, 200, 200, false)
            .await;

        // While the score is still decreasing, we should see a bunch of
        // InternalError's. Once we see a `DataIsUnavailable` error, we should
        // only see that error.
        if !seen_data_unavailable_err {
            match result {
                Ok(response) => {
                    response
                        .context
                        .response_callback
                        .notify_bad_response(crate::ResponseError::ProofVerificationError);
                }
                Err(Error::DataIsUnavailable(_)) => {
                    seen_data_unavailable_err = true;
                }
                Err(_) => panic!("unexpected result: {:?}", result),
            }
        } else {
            assert_matches!(result, Err(Error::DataIsUnavailable(_)));
        }
    }

    // Peer should eventually get ignored and we should consider this request
    // range unserviceable.
    assert!(seen_data_unavailable_err);

    // The global summary should no longer contain the bad peer's advertisement.
    client.update_global_summary_cache();
    let global_summary = client.get_global_data_summary();
    assert!(!global_summary
        .advertised_data
        .transactions
        .contains(&CompleteDataRange::new(0, 200).unwrap()));
}

#[tokio::test]
async fn compression_mismatch_disabled() {
    ::aptos_logger::Logger::init_for_testing();

    // Disable compression
    let data_client_config = AptosDataClientConfig {
        use_compression: false,
        ..Default::default()
    };
    let (mut mock_network, mock_time, client, poller) =
        MockNetwork::new(None, Some(data_client_config), None);

    tokio::spawn(poller.start_poller());

    // Add a connected peer
    let _ = mock_network.add_peer(true);

    // Advance time so the poller sends a data summary request
    tokio::task::yield_now().await;
    mock_time.advance_async(Duration::from_millis(1_000)).await;

    // Receive their request and respond
    let (_, _, _, response_sender) = mock_network.next_request().await.unwrap();
    let data_response = DataResponse::StorageServerSummary(mock_storage_summary(200));
    response_sender.send(Ok(
        StorageServiceResponse::new(data_response, false).unwrap()
    ));

    // Let the poller finish processing the response
    tokio::task::yield_now().await;

    // Handle the client's transactions request using compression
    tokio::spawn(async move {
        let (_, _, request, response_sender) = mock_network.next_request().await.unwrap();
        assert!(!request.use_compression);

        // Compress the response
        let data_response =
            DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
        let storage_response = StorageServiceResponse::new(data_response, true).unwrap();
        response_sender.send(Ok(storage_response));
    });

    // The client should receive a compressed response and return an error
    let response = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap_err();
    assert_matches!(response, Error::InvalidResponse(_));
}

#[tokio::test]
async fn compression_mismatch_enabled() {
    ::aptos_logger::Logger::init_for_testing();

    // Enable compression
    let data_client_config = AptosDataClientConfig {
        use_compression: true,
        ..Default::default()
    };
    let (mut mock_network, mock_time, client, poller) =
        MockNetwork::new(None, Some(data_client_config), None);

    tokio::spawn(poller.start_poller());

    // Add a connected peer
    let _ = mock_network.add_peer(true);

    // Advance time so the poller sends a data summary request
    tokio::task::yield_now().await;
    mock_time.advance_async(Duration::from_millis(1_000)).await;

    // Receive their request and respond
    let (_, _, _, response_sender) = mock_network.next_request().await.unwrap();
    let data_response = DataResponse::StorageServerSummary(mock_storage_summary(200));
    response_sender.send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));

    // Let the poller finish processing the response
    tokio::task::yield_now().await;

    // Handle the client's transactions request without compression
    tokio::spawn(async move {
        let (_, _, request, response_sender) = mock_network.next_request().await.unwrap();
        assert!(request.use_compression);

        // Compress the response
        let data_response =
            DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
        let storage_response = StorageServiceResponse::new(data_response, false).unwrap();
        response_sender.send(Ok(storage_response));
    });

    // The client should receive a compressed response and return an error
    let response = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap_err();
    assert_matches!(response, Error::InvalidResponse(_));
}

#[tokio::test]
async fn disable_compression() {
    ::aptos_logger::Logger::init_for_testing();

    // Disable compression
    let data_client_config = AptosDataClientConfig {
        use_compression: false,
        ..Default::default()
    };
    let (mut mock_network, mock_time, client, poller) =
        MockNetwork::new(None, Some(data_client_config), None);

    tokio::spawn(poller.start_poller());

    // Add a connected peer
    let expected_peer = mock_network.add_peer(true);

    // Advance time so the poller sends a data summary request
    tokio::task::yield_now().await;
    mock_time.advance_async(Duration::from_millis(1_000)).await;

    // Receive their request
    let (peer, protocol, request, response_sender) = mock_network.next_request().await.unwrap();
    assert_eq!(peer, expected_peer.peer_id());
    assert_eq!(protocol, ProtocolId::StorageServiceRpc);
    assert!(!request.use_compression);
    assert_matches!(request.data_request, DataRequest::GetStorageServerSummary);

    // Fulfill their request
    let data_response = DataResponse::StorageServerSummary(mock_storage_summary(200));
    response_sender.send(Ok(
        StorageServiceResponse::new(data_response, false).unwrap()
    ));

    // Let the poller finish processing the response
    tokio::task::yield_now().await;

    // Handle the client's transactions request
    tokio::spawn(async move {
        let (peer, protocol, request, response_sender) = mock_network.next_request().await.unwrap();

        assert_eq!(peer, expected_peer.peer_id());
        assert_eq!(protocol, ProtocolId::StorageServiceRpc);
        assert!(!request.use_compression);
        assert_matches!(
            request.data_request,
            DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                start_version: 50,
                end_version: 100,
                proof_version: 100,
                include_events: false,
            })
        );

        let data_response =
            DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
        let storage_response = StorageServiceResponse::new(data_response, false).unwrap();
        response_sender.send(Ok(storage_response));
    });

    // The client's request should succeed since a peer finally has advertised
    // data for this range.
    let response = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap();
    assert_eq!(response.payload, TransactionListWithProof::new_empty());
}

#[tokio::test]
async fn bad_peer_is_eventually_added_back() {
    ::aptos_logger::Logger::init_for_testing();
    let (mut mock_network, mock_time, client, poller) = MockNetwork::new(None, None, None);

    // Add a connected peer.
    mock_network.add_peer(true);

    tokio::spawn(poller.start_poller());
    tokio::spawn(async move {
        while let Some((_, _, request, response_sender)) = mock_network.next_request().await {
            match request.data_request {
                DataRequest::GetTransactionsWithProof(_) => {
                    let data_response =
                        DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
                    response_sender.send(Ok(StorageServiceResponse::new(
                        data_response,
                        request.use_compression,
                    )
                    .unwrap()));
                }
                DataRequest::GetStorageServerSummary => {
                    let data_response =
                        DataResponse::StorageServerSummary(mock_storage_summary(200));
                    response_sender.send(Ok(StorageServiceResponse::new(
                        data_response,
                        request.use_compression,
                    )
                    .unwrap()));
                }
                _ => panic!("unexpected: {:?}", request),
            }
        }
    });

    // Advance time so the poller sends data summary requests.
    let summary_poll_interval = Duration::from_millis(1_000);
    for _ in 0..2 {
        tokio::task::yield_now().await;
        mock_time.advance_async(summary_poll_interval).await;
    }

    // Initially this request range is serviceable by this peer.
    let global_summary = client.get_global_data_summary();
    assert!(global_summary
        .advertised_data
        .transactions
        .contains(&CompleteDataRange::new(0, 200).unwrap()));

    // Keep decreasing this peer's score by considering its responses bad.
    // Eventually its score drops below IGNORE_PEER_THRESHOLD.
    for _ in 0..20 {
        let result = client.get_transactions_with_proof(200, 0, 200, false).await;

        if let Ok(response) = result {
            response
                .context
                .response_callback
                .notify_bad_response(crate::ResponseError::ProofVerificationError);
        }
    }

    // Peer is eventually ignored and this request range unserviceable.
    client.update_global_summary_cache();
    let global_summary = client.get_global_data_summary();
    assert!(!global_summary
        .advertised_data
        .transactions
        .contains(&CompleteDataRange::new(0, 200).unwrap()));

    // This peer still responds to the StorageServerSummary requests.
    // Its score keeps increasing and this peer is eventually added back.
    for _ in 0..20 {
        mock_time.advance_async(summary_poll_interval).await;
    }

    let global_summary = client.get_global_data_summary();
    assert!(global_summary
        .advertised_data
        .transactions
        .contains(&CompleteDataRange::new(0, 200).unwrap()));
}

#[tokio::test]
async fn optimal_chunk_size_calculations() {
    // Create a test storage service config
    let max_epoch_chunk_size = 600;
    let max_state_chunk_size = 500;
    let max_transaction_chunk_size = 700;
    let max_transaction_output_chunk_size = 800;
    let storage_service_config = StorageServiceConfig {
        max_concurrent_requests: 0,
        max_epoch_chunk_size,
        max_lru_cache_size: 0,
        max_network_channel_size: 0,
        max_network_chunk_bytes: 0,
        max_state_chunk_size,
        max_subscription_period_ms: 0,
        max_transaction_chunk_size,
        max_transaction_output_chunk_size,
        storage_summary_refresh_interval_ms: 0,
    };

    // Test median calculations
    let optimal_chunk_sizes = calculate_optimal_chunk_sizes(
        &storage_service_config,
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
        calculate_optimal_chunk_sizes(&storage_service_config, vec![], vec![], vec![], vec![]);
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
        &storage_service_config,
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

/// A helper method that fetches peers to poll depending on the peer priority
fn fetch_peer_to_poll(
    client: AptosNetDataClient,
    is_priority_peer: bool,
) -> Result<Option<PeerNetworkId>, Error> {
    // Fetch the next peer to poll
    let result = if is_priority_peer {
        client.fetch_prioritized_peer_to_poll()
    } else {
        client.fetch_regular_peer_to_poll()
    };

    // If we get a peer, mark the peer as having an in-flight request
    if let Ok(Some(peer_to_poll)) = result {
        client.in_flight_request_started(&peer_to_poll);
    }

    result
}

/// Fetches the number of in flight requests for peers depending on priority
fn get_num_in_flight_polls(client: AptosNetDataClient, is_priority_peer: bool) -> u64 {
    if is_priority_peer {
        client.peer_states.read().num_in_flight_priority_polls()
    } else {
        client.peer_states.read().num_in_flight_regular_polls()
    }
}
