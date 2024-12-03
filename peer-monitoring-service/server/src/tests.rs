// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    metrics, storage::StorageReader, tests::database_mock::MockDatabaseReader,
    PeerMonitoringServiceNetworkEvents, PeerMonitoringServiceServer, MAX_DISTANCE_FROM_VALIDATORS,
    PEER_MONITORING_SERVER_VERSION,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::{BaseConfig, NodeConfig, PeerMonitoringServiceConfig, PeerRole, RoleType},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_crypto::HashValue;
use aptos_logger::Level;
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    application::{
        interface::NetworkServiceEvents, metadata::ConnectionState, storage::PeersAndMetadata,
    },
    protocols::{
        network::{NetworkEvents, NewNetworkEvents, ReceivedMessage},
        wire::{
            handshake::v1::{MessagingProtocolVersion, ProtocolId, ProtocolIdSet},
            messaging::v1::{NetworkMessage, RpcRequest},
        },
    },
    transport::{ConnectionId, ConnectionMetadata},
};
use aptos_peer_monitoring_service_types::{
    request::{LatencyPingRequest, PeerMonitoringServiceRequest},
    response::{
        NetworkInformationResponse, NodeInformationResponse, PeerMonitoringServiceResponse,
        ServerProtocolVersionResponse,
    },
    PeerMonitoringMetadata, PeerMonitoringServiceError, PeerMonitoringServiceMessage,
};
use aptos_storage_interface::{DbReader, LedgerSummary, Order};
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    contract_event::EventWithVersion,
    epoch_change::EpochChangeProof,
    event::EventKey,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    network_address::NetworkAddress,
    proof::{AccumulatorConsistencyProof, SparseMerkleProof, TransactionAccumulatorSummary},
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        AccountTransactionsWithProof, TransactionListWithProof, TransactionOutputListWithProof,
        TransactionWithProof, Version,
    },
    PeerId,
};
use futures::channel::oneshot;
use maplit::btreemap;
use mockall::mock;
use rand::{rngs::OsRng, Rng};
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

// Useful test constants
const LOCAL_HOST_NET_ADDR: &str = "/ip4/127.0.0.1/tcp/8081";

#[tokio::test]
async fn test_get_server_protocol_version() {
    // Create the peer monitoring client and server
    let (mut mock_client, service, _, _) = MockClient::new(None, None, None);
    tokio::spawn(service.start());

    // Process a request to fetch the protocol version
    let request = PeerMonitoringServiceRequest::GetServerProtocolVersion;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response =
        PeerMonitoringServiceResponse::ServerProtocolVersion(ServerProtocolVersionResponse {
            version: PEER_MONITORING_SERVER_VERSION,
        });
    assert_eq!(response, expected_response);
}

#[tokio::test]
async fn test_get_network_information_fullnode() {
    // Create the peer monitoring client and server
    let base_config = BaseConfig {
        role: RoleType::FullNode, // The server is a fullnode
        ..Default::default()
    };
    let (mut mock_client, service, _, peers_and_metadata) =
        MockClient::new(Some(base_config), None, None);
    tokio::spawn(service.start());

    // Process a client request to fetch the network information and verify an empty response
    verify_network_information(
        &mut mock_client,
        BTreeMap::new(),
        MAX_DISTANCE_FROM_VALIDATORS,
    )
    .await;

    // Connect a new peer to the fullnode
    let peer_id_1 = PeerId::random();
    let peer_network_id_1 = PeerNetworkId::new(NetworkId::Public, peer_id_1);
    let mut connection_metadata_1 = create_connection_metadata(peer_id_1, PeerRole::Unknown);
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_1, connection_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    let expected_peers = btreemap! {peer_network_id_1 => connection_metadata_1.clone()};
    verify_network_information(
        &mut mock_client,
        expected_peers.clone(),
        MAX_DISTANCE_FROM_VALIDATORS,
    )
    .await;

    // Update the peer monitoring metadata for peer 1
    let peer_distance_1 = MAX_DISTANCE_FROM_VALIDATORS; // Peer 1 is not connected to anyone else
    let latest_network_info_response = NetworkInformationResponse {
        connected_peers: transform_connection_metadata(expected_peers.clone()),
        distance_from_validators: peer_distance_1,
    };
    let peer_monitoring_metadata_1 =
        PeerMonitoringMetadata::new(None, None, Some(latest_network_info_response), None, None);
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_1, peer_monitoring_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        expected_peers.clone(),
        MAX_DISTANCE_FROM_VALIDATORS,
    )
    .await;

    // Update the peer monitoring metadata and connection metadata for peer 1
    let peer_distance_1 = 2; // Peer 1 now has other connections
    let latest_network_info_response = NetworkInformationResponse {
        connected_peers: transform_connection_metadata(expected_peers),
        distance_from_validators: peer_distance_1,
    };
    let peer_monitoring_metadata_1 =
        PeerMonitoringMetadata::new(None, None, Some(latest_network_info_response), None, None);
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_1, peer_monitoring_metadata_1.clone())
        .unwrap();
    connection_metadata_1.connection_id = ConnectionId::from(101);
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_1, connection_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        btreemap! {peer_network_id_1 => connection_metadata_1.clone()},
        peer_distance_1 + 1,
    )
    .await;

    // Connect another peer to the fullnode
    let peer_id_2 = PeerId::random();
    let peer_network_id_2 = PeerNetworkId::new(NetworkId::Validator, peer_id_2);
    let peer_distance_2 = 0; // The peer is a validator
    let connection_metadata_2 = create_connection_metadata(peer_id_2, PeerRole::Validator);
    let expected_peers = btreemap! {peer_network_id_1 => connection_metadata_1.clone(), peer_network_id_2 => connection_metadata_2.clone()};
    let latest_network_info_response = NetworkInformationResponse {
        connected_peers: transform_connection_metadata(expected_peers),
        distance_from_validators: peer_distance_2,
    };
    let peer_monitoring_metadata_2 =
        PeerMonitoringMetadata::new(None, None, Some(latest_network_info_response), None, None);
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_2, connection_metadata_2.clone())
        .unwrap();
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_2, peer_monitoring_metadata_2.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        btreemap! {peer_network_id_1 => connection_metadata_1.clone(), peer_network_id_2 => connection_metadata_2},
        peer_distance_2 + 1,
    )
    .await;

    // Disconnect peer 2
    peers_and_metadata
        .update_connection_state(peer_network_id_2, ConnectionState::Disconnected)
        .unwrap();

    // Process a request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        btreemap! {peer_network_id_1 => connection_metadata_1},
        peer_distance_1 + 1,
    )
    .await;
}

#[tokio::test]
async fn test_get_network_information_validator() {
    // Create the peer monitoring client and server
    let base_config = BaseConfig {
        role: RoleType::Validator, // The server is a validator
        ..Default::default()
    };
    let (mut mock_client, service, _, peers_and_metadata) =
        MockClient::new(Some(base_config), None, None);
    tokio::spawn(service.start());

    // Process a client request to fetch the network information and verify
    // the distance is the max (the server has no peers!).
    verify_network_information(
        &mut mock_client,
        BTreeMap::new(),
        MAX_DISTANCE_FROM_VALIDATORS,
    )
    .await;

    // Connect a new peer to the validator (another validator)
    let peer_id_1 = PeerId::random();
    let peer_network_id_1 = PeerNetworkId::new(NetworkId::Validator, peer_id_1);
    let connection_metadata_1 = create_connection_metadata(peer_id_1, PeerRole::Validator);
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_1, connection_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    let expected_peers = btreemap! {peer_network_id_1 => connection_metadata_1.clone()};
    verify_network_information(&mut mock_client, expected_peers.clone(), 0).await;

    // Update the peer monitoring metadata for peer 1
    let peer_distance_1 = 1; // Peer 1 now has other connections
    let latest_network_info_response = NetworkInformationResponse {
        connected_peers: transform_connection_metadata(expected_peers.clone()),
        distance_from_validators: peer_distance_1,
    };
    let peer_monitoring_metadata_1 =
        PeerMonitoringMetadata::new(None, None, Some(latest_network_info_response), None, None);
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_1, peer_monitoring_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    verify_network_information(&mut mock_client, expected_peers, 0).await;

    // Connect another peer to the validator (a VFN)
    let peer_id_2 = PeerId::random();
    let peer_network_id_2 = PeerNetworkId::new(NetworkId::Vfn, peer_id_2);
    let peer_distance_2 = 2; // The peer is a VFN
    let connection_metadata_2 = create_connection_metadata(peer_id_2, PeerRole::ValidatorFullNode);
    let expected_peers = btreemap! {peer_network_id_1 => connection_metadata_1.clone(), peer_network_id_2 => connection_metadata_2.clone()};
    let latest_network_info_response = NetworkInformationResponse {
        connected_peers: transform_connection_metadata(expected_peers.clone()),
        distance_from_validators: peer_distance_2,
    };
    let peer_monitoring_metadata_2 =
        PeerMonitoringMetadata::new(None, None, Some(latest_network_info_response), None, None);
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_2, connection_metadata_2.clone())
        .unwrap();
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_2, peer_monitoring_metadata_2.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    verify_network_information(&mut mock_client, expected_peers, 0).await;

    // Disconnect peer 1
    peers_and_metadata
        .update_connection_state(peer_network_id_1, ConnectionState::Disconnected)
        .unwrap();

    // Process a request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        btreemap! {peer_network_id_2 => connection_metadata_2},
        peer_distance_2 + 1,
    )
    .await;
}

#[tokio::test]
async fn test_get_node_information() {
    // Setup the mock data
    let highest_synced_epoch = 5;
    let highest_synced_version = 1000;
    let ledger_timestamp_usecs = 9734834;
    let block_info = BlockInfo::new(
        highest_synced_epoch,
        0,
        HashValue::zero(),
        HashValue::zero(),
        highest_synced_version,
        ledger_timestamp_usecs,
        None,
    );
    let latest_ledger_info = LedgerInfoWithSignatures::new(
        LedgerInfo::new(block_info, HashValue::zero()),
        AggregateSignature::empty(),
    );
    let lowest_available_version = 19;

    // Create the mock storage reader
    let mut mock_db_reader = create_mock_db_reader();

    // Setup the mock expectations
    mock_db_reader
        .expect_get_latest_ledger_info()
        .returning(move || Ok(latest_ledger_info.clone()));
    mock_db_reader
        .expect_get_first_txn_version()
        .returning(move || Ok(Some(lowest_available_version)));

    // Create the peer monitoring client and server
    let storage_reader = StorageReader::new(Arc::new(mock_db_reader));
    let (mut mock_client, service, time_service, _) =
        MockClient::new(None, None, Some(storage_reader));
    tokio::spawn(service.start());

    // Process a client request to fetch the node information and verify the response
    let mut total_uptime = Duration::from_millis(0);
    verify_node_information(
        &mut mock_client,
        highest_synced_epoch,
        highest_synced_version,
        ledger_timestamp_usecs,
        lowest_available_version,
        total_uptime,
    )
    .await;

    // Handle several more node information requests with new uptimes
    for _ in 0..10 {
        // Elapse a little bit of time
        let duration_to_elapse = Duration::from_millis(100);
        time_service.advance(duration_to_elapse);
        total_uptime = total_uptime.saturating_add(duration_to_elapse);

        // Process a client request to fetch the node information and verify the response
        verify_node_information(
            &mut mock_client,
            highest_synced_epoch,
            highest_synced_version,
            ledger_timestamp_usecs,
            lowest_available_version,
            total_uptime,
        )
        .await;
    }
}

#[tokio::test]
async fn test_latency_ping_request() {
    // Create the peer monitoring client and server
    let (mut mock_client, service, _, _) = MockClient::new(None, None, None);
    tokio::spawn(service.start());

    // Process several requests to perform latency pings
    for i in 0..10 {
        let request =
            PeerMonitoringServiceRequest::LatencyPing(LatencyPingRequest { ping_counter: i });
        let response = mock_client.send_request(request).await.unwrap();
        match response {
            PeerMonitoringServiceResponse::LatencyPing(latecy_ping_response) => {
                assert_eq!(latecy_ping_response.ping_counter, i);
            },
            _ => panic!("Expected latency ping response but got: {:?}", response),
        }
    }
}

/// A simple utility function to create a new connection metadata for tests
fn create_connection_metadata(peer_id: AccountAddress, peer_role: PeerRole) -> ConnectionMetadata {
    ConnectionMetadata::new(
        peer_id,
        ConnectionId::default(),
        NetworkAddress::from_str(LOCAL_HOST_NET_ADDR).unwrap(),
        ConnectionOrigin::Inbound,
        MessagingProtocolVersion::V1,
        ProtocolIdSet::empty(),
        peer_role,
    )
}

/// A simple utility function that sends a request for network info using the given
/// client, and verifies the response is correct.
async fn verify_network_information(
    client: &mut MockClient,
    expected_peers: BTreeMap<PeerNetworkId, ConnectionMetadata>,
    expected_distance_from_validators: u64,
) {
    // Send a request to fetch the network information
    let request = PeerMonitoringServiceRequest::GetNetworkInformation;
    let response = client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response =
        PeerMonitoringServiceResponse::NetworkInformation(NetworkInformationResponse {
            connected_peers: transform_connection_metadata(expected_peers),
            distance_from_validators: expected_distance_from_validators,
        });
    assert_eq!(response, expected_response);
}

/// Transforms the connection metadata for the given peers into
/// metadata expected by the peer monitoring service.
fn transform_connection_metadata(
    expected_peers: BTreeMap<PeerNetworkId, ConnectionMetadata>,
) -> BTreeMap<PeerNetworkId, aptos_peer_monitoring_service_types::response::ConnectionMetadata> {
    expected_peers
        .into_iter()
        .map(|(peer_id, metadata)| {
            let connection_metadata =
                aptos_peer_monitoring_service_types::response::ConnectionMetadata::new(
                    metadata.addr,
                    metadata.remote_peer_id,
                    metadata.role,
                );
            (peer_id, connection_metadata)
        })
        .collect()
}

/// A simple utility function that sends a request for node info using the given
/// client, and verifies the response is correct.
async fn verify_node_information(
    client: &mut MockClient,
    highest_synced_epoch: u64,
    highest_synced_version: u64,
    ledger_timestamp_usecs: u64,
    lowest_available_version: u64,
    uptime: Duration,
) {
    // Send a request to fetch the node information
    let request = PeerMonitoringServiceRequest::GetNodeInformation;
    let response = client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response =
        PeerMonitoringServiceResponse::NodeInformation(NodeInformationResponse {
            build_information: aptos_build_info::get_build_information(),
            highest_synced_epoch,
            highest_synced_version,
            ledger_timestamp_usecs,
            lowest_available_version,
            uptime,
        });
    assert_eq!(response, expected_response);
}

// A wrapper around the inbound network interface/channel for easily sending
/// mock client requests to a peer monitoring service server.
struct MockClient {
    peer_manager_notifiers:
        HashMap<NetworkId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
}

impl MockClient {
    fn new(
        base_config: Option<BaseConfig>,
        peer_monitoring_config: Option<PeerMonitoringServiceConfig>,
        storage_reader: Option<StorageReader>,
    ) -> (
        Self,
        PeerMonitoringServiceServer<StorageReader>,
        MockTimeService,
        Arc<PeersAndMetadata>,
    ) {
        initialize_logger();

        // Create the node config
        let base_config = base_config.unwrap_or_default();
        let peer_monitoring_config = peer_monitoring_config.unwrap_or_default();
        let node_config = NodeConfig {
            base: base_config,
            peer_monitoring_service: peer_monitoring_config,
            ..Default::default()
        };

        // Setup the networks and the network events
        let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);
        let mut network_and_events = HashMap::new();
        let mut peer_manager_notifiers = HashMap::new();
        for network_id in network_ids {
            let queue_cfg = aptos_channel::Config::new(
                peer_monitoring_config.max_network_channel_size as usize,
            )
            .queue_style(QueueStyle::FIFO)
            .counters(&metrics::PENDING_PEER_MONITORING_SERVER_NETWORK_EVENTS);
            let (peer_manager_notifier, peer_manager_notification_receiver) = queue_cfg.build();

            let network_events = NetworkEvents::new(peer_manager_notification_receiver, None, true);
            network_and_events.insert(network_id, network_events);
            peer_manager_notifiers.insert(network_id, peer_manager_notifier);
        }
        let peer_monitoring_network_events =
            PeerMonitoringServiceNetworkEvents::new(NetworkServiceEvents::new(network_and_events));

        // Create the storage service
        let executor = tokio::runtime::Handle::current();
        let mock_time_service = TimeService::mock();
        let storage_reader =
            storage_reader.unwrap_or_else(|| StorageReader::new(Arc::new(create_mock_db_reader())));
        let peer_monitoring_server = PeerMonitoringServiceServer::new(
            node_config,
            executor,
            peer_monitoring_network_events,
            peers_and_metadata.clone(),
            storage_reader,
            mock_time_service.clone(),
        );

        // Create the client
        let mock_client = Self {
            peer_manager_notifiers,
        };

        (
            mock_client,
            peer_monitoring_server,
            mock_time_service.into_mock(),
            peers_and_metadata,
        )
    }

    /// Sends the specified request and returns the response from the server
    async fn send_request(
        &mut self,
        request: PeerMonitoringServiceRequest,
    ) -> Result<PeerMonitoringServiceResponse, PeerMonitoringServiceError> {
        let peer_id = PeerId::random();
        let protocol_id = ProtocolId::PeerMonitoringServiceRpc;
        let network_id = get_random_network_id();

        // Create an inbound RPC request
        let request_data = protocol_id
            .to_bytes(&PeerMonitoringServiceMessage::Request(request))
            .unwrap();
        let (request_sender, request_receiver) = oneshot::channel();
        let request_notification = ReceivedMessage {
            message: NetworkMessage::RpcRequest(RpcRequest {
                protocol_id,
                request_id: 42,
                priority: 0,
                raw_request: request_data.clone(),
            }),
            sender: PeerNetworkId::new(network_id, peer_id),
            receive_timestamp_micros: 0,
            rpc_replier: Some(Arc::new(request_sender)),
        };

        // Send the request to the peer monitoring service
        self.peer_manager_notifiers
            .get(&network_id)
            .unwrap()
            .push((peer_id, protocol_id), request_notification)
            .unwrap();

        // Wait for the response from the peer monitoring service
        let response_data = request_receiver.await.unwrap().unwrap();
        let response = protocol_id
            .from_bytes::<PeerMonitoringServiceMessage>(&response_data)
            .unwrap();
        match response {
            PeerMonitoringServiceMessage::Response(response) => response,
            _ => panic!("Unexpected response message: {:?}", response),
        }
    }
}

/// Initializes the Aptos logger for tests
pub fn initialize_logger() {
    aptos_logger::Logger::builder()
        .is_async(false)
        .level(Level::Debug)
        .build();
}

/// Returns a random network ID
fn get_random_network_id() -> NetworkId {
    let mut rng = OsRng;
    let random_number: u8 = rng.gen();
    match random_number % 3 {
        0 => NetworkId::Validator,
        1 => NetworkId::Vfn,
        2 => NetworkId::Public,
        num => panic!("This shouldn't be possible! Got num: {:?}", num),
    }
}

/// Creates a mock database reader
pub fn create_mock_db_reader() -> MockDatabaseReader {
    MockDatabaseReader::new()
}

// This automatically creates a MockDatabaseReader.
// TODO(joshlind): if we frequently use these mocks, we should define a single
// mock test crate to be shared across the codebase.
mod database_mock {
    use super::*;
    use aptos_storage_interface::Result;

    mock! {
        pub DatabaseReader {}
        impl DbReader for DatabaseReader {
            fn get_epoch_ending_ledger_infos(
                &self,
                start_epoch: u64,
                end_epoch: u64,
            ) -> Result<EpochChangeProof>;

            fn get_transactions(
                &self,
                start_version: Version,
                batch_size: u64,
                ledger_version: Version,
                fetch_events: bool,
            ) -> Result<TransactionListWithProof>;

            fn get_transaction_by_hash(
                &self,
                hash: HashValue,
                ledger_version: Version,
                fetch_events: bool,
            ) -> Result<Option<TransactionWithProof>>;

            fn get_transaction_by_version(
                &self,
                version: Version,
                ledger_version: Version,
                fetch_events: bool,
            ) -> Result<TransactionWithProof>;

            fn get_first_txn_version(&self) -> Result<Option<Version>>;

            fn get_first_write_set_version(&self) -> Result<Option<Version>>;

            fn get_transaction_outputs(
                &self,
                start_version: Version,
                limit: u64,
                ledger_version: Version,
            ) -> Result<TransactionOutputListWithProof>;

            fn get_events(
                &self,
                event_key: &EventKey,
                start: u64,
                order: Order,
                limit: u64,
                ledger_version: Version,
            ) -> Result<Vec<EventWithVersion>>;

            fn get_block_timestamp(&self, version: u64) -> Result<u64>;

            fn get_last_version_before_timestamp(
                &self,
                _timestamp: u64,
                _ledger_version: Version,
            ) -> Result<Version>;

            fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>>;

            fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures>;

            fn get_synced_version(&self) -> Result<Option<Version>>;

            fn get_latest_ledger_info_version(&self) -> Result<Version>;

            fn get_latest_commit_metadata(&self) -> Result<(Version, u64)>;

            fn get_account_transaction(
                &self,
                address: AccountAddress,
                seq_num: u64,
                include_events: bool,
                ledger_version: Version,
            ) -> Result<Option<TransactionWithProof>>;

            fn get_account_transactions(
                &self,
                address: AccountAddress,
                seq_num: u64,
                limit: u64,
                include_events: bool,
                ledger_version: Version,
            ) -> Result<AccountTransactionsWithProof>;

            fn get_state_proof_with_ledger_info(
                &self,
                known_version: u64,
                ledger_info: LedgerInfoWithSignatures,
            ) -> Result<StateProof>;

            fn get_state_proof(&self, known_version: u64) -> Result<StateProof>;

            fn get_state_value_with_proof_by_version(
                &self,
                state_key: &StateKey,
                version: Version,
            ) -> Result<(Option<StateValue>, SparseMerkleProof)>;

            fn get_pre_committed_ledger_summary(&self) -> Result<LedgerSummary>;

            fn get_epoch_ending_ledger_info(&self, known_version: u64) -> Result<LedgerInfoWithSignatures>;

            fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue>;

            fn get_accumulator_consistency_proof(
                &self,
                _client_known_version: Option<Version>,
                _ledger_version: Version,
            ) -> Result<AccumulatorConsistencyProof>;

            fn get_accumulator_summary(
                &self,
                ledger_version: Version,
            ) -> Result<TransactionAccumulatorSummary>;

            fn get_state_item_count(&self, version: Version) -> Result<usize>;

            fn get_state_value_chunk_with_proof(
                &self,
                version: Version,
                start_idx: usize,
                chunk_size: usize,
            ) -> Result<StateValueChunkWithProof>;

            fn get_epoch_snapshot_prune_window(&self) -> Result<usize>;
        }
    }
}
