// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::{
    DataSummaryPoller, DiemDataClient, DiemNetDataClient, Error, DATA_SUMMARY_POLL_INTERVAL,
};
use channel::{diem_channel, message_queues::QueueStyle};
use claim::{assert_err, assert_matches};
use diem_config::{
    config::StorageServiceConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use diem_crypto::HashValue;
use diem_time_service::{MockTimeService, TimeService};
use diem_types::{
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::{TransactionListWithProof, Version},
    PeerId,
};
use futures::StreamExt;
use maplit::hashmap;
use network::{
    application::{interface::MultiNetworkSender, storage::PeerMetadataStorage},
    peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
    protocols::{network::NewNetworkSender, wire::handshake::v1::ProtocolId},
    transport::ConnectionMetadata,
};
use std::{collections::BTreeMap, sync::Arc};
use storage_service_client::{StorageServiceClient, StorageServiceNetworkSender};
use storage_service_server::network::{NetworkRequest, ResponseSender};
use storage_service_types::{
    CompleteDataRange, DataSummary, ProtocolMetadata, StorageServerSummary, StorageServiceError,
    StorageServiceMessage, StorageServiceRequest, StorageServiceResponse,
    TransactionsWithProofRequest,
};

fn mock_ledger_info(version: Version) -> LedgerInfoWithSignatures {
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(0, 0, HashValue::zero(), HashValue::zero(), version, 0, None),
            HashValue::zero(),
        ),
        BTreeMap::new(),
    )
}

fn mock_storage_summary(version: Version) -> StorageServerSummary {
    StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: 1000,
            max_transaction_chunk_size: 1000,
            max_transaction_output_chunk_size: 1000,
            max_account_states_chunk_size: 1000,
        },
        data_summary: DataSummary {
            synced_ledger_info: Some(mock_ledger_info(version)),
            epoch_ending_ledger_infos: None,
            transactions: Some(CompleteDataRange::new(0, version).unwrap()),
            transaction_outputs: None,
            account_states: None,
        },
    }
}

struct MockNetwork {
    peer_mgr_reqs_rx: diem_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    peer_infos: Arc<PeerMetadataStorage>,
}

impl MockNetwork {
    fn new() -> (Self, MockTimeService, DiemNetDataClient, DataSummaryPoller) {
        let queue_cfg = diem_channel::Config::new(10).queue_style(QueueStyle::FIFO);
        let (peer_mgr_reqs_tx, peer_mgr_reqs_rx) = queue_cfg.build();
        let (connection_reqs_tx, _connection_reqs_rx) = queue_cfg.build();

        let network_sender = MultiNetworkSender::new(hashmap! {
            NetworkId::Validator => StorageServiceNetworkSender::new(
                PeerManagerRequestSender::new(peer_mgr_reqs_tx),
                ConnectionRequestSender::new(connection_reqs_tx),
            )
        });

        let peer_infos = PeerMetadataStorage::new(&[NetworkId::Validator]);
        let network_client = StorageServiceClient::new(network_sender, peer_infos.clone());

        let mock_time = TimeService::mock();
        let (client, poller) = DiemNetDataClient::new(
            StorageServiceConfig::default(),
            mock_time.clone(),
            network_client,
        );

        let mock_network = Self {
            peer_mgr_reqs_rx,
            peer_infos,
        };
        (mock_network, mock_time.into_mock(), client, poller)
    }

    /// Add a new random connected peer to the network peer DB
    fn add_connected_peer(&mut self) -> PeerNetworkId {
        let network_id = NetworkId::Validator;
        let peer_id = PeerId::random();
        let mut connection_metadata = ConnectionMetadata::mock(peer_id);
        connection_metadata
            .application_protocols
            .insert(ProtocolId::StorageServiceRpc);

        self.peer_infos
            .insert_connection(network_id, connection_metadata);
        PeerNetworkId::new(network_id, peer_id)
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
async fn test_request_works_only_when_data_available() {
    ::diem_logger::Logger::init_for_testing();
    let (mut mock_network, mock_time, client, poller) = MockNetwork::new();

    tokio::spawn(poller.start());

    // this request should fail because no peers are currently connected
    let error = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataIsUnavailable(_));

    // add a connected peer
    let expected_peer = mock_network.add_connected_peer();

    // requesting some txns now will still fail since no peers are advertising
    // availability for the desired range.
    let error = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap_err();
    assert_matches!(error, Error::DataIsUnavailable(_));

    // advance time so the poller sends a data summary request
    tokio::task::yield_now().await;
    mock_time.advance_async(DATA_SUMMARY_POLL_INTERVAL).await;

    // receive their request and fulfill it
    let (peer, protocol, request, response_sender) = mock_network.next_request().await.unwrap();
    assert_eq!(peer, expected_peer.peer_id());
    assert_eq!(protocol, ProtocolId::StorageServiceRpc);
    assert_matches!(request, StorageServiceRequest::GetStorageServerSummary);

    let summary = mock_storage_summary(200);
    response_sender.send(Ok(StorageServiceResponse::StorageServerSummary(summary)));

    // let the poller finish processing the response
    tokio::task::yield_now().await;

    // handle the client's transactions request
    tokio::spawn(async move {
        let (peer, protocol, request, response_sender) = mock_network.next_request().await.unwrap();

        assert_eq!(peer, expected_peer.peer_id());
        assert_eq!(protocol, ProtocolId::StorageServiceRpc);
        assert_matches!(
            request,
            StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                start_version: 50,
                end_version: 100,
                proof_version: 100,
                include_events: false,
            })
        );

        response_sender.send(Ok(StorageServiceResponse::TransactionsWithProof(
            TransactionListWithProof::new_empty(),
        )));
    });

    // the client's request should succeed since a peer finally has advertised
    // data for this range.
    let response = client
        .get_transactions_with_proof(100, 50, 100, false)
        .await
        .unwrap();
    assert_eq!(response.payload, TransactionListWithProof::new_empty());
}

// 1. 2 peers
// 2. one advertises bad range, one advertises honest range
// 3. sending a bunch of requests to the bad range (which will always go to the
//    bad peer) should lower bad peer's score
// 4. eventually bad peer score should hit threshold and we err with no available

#[tokio::test]
async fn bad_peer_is_eventually_banned_internal() {
    ::diem_logger::Logger::init_for_testing();
    let (mut mock_network, _mock_time, client, _poller) = MockNetwork::new();

    let good_peer = mock_network.add_connected_peer();
    let bad_peer = mock_network.add_connected_peer();

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
                response_sender.send(Ok(StorageServiceResponse::TransactionsWithProof(
                    TransactionListWithProof::new_empty(),
                )));
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
    ::diem_logger::Logger::init_for_testing();
    let (mut mock_network, _mock_time, client, _poller) = MockNetwork::new();

    let bad_peer = mock_network.add_connected_peer();

    // Bypass poller and just add the storage summaries directly.
    // Bad peer advertises txns 0 -> 200 (but can't actually service).
    client.update_summary(bad_peer, mock_storage_summary(200));
    client.update_global_summary_cache();

    // Spawn a handler for both peers.
    tokio::spawn(async move {
        while let Some((_, _, _, response_sender)) = mock_network.next_request().await {
            response_sender.send(Ok(StorageServiceResponse::TransactionsWithProof(
                TransactionListWithProof::new_empty(),
            )));
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
async fn bad_peer_is_eventually_added_back() {
    ::diem_logger::Logger::init_for_testing();
    let (mut mock_network, mock_time, client, poller) = MockNetwork::new();

    // Add a connected peer.
    mock_network.add_connected_peer();

    tokio::spawn(poller.start());
    tokio::spawn(async move {
        while let Some((_, _, request, response_sender)) = mock_network.next_request().await {
            match request {
                StorageServiceRequest::GetTransactionsWithProof(_) => {
                    response_sender.send(Ok(StorageServiceResponse::TransactionsWithProof(
                        TransactionListWithProof::new_empty(),
                    )))
                }
                StorageServiceRequest::GetStorageServerSummary => response_sender.send(Ok(
                    StorageServiceResponse::StorageServerSummary(mock_storage_summary(200)),
                )),
                _ => panic!("unexpected: {:?}", request),
            }
        }
    });

    // Advance time so the poller sends a data summary request.
    tokio::task::yield_now().await;
    mock_time.advance_async(DATA_SUMMARY_POLL_INTERVAL).await;

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
        mock_time.advance_async(DATA_SUMMARY_POLL_INTERVAL).await;
    }

    let global_summary = client.get_global_data_summary();
    assert!(global_summary
        .advertised_data
        .transactions
        .contains(&CompleteDataRange::new(0, 200).unwrap()));
}
