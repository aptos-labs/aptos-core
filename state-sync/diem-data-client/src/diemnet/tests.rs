// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::{
    DataClientPayload, DataSummaryPoller, DiemDataClient, DiemNetDataClient, Error,
    DATA_SUMMARY_POLL_INTERVAL,
};
use channel::{diem_channel, message_queues::QueueStyle};
use claim::assert_matches;
use diem_config::network_id::{NetworkId, PeerNetworkId};
use diem_time_service::{MockTimeService, TimeService};
use diem_types::{transaction::TransactionListWithProof, PeerId};
use futures::StreamExt;
use maplit::hashmap;
use network::{
    application::{interface::MultiNetworkSender, storage::PeerMetadataStorage},
    peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
    protocols::{network::NewNetworkSender, wire::handshake::v1::ProtocolId},
    transport::ConnectionMetadata,
};
use std::sync::Arc;
use storage_service_client::{StorageServiceClient, StorageServiceNetworkSender};
use storage_service_server::network::{NetworkRequest, ResponseSender};
use storage_service_types::{
    CompleteDataRange, DataSummary, ProtocolMetadata, StorageServerSummary, StorageServiceMessage,
    StorageServiceRequest, StorageServiceResponse, TransactionsWithProofRequest,
};

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
        let (client, poller) = DiemNetDataClient::new(mock_time.clone(), network_client);

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

    let summary = StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: 1000,
            max_transaction_chunk_size: 1000,
            max_transaction_output_chunk_size: 1000,
            max_account_states_chunk_size: 1000,
        },
        data_summary: DataSummary {
            synced_ledger_info: None,
            epoch_ending_ledger_infos: None,
            transactions: Some(CompleteDataRange::from_genesis(200)),
            transaction_outputs: None,
            account_states: None,
        },
    };
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
                expected_num_transactions: 51,
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

    assert_eq!(
        response.response_payload,
        DataClientPayload::TransactionsWithProof(TransactionListWithProof::new_empty())
    );
}
