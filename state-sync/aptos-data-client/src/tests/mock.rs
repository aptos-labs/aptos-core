// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::AptosDataClient,
    error::Result,
    global_summary::GlobalDataSummary,
    interface::{AptosDataClientInterface, Response},
    poller::DataSummaryPoller,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    application::{interface::NetworkClient, metadata::ConnectionState, storage::PeersAndMetadata},
    peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
    protocols::{
        network::{NetworkSender, NewNetworkSender},
        wire::handshake::v1::ProtocolId,
    },
    transport::ConnectionMetadata,
};
use aptos_storage_interface::DbReader;
use aptos_storage_service_client::StorageServiceClient;
use aptos_storage_service_server::network::{NetworkRequest, ResponseSender};
use aptos_storage_service_types::{
    responses::TransactionOrOutputListWithProof, Epoch, StorageServiceMessage,
};
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
    PeerId,
};
use async_trait::async_trait;
use futures::StreamExt;
use maplit::hashmap;
use mockall::mock;
use std::sync::Arc;

/// A simple mock network for testing the data client
pub struct MockNetwork {
    network_id: NetworkId,
    peer_mgr_reqs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    peers_and_metadata: Arc<PeersAndMetadata>,
}

impl MockNetwork {
    pub fn new(
        base_config: Option<BaseConfig>,
        data_client_config: Option<AptosDataClientConfig>,
        networks: Option<Vec<NetworkId>>,
    ) -> (Self, MockTimeService, AptosDataClient, DataSummaryPoller) {
        // Setup the request managers
        let queue_cfg = aptos_channel::Config::new(10).queue_style(QueueStyle::FIFO);
        let (peer_mgr_reqs_tx, peer_mgr_reqs_rx) = queue_cfg.build();
        let (connection_reqs_tx, _connection_reqs_rx) = queue_cfg.build();

        // Setup the network client
        let network_sender = NetworkSender::new(
            PeerManagerRequestSender::new(peer_mgr_reqs_tx),
            ConnectionRequestSender::new(connection_reqs_tx),
        );
        let networks = networks
            .unwrap_or_else(|| vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public]);
        let peers_and_metadata = PeersAndMetadata::new(&networks);
        let client_network_id = NetworkId::Validator;
        let network_client = NetworkClient::new(
            vec![],
            vec![ProtocolId::StorageServiceRpc],
            hashmap! {
            client_network_id => network_sender},
            peers_and_metadata.clone(),
        );

        // Create a storage service client
        let storage_service_client = StorageServiceClient::new(network_client);

        // Create an aptos data client
        let mock_time = TimeService::mock();
        let base_config = base_config.unwrap_or_default();
        let data_client_config = data_client_config.unwrap_or_default();
        let (client, poller) = AptosDataClient::new(
            data_client_config,
            base_config,
            mock_time.clone(),
            create_mock_db_reader(),
            storage_service_client,
            None,
        );

        // Create the mock network
        let mock_network = Self {
            network_id: client_network_id,
            peer_mgr_reqs_rx,
            peers_and_metadata,
        };

        (mock_network, mock_time.into_mock(), client, poller)
    }

    /// Add a new peer to the network peer DB
    pub fn add_peer(&mut self, priority: bool) -> PeerNetworkId {
        // Get the network id
        let network_id = if priority {
            NetworkId::Validator
        } else {
            NetworkId::Public
        };
        self.add_peer_with_network_id(network_id, false)
    }

    /// Add a new peer to the network peer DB with the specified network
    pub fn add_peer_with_network_id(
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
        self.peers_and_metadata
            .insert_connection_metadata(peer_network_id, connection_metadata)
            .unwrap();

        // Return the new peer
        peer_network_id
    }

    /// Disconnects the peer in the network peer DB
    pub fn disconnect_peer(&mut self, peer: PeerNetworkId) {
        self.update_peer_state(peer, ConnectionState::Disconnected);
    }

    /// Reconnects the peer in the network peer DB
    pub fn reconnect_peer(&mut self, peer: PeerNetworkId) {
        self.update_peer_state(peer, ConnectionState::Connected);
    }

    /// Updates the state of the given peer
    fn update_peer_state(&mut self, peer: PeerNetworkId, state: ConnectionState) {
        self.peers_and_metadata
            .update_connection_state(peer, state)
            .unwrap();
    }

    /// Get the next request sent from the client
    pub async fn next_request(&mut self) -> Option<NetworkRequest> {
        match self.peer_mgr_reqs_rx.next().await {
            Some(PeerManagerRequest::SendRpc(peer_id, network_request)) => {
                let peer_network_id = PeerNetworkId::new(self.network_id, peer_id);
                let protocol_id = network_request.protocol_id;
                let data = network_request.data;
                let res_tx = network_request.res_tx;

                let message: StorageServiceMessage = bcs::from_bytes(data.as_ref()).unwrap();
                let storage_service_request = match message {
                    StorageServiceMessage::Request(request) => request,
                    _ => panic!("unexpected: {:?}", message),
                };
                let response_sender = ResponseSender::new(res_tx);

                Some(NetworkRequest {
                    peer_network_id,
                    protocol_id,
                    storage_service_request,
                    response_sender,
                })
            },
            Some(PeerManagerRequest::SendDirectSend(_, _)) => panic!("Unexpected direct send msg"),
            None => None,
        }
    }
}

/// Creates a mock data client for testing
pub fn create_mock_data_client() -> Arc<dyn AptosDataClientInterface + Send + Sync> {
    Arc::new(MockAptosDataClient::new())
}

// This automatically creates a MockAptosDataClient
mock! {
    pub AptosDataClient {}

    #[async_trait]
    impl AptosDataClientInterface for AptosDataClient {
        fn get_global_data_summary(&self) -> GlobalDataSummary;

        async fn get_epoch_ending_ledger_infos(
            &self,
            start_epoch: Epoch,
            expected_end_epoch: Epoch,
            request_timeout_ms: u64,
        ) -> Result<Response<Vec<LedgerInfoWithSignatures>>>;

        async fn get_new_transaction_outputs_with_proof(
            &self,
            known_version: Version,
            known_epoch: Epoch,
            request_timeout_ms: u64,
        ) -> Result<Response<(TransactionOutputListWithProof, LedgerInfoWithSignatures)>>;

        async fn get_new_transactions_with_proof(
            &self,
            known_version: Version,
            known_epoch: Epoch,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<(TransactionListWithProof, LedgerInfoWithSignatures)>>;

        async fn get_new_transactions_or_outputs_with_proof(
            &self,
            known_version: Version,
            known_epoch: Epoch,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<(TransactionOrOutputListWithProof, LedgerInfoWithSignatures)>>;

        async fn get_number_of_states(
            &self,
            version: Version,
            request_timeout_ms: u64,
        ) -> Result<Response<u64>>;

        async fn get_state_values_with_proof(
            &self,
            version: u64,
            start_index: u64,
            end_index: u64,
            request_timeout_ms: u64,
        ) -> Result<Response<StateValueChunkWithProof>>;

        async fn get_transaction_outputs_with_proof(
            &self,
            proof_version: Version,
            start_version: Version,
            end_version: Version,
            request_timeout_ms: u64,
        ) -> Result<Response<TransactionOutputListWithProof>>;

        async fn get_transactions_with_proof(
            &self,
            proof_version: Version,
            start_version: Version,
            end_version: Version,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<TransactionListWithProof>>;

        async fn get_transactions_or_outputs_with_proof(
            &self,
            proof_version: Version,
            start_version: Version,
            end_version: Version,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<TransactionOrOutputListWithProof>>;
    }
}

/// Creates a mock database reader for testing
pub fn create_mock_db_reader() -> Arc<dyn DbReader> {
    Arc::new(MockDatabaseReader {})
}

/// A simple mock database reader that only implements
/// the functions required by the tests.
pub struct MockDatabaseReader {}
impl DbReader for MockDatabaseReader {
    fn get_block_timestamp(&self, version: Version) -> anyhow::Result<u64> {
        Ok(version * 100_000)
    }
}
