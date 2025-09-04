// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::VelorDataClient,
    error::Result,
    global_summary::GlobalDataSummary,
    interface::{VelorDataClientInterface, Response, SubscriptionRequestMetadata},
    poller::DataSummaryPoller,
    priority::PeerPriority,
};
use velor_channels::{velor_channel, message_queues::QueueStyle};
use velor_config::{
    config::{VelorDataClientConfig, BaseConfig, RoleType},
    network_id::{NetworkId, PeerNetworkId},
};
use velor_netcore::transport::ConnectionOrigin;
use velor_network::{
    application::{interface::NetworkClient, metadata::ConnectionState, storage::PeersAndMetadata},
    peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
    protocols::{
        network::{NetworkSender, NewNetworkSender},
        wire::handshake::v1::ProtocolId,
    },
    transport::ConnectionMetadata,
};
use velor_peer_monitoring_service_types::{
    response::NetworkInformationResponse, PeerMonitoringMetadata,
};
use velor_storage_interface::DbReader;
use velor_storage_service_client::StorageServiceClient;
use velor_storage_service_server::network::{NetworkRequest, ResponseSender};
use velor_storage_service_types::{
    responses::TransactionOrOutputListWithProofV2, Epoch, StorageServiceMessage,
};
use velor_time_service::{MockTimeService, TimeService};
use velor_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProofV2, TransactionOutputListWithProofV2, Version},
    PeerId,
};
use async_trait::async_trait;
use futures::StreamExt;
use mockall::mock;
use rand::{rngs::OsRng, Rng};
use std::{collections::HashMap, sync::Arc};

/// A simple mock network for testing the data client
pub struct MockNetwork {
    base_config: BaseConfig,                   // The base config of the node
    networks: Vec<NetworkId>,                  // The networks that the node is connected to
    peers_and_metadata: Arc<PeersAndMetadata>, // The peers and metadata struct
    peer_mgr_reqs_rxs:
        HashMap<NetworkId, velor_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>>, // The peer manager request receivers
}

impl MockNetwork {
    pub fn new(
        base_config: Option<BaseConfig>,
        data_client_config: Option<VelorDataClientConfig>,
        networks: Option<Vec<NetworkId>>,
    ) -> (Self, MockTimeService, VelorDataClient, DataSummaryPoller) {
        // Initialize the logger for testing
        ::velor_logger::Logger::init_for_testing();

        // Setup the network IDs
        let networks = networks
            .unwrap_or_else(|| vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public]);

        // Create the network senders and receivers for each network
        let mut network_senders = HashMap::new();
        let mut peer_mgr_reqs_rxs = HashMap::new();
        for network in &networks {
            // Setup the request managers
            let queue_cfg = velor_channel::Config::new(10).queue_style(QueueStyle::FIFO);
            let (peer_mgr_reqs_tx, peer_mgr_reqs_rx) = queue_cfg.build();
            let (connection_reqs_tx, _connection_reqs_rx) = queue_cfg.build();

            // Create the network sender
            let network_sender = NetworkSender::new(
                PeerManagerRequestSender::new(peer_mgr_reqs_tx),
                ConnectionRequestSender::new(connection_reqs_tx),
            );

            // Save the network sender and the request receiver
            network_senders.insert(*network, network_sender);
            peer_mgr_reqs_rxs.insert(*network, peer_mgr_reqs_rx);
        }

        // Create the network client
        let peers_and_metadata = PeersAndMetadata::new(&networks);
        let network_client = NetworkClient::new(
            vec![],
            vec![ProtocolId::StorageServiceRpc],
            network_senders,
            peers_and_metadata.clone(),
        );

        // Create a storage service client
        let storage_service_client = StorageServiceClient::new(network_client);

        // Create an velor data client
        let mock_time = TimeService::mock();
        let base_config = base_config.unwrap_or_default();
        let data_client_config = data_client_config.unwrap_or_default();
        let (client, poller) = VelorDataClient::new(
            data_client_config,
            base_config.clone(),
            mock_time.clone(),
            create_mock_db_reader(),
            storage_service_client,
            None,
        );

        // Create the mock network
        let mock_network = Self {
            base_config,
            networks,
            peer_mgr_reqs_rxs,
            peers_and_metadata,
        };

        (mock_network, mock_time.into_mock(), client, poller)
    }

    /// Add a new peer to the network peer DB
    pub fn add_peer(&mut self, peer_priority: PeerPriority) -> PeerNetworkId {
        // Determine the network ID and connection direction
        // based on the given peer priority and the node role.
        let (network_id, outbound_connection) = match self.base_config.role {
            RoleType::Validator => {
                // Validators prioritize other validators, then VFNs, then PFNs
                match peer_priority {
                    PeerPriority::HighPriority => (NetworkId::Validator, true),
                    PeerPriority::MediumPriority => (NetworkId::Vfn, false),
                    PeerPriority::LowPriority => (NetworkId::Public, false),
                }
            },
            RoleType::FullNode => {
                if self.networks.contains(&NetworkId::Vfn) {
                    // VFNs prioritize validators, then other VFNs, then PFNs
                    match peer_priority {
                        PeerPriority::HighPriority => (NetworkId::Vfn, true),
                        PeerPriority::MediumPriority => (NetworkId::Public, true), // Outbound connection to VFN
                        PeerPriority::LowPriority => (NetworkId::Public, false), // Inbound connection from PFN
                    }
                } else {
                    // PFNs prioritize VFNs, then other PFNs
                    match peer_priority {
                        PeerPriority::HighPriority => (NetworkId::Public, true), // Outbound connection to VFN
                        PeerPriority::MediumPriority => {
                            unimplemented!("Medium priority peers are not yet supported for PFNs!")
                        },
                        PeerPriority::LowPriority => (NetworkId::Public, false), // Inbound connection from PFN
                    }
                }
            },
        };

        // Create and add the new peer
        self.add_peer_with_network_id(network_id, outbound_connection)
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

        // Insert peer monitoring metadata for the peer
        let network_info_response = NetworkInformationResponse {
            connected_peers: Default::default(),
            distance_from_validators: OsRng.gen(),
        };
        let peer_monitoring_metadata = PeerMonitoringMetadata::new(
            Some(OsRng.gen()),
            None,
            Some(network_info_response),
            None,
            None,
        );
        self.peers_and_metadata
            .update_peer_monitoring_metadata(peer_network_id, peer_monitoring_metadata)
            .unwrap();

        // Return the new peer
        peer_network_id
    }

    /// Returns the peers and metadata
    pub fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.peers_and_metadata.clone()
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
    pub async fn next_request(&mut self, network_id: NetworkId) -> Option<NetworkRequest> {
        let peer_mgr_reqs_rx = self.peer_mgr_reqs_rxs.get_mut(&network_id).unwrap();
        match peer_mgr_reqs_rx.next().await {
            Some(PeerManagerRequest::SendRpc(peer_id, network_request)) => {
                let peer_network_id = PeerNetworkId::new(network_id, peer_id);
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
pub fn create_mock_data_client() -> Arc<dyn VelorDataClientInterface + Send + Sync> {
    Arc::new(MockVelorDataClient::new())
}

// This automatically creates a MockVelorDataClient
mock! {
    pub VelorDataClient {}

    #[async_trait]
    impl VelorDataClientInterface for VelorDataClient {
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
        ) -> Result<Response<(TransactionOutputListWithProofV2, LedgerInfoWithSignatures)>>;

        async fn get_new_transactions_with_proof(
            &self,
            known_version: Version,
            known_epoch: Epoch,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<(TransactionListWithProofV2, LedgerInfoWithSignatures)>>;

        async fn get_new_transactions_or_outputs_with_proof(
            &self,
            known_version: Version,
            known_epoch: Epoch,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<(TransactionOrOutputListWithProofV2, LedgerInfoWithSignatures)>>;

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
        ) -> Result<Response<TransactionOutputListWithProofV2>>;

        async fn get_transactions_with_proof(
            &self,
            proof_version: Version,
            start_version: Version,
            end_version: Version,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<TransactionListWithProofV2>>;

        async fn get_transactions_or_outputs_with_proof(
            &self,
            proof_version: Version,
            start_version: Version,
            end_version: Version,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<TransactionOrOutputListWithProofV2>>;

        async fn subscribe_to_transaction_outputs_with_proof(
            &self,
            subscription_request_metadata: SubscriptionRequestMetadata,
            request_timeout_ms: u64,
        ) -> Result<Response<(TransactionOutputListWithProofV2, LedgerInfoWithSignatures)>>;

        async fn subscribe_to_transactions_with_proof(
            &self,
            subscription_request_metadata: SubscriptionRequestMetadata,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<(TransactionListWithProofV2, LedgerInfoWithSignatures)>>;

        async fn subscribe_to_transactions_or_outputs_with_proof(
            &self,
            subscription_request_metadata: SubscriptionRequestMetadata,
            include_events: bool,
            request_timeout_ms: u64,
        ) -> Result<Response<(TransactionOrOutputListWithProofV2, LedgerInfoWithSignatures)>>;
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
    fn get_block_timestamp(&self, version: Version) -> velor_storage_interface::Result<u64> {
        Ok(version * 100_000)
    }
}
