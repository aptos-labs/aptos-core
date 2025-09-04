// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::key_value::{PeerStateKey, StateValueInterface},
    spawn_peer_metadata_updater, start_peer_monitor_with_state,
    tests::mock::MockMonitoringServer,
    PeerMonitorState, PeerMonitoringServiceClient, PeerState,
};
use velor_config::{
    config::{
        LatencyMonitoringConfig, NetworkMonitoringConfig, NodeConfig, NodeMonitoringConfig,
        PeerMonitoringServiceConfig, PeerRole,
    },
    network_id::{NetworkId, PeerNetworkId},
};
use velor_network::application::{interface::NetworkClient, storage::PeersAndMetadata};
use velor_peer_monitoring_service_types::{
    request::{LatencyPingRequest, PeerMonitoringServiceRequest},
    response::{
        ConnectionMetadata, LatencyPingResponse, NetworkInformationResponse,
        NodeInformationResponse, PeerMonitoringServiceResponse, ServerProtocolVersionResponse,
    },
    PeerMonitoringServiceMessage,
};
use velor_time_service::{MockTimeService, TimeService, TimeServiceTrait};
use velor_types::{network_address::NetworkAddress, PeerId};
use maplit::btreemap;
use rand::{rngs::OsRng, Rng};
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    runtime::Handle,
    time::{sleep, timeout},
};

// Useful test constants
const PAUSE_FOR_SETUP_SECS: u64 = 1;
const MAX_WAIT_TIME_SECS: u64 = 10;
const SLEEP_DURATION_MS: u64 = 500;
const UNREALISTIC_INTERVAL_MS: u64 = 1_000_000_000; // Unrealistically high interval

/// Returns a config where only latency pings are refreshed
pub fn config_with_latency_ping_requests() -> NodeConfig {
    NodeConfig {
        peer_monitoring_service: PeerMonitoringServiceConfig {
            network_monitoring: disabled_network_monitoring_config(),
            node_monitoring: disabled_node_monitoring_config(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Returns a config where only network infos are refreshed
pub fn config_with_network_info_requests() -> NodeConfig {
    NodeConfig {
        peer_monitoring_service: PeerMonitoringServiceConfig {
            latency_monitoring: disabled_latency_monitoring_config(),
            node_monitoring: disabled_node_monitoring_config(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Returns a config where only node infos are refreshed
pub fn config_with_node_info_requests() -> NodeConfig {
    NodeConfig {
        peer_monitoring_service: PeerMonitoringServiceConfig {
            latency_monitoring: disabled_latency_monitoring_config(),
            network_monitoring: disabled_network_monitoring_config(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Returns a config where node info requests don't refresh
pub fn config_with_only_latency_and_network_requests() -> NodeConfig {
    NodeConfig {
        peer_monitoring_service: PeerMonitoringServiceConfig {
            node_monitoring: disabled_node_monitoring_config(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Returns a simple connected peers map for testing purposes
pub fn create_connected_peers_map() -> BTreeMap<PeerNetworkId, ConnectionMetadata> {
    btreemap! { PeerNetworkId::random() => ConnectionMetadata::new(NetworkAddress::mock(), PeerId::random(), PeerRole::Unknown) }
}

/// Returns a build info map that is too large
pub fn create_large_build_info_map() -> BTreeMap<String, String> {
    let mut build_info = BTreeMap::new();
    for i in 0..100_000 {
        build_info.insert(i.to_string(), i.to_string());
    }
    build_info
}

/// Returns a connected peers map that is too large
pub fn create_large_connected_peers_map() -> BTreeMap<PeerNetworkId, ConnectionMetadata> {
    let mut peers = BTreeMap::new();
    for _ in 0..100_000 {
        peers.insert(
            PeerNetworkId::random(),
            ConnectionMetadata::new(NetworkAddress::mock(), PeerId::random(), PeerRole::Unknown),
        );
    }
    peers
}

/// Creates a network info response with the given data
pub fn create_network_info_response(
    connected_peers: &BTreeMap<PeerNetworkId, ConnectionMetadata>,
    distance_from_validators: u64,
) -> NetworkInformationResponse {
    NetworkInformationResponse {
        connected_peers: connected_peers.clone(),
        distance_from_validators,
    }
}

/// Creates a node info response with the given data
pub fn create_node_info_response(
    build_information: BTreeMap<String, String>,
    highest_synced_epoch: u64,
    highest_synced_version: u64,
    ledger_timestamp_usecs: u64,
    lowest_available_version: u64,
    uptime: Duration,
) -> NodeInformationResponse {
    NodeInformationResponse {
        build_information,
        highest_synced_epoch,
        highest_synced_version,
        ledger_timestamp_usecs,
        lowest_available_version,
        uptime,
    }
}

/// Returns a latency monitoring config where latency requests are disabled
pub fn disabled_latency_monitoring_config() -> LatencyMonitoringConfig {
    LatencyMonitoringConfig {
        latency_ping_interval_ms: UNREALISTIC_INTERVAL_MS,
        ..Default::default()
    }
}

/// Returns a network monitoring config where network infos are disabled
pub fn disabled_network_monitoring_config() -> NetworkMonitoringConfig {
    NetworkMonitoringConfig {
        network_info_request_interval_ms: UNREALISTIC_INTERVAL_MS,
        ..Default::default()
    }
}

/// Returns a node monitoring config where node infos are disabled
pub fn disabled_node_monitoring_config() -> NodeMonitoringConfig {
    NodeMonitoringConfig {
        node_info_request_interval_ms: UNREALISTIC_INTERVAL_MS,
        ..Default::default()
    }
}

/// Elapses enough time for a latency update to occur
pub async fn elapse_latency_update_interval(node_config: NodeConfig, mock_time: MockTimeService) {
    let latency_monitoring_config = node_config.peer_monitoring_service.latency_monitoring;
    mock_time
        .advance_ms_async(latency_monitoring_config.latency_ping_interval_ms + 1)
        .await;
}

/// Elapses enough time for the peer metadata updater loop to execute
pub async fn elapse_metadata_updater_interval(node_config: NodeConfig, mock_time: MockTimeService) {
    let peer_monitoring_config = node_config.peer_monitoring_service;
    mock_time
        .advance_ms_async(peer_monitoring_config.metadata_update_interval_ms + 1)
        .await;
}

/// Elapses enough time for a network info update to occur
pub async fn elapse_network_info_update_interval(
    node_config: NodeConfig,
    mock_time: MockTimeService,
) {
    let network_monitoring_config = node_config.peer_monitoring_service.network_monitoring;
    mock_time
        .advance_ms_async(network_monitoring_config.network_info_request_interval_ms + 1)
        .await;
}

/// Elapses enough time for a node info update to occur
pub async fn elapse_node_info_update_interval(node_config: NodeConfig, mock_time: MockTimeService) {
    let node_monitoring_config = node_config.peer_monitoring_service.node_monitoring;
    mock_time
        .advance_ms_async(node_monitoring_config.node_info_request_interval_ms + 1)
        .await;
}

/// Elapses enough time for the monitoring loop to execute
pub async fn elapse_peer_monitor_interval(node_config: NodeConfig, mock_time: MockTimeService) {
    let peer_monitoring_config = node_config.peer_monitoring_service;
    let peer_monitor_duration_ms = peer_monitoring_config.peer_monitor_interval_usec / 1000;
    mock_time
        .advance_ms_async(peer_monitor_duration_ms + 1)
        .await;
}

/// A simple helper function that returns a valid distance from
/// the validators based on the given peer.
pub fn get_distance_from_validators(peer_network_id: &PeerNetworkId) -> u64 {
    match peer_network_id.network_id() {
        NetworkId::Validator => 0,
        NetworkId::Vfn => 1,
        NetworkId::Public => 2,
    }
}

/// Returns a random u64 for test purposes
fn get_random_u64() -> u64 {
    OsRng.gen()
}

/// Handle several latency ping requests and responses for the given peer
pub async fn handle_several_latency_pings(
    mock_monitoring_server: &mut MockMonitoringServer,
    peer_monitor_state: &PeerMonitorState,
    node_config: &NodeConfig,
    mock_time: &MockTimeService,
    peer_network_id: &PeerNetworkId,
) {
    for i in 0..5 {
        verify_and_handle_latency_ping(
            &peer_network_id.network_id(),
            mock_monitoring_server,
            peer_monitor_state,
            node_config,
            peer_network_id,
            mock_time,
            i + 1,
            i + 2,
        )
        .await;
    }
}

/// Initializes all the peer states by running the peer monitor loop
/// once and ensuring the correct requests and responses are received.
/// Returns the network info and node info responses used during execution.
pub async fn initialize_and_verify_peer_states(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    peer_monitor_state: &PeerMonitorState,
    node_config: &NodeConfig,
    peer_network_id: &PeerNetworkId,
    mock_time: &MockTimeService,
) -> (NetworkInformationResponse, NodeInformationResponse) {
    // Create the network info response
    let distance_from_validators = get_distance_from_validators(peer_network_id);
    let network_info_response =
        create_network_info_response(&create_connected_peers_map(), distance_from_validators);

    // Create the node info response
    let node_info_response = create_random_node_info_response();

    // Elapse enough time for the peer monitor to execute
    let time_before_update = mock_time.now();
    elapse_peer_monitor_interval(node_config.clone(), mock_time.clone()).await;

    // Verify the initial client requests and send responses
    let num_expected_requests = PeerStateKey::get_all_keys().len() as u64;
    verify_all_requests_and_respond(
        network_id,
        mock_monitoring_server,
        num_expected_requests,
        Some(network_info_response.clone()),
        Some(node_info_response.clone()),
    )
    .await;

    // Wait until the peer state is updated by the client
    wait_for_peer_state_update(
        time_before_update,
        peer_monitor_state,
        peer_network_id,
        PeerStateKey::get_all_keys(),
    )
    .await;

    // Verify the new state of the peer monitor
    verify_peer_monitor_state(
        peer_monitor_state,
        peer_network_id,
        1,
        network_info_response.clone(),
        node_info_response.clone(),
    );

    (network_info_response, node_info_response)
}

/// Creates a new network info response with random values
pub fn create_random_network_info_response() -> NetworkInformationResponse {
    // Create the random values
    let connected_peers = create_connected_peers_map();
    let distance_from_validators = 0;

    // Create and return the network info response
    create_network_info_response(&connected_peers, distance_from_validators)
}

/// Creates a new network info response with random values
pub fn create_random_node_info_response() -> NodeInformationResponse {
    // Create the random values
    let build_information = velor_build_info::get_build_information();
    let highest_synced_epoch = get_random_u64();
    let highest_synced_version = get_random_u64();
    let ledger_timestamp_usecs = get_random_u64();
    let lowest_available_version = get_random_u64();
    let uptime = Duration::from_millis(get_random_u64());

    // Create and return the node info response
    create_node_info_response(
        build_information,
        highest_synced_epoch,
        highest_synced_version,
        ledger_timestamp_usecs,
        lowest_available_version,
        uptime,
    )
}

/// Spawns the given task with a timeout
pub async fn spawn_with_timeout(task: impl Future<Output = ()>, timeout_error_message: &str) {
    let timeout_duration = Duration::from_secs(MAX_WAIT_TIME_SECS);
    timeout(timeout_duration, task)
        .await
        .expect(timeout_error_message)
}

/// Spawns the peer metadata updater
pub async fn start_peer_metadata_updater(
    peer_monitor_state: &PeerMonitorState,
    peers_and_metadata: Arc<PeersAndMetadata>,
    time_service: &TimeService,
    node_config: &NodeConfig,
) {
    // Spawn the peer metadata updater
    tokio::spawn(spawn_peer_metadata_updater(
        node_config.peer_monitoring_service,
        peer_monitor_state.clone(),
        peers_and_metadata,
        time_service.clone(),
        Some(Handle::current()),
    ));

    // Wait for some time so that the peer metadata updater starts before we return
    sleep(Duration::from_secs(PAUSE_FOR_SETUP_SECS)).await
}

/// Spawns the peer monitor
pub async fn start_peer_monitor(
    peer_monitoring_client: PeerMonitoringServiceClient<
        NetworkClient<PeerMonitoringServiceMessage>,
    >,
    peer_monitor_state: &PeerMonitorState,
    time_service: &TimeService,
    node_config: &NodeConfig,
) {
    // Spawn the peer monitor state
    tokio::spawn(start_peer_monitor_with_state(
        node_config.clone(),
        peer_monitoring_client,
        peer_monitor_state.clone(),
        time_service.clone(),
        Some(Handle::current()),
    ));

    // Wait for some time so that the peer monitor starts before we return
    sleep(Duration::from_secs(PAUSE_FOR_SETUP_SECS)).await
}

/// Updates the latency info state for the peer
pub fn update_latency_info_for_peer(
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_network_id: &PeerNetworkId,
    peer_state: &mut PeerState,
    request_ping_counter: u64,
    response_ping_counter: u64,
    response_time_secs: f64,
) {
    // Get the latency info state
    let latency_info_state = peer_state
        .get_peer_state_value(&PeerStateKey::LatencyInfo)
        .unwrap();

    // Get the peer metadata
    let peer_metadata = peers_and_metadata
        .get_metadata_for_peer(*peer_network_id)
        .unwrap();

    // Create the latency info request and response
    let latency_info_request = PeerMonitoringServiceRequest::LatencyPing(LatencyPingRequest {
        ping_counter: request_ping_counter,
    });
    let latency_info_response = PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
        ping_counter: response_ping_counter,
    });

    // Update the latency info state
    latency_info_state
        .write()
        .handle_monitoring_service_response(
            peer_network_id,
            peer_metadata,
            latency_info_request,
            latency_info_response,
            response_time_secs,
        );
}

/// Updates the network info state for the peer
pub fn update_network_info_for_peer(
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_network_id: &PeerNetworkId,
    peer_state: &mut PeerState,
    connected_peers: BTreeMap<PeerNetworkId, ConnectionMetadata>,
    distance_from_validators: u64,
    response_time_secs: f64,
) {
    // Get the network info state
    let network_info_state = peer_state
        .get_peer_state_value(&PeerStateKey::NetworkInfo)
        .unwrap();

    // Get the peer metadata
    let peer_metadata = peers_and_metadata
        .get_metadata_for_peer(*peer_network_id)
        .unwrap();

    // Create the network info request and response
    let network_info_request = PeerMonitoringServiceRequest::GetNetworkInformation;
    let network_info_response = PeerMonitoringServiceResponse::NetworkInformation(
        create_network_info_response(&connected_peers, distance_from_validators),
    );

    // Update the network info state
    network_info_state
        .write()
        .handle_monitoring_service_response(
            peer_network_id,
            peer_metadata,
            network_info_request,
            network_info_response,
            response_time_secs,
        );
}

/// Elapses enough time for a latency ping and handles the ping
pub async fn verify_and_handle_latency_ping(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    peer_monitor_state: &PeerMonitorState,
    node_config: &NodeConfig,
    peer_network_id: &PeerNetworkId,
    mock_time: &MockTimeService,
    expected_latency_ping_counter: u64,
    expected_num_recorded_latency_pings: u64,
) {
    // Elapse enough time for a latency ping update
    let time_before_update = mock_time.now();
    elapse_latency_update_interval(node_config.clone(), mock_time.clone()).await;

    // Verify that a single latency request is received and respond
    verify_latency_request_and_respond(
        network_id,
        mock_monitoring_server,
        expected_latency_ping_counter,
        false,
        false,
        false,
    )
    .await;

    // Wait until the latency peer state is updated by the client
    wait_for_peer_state_update(
        time_before_update,
        peer_monitor_state,
        peer_network_id,
        vec![PeerStateKey::LatencyInfo],
    )
    .await;

    // Verify the latency ping state
    verify_peer_latency_state(
        peer_monitor_state,
        peer_network_id,
        expected_num_recorded_latency_pings,
        0,
    );
}

/// Elapses enough time for a network info request and handles the response
pub async fn verify_and_handle_network_info_request(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    peer_monitor_state: &PeerMonitorState,
    node_config: &NodeConfig,
    peer_network_id: &PeerNetworkId,
    mock_time: &MockTimeService,
    network_info_response: NetworkInformationResponse,
) {
    // Elapse enough time for a network info update
    let time_before_update = mock_time.now();
    elapse_network_info_update_interval(node_config.clone(), mock_time.clone()).await;

    // Verify that a single network info request is received and respond
    verify_network_info_request_and_respond(
        network_id,
        mock_monitoring_server,
        network_info_response.clone(),
        false,
        false,
        false,
        false,
    )
    .await;

    // Wait until the network info state is updated by the client
    wait_for_peer_state_update(
        time_before_update,
        peer_monitor_state,
        peer_network_id,
        vec![PeerStateKey::NetworkInfo],
    )
    .await;

    // Verify the network info state
    verify_peer_network_state(
        peer_monitor_state,
        peer_network_id,
        network_info_response,
        0,
    );
}

/// Elapses enough time for a node info request and handles the response
pub async fn verify_and_handle_node_info_request(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    peer_monitor_state: &PeerMonitorState,
    node_config: &NodeConfig,
    peer_network_id: &PeerNetworkId,
    mock_time: &MockTimeService,
    node_info_response: NodeInformationResponse,
) {
    // Elapse enough time for a node info update
    let time_before_update = mock_time.now();
    elapse_node_info_update_interval(node_config.clone(), mock_time.clone()).await;

    // Verify that a single node info request is received and respond
    verify_node_info_request_and_respond(
        network_id,
        mock_monitoring_server,
        node_info_response.clone(),
        false,
        false,
        false,
    )
    .await;

    // Wait until the network info state is updated by the client
    wait_for_peer_state_update(
        time_before_update,
        peer_monitor_state,
        peer_network_id,
        vec![PeerStateKey::NodeInfo],
    )
    .await;

    // Verify the network info state
    verify_peer_node_state(peer_monitor_state, peer_network_id, node_info_response, 0);
}

/// Verifies that all request types are received by the server
/// and responds to them using the specified data.
pub async fn verify_all_requests_and_respond(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    num_expected_requests: u64,
    network_information_response: Option<NetworkInformationResponse>,
    node_information_response: Option<NodeInformationResponse>,
) {
    // Create a task that waits for all the requests and sends responses
    let handle_requests = async move {
        // The set of requests already seen to ensure we only receive one of each request type
        let mut request_types_already_seen = HashSet::new();

        // Handle each request
        for _ in 0..num_expected_requests {
            // Get the network request
            let network_request = mock_monitoring_server
                .next_request(network_id)
                .await
                .unwrap();

            // Verify we haven't seen a request of this type before
            let request_type = network_request
                .peer_monitoring_service_request
                .get_label()
                .to_string();
            if request_types_already_seen.contains(&request_type) {
                panic!("Received duplicate requests of type: {:?}", request_type);
            } else {
                request_types_already_seen.insert(request_type);
            }

            // Process the peer monitoring request
            let response = match network_request.peer_monitoring_service_request {
                PeerMonitoringServiceRequest::GetNetworkInformation => {
                    PeerMonitoringServiceResponse::NetworkInformation(
                        network_information_response.clone().unwrap(),
                    )
                },
                PeerMonitoringServiceRequest::GetNodeInformation => {
                    PeerMonitoringServiceResponse::NodeInformation(
                        node_information_response.clone().unwrap(),
                    )
                },
                PeerMonitoringServiceRequest::LatencyPing(latency_ping) => {
                    PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
                        ping_counter: latency_ping.ping_counter,
                    })
                },
                request => panic!("Unexpected monitoring request received: {:?}", request),
            };

            // Send the response
            network_request.response_sender.send(Ok(response));
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        handle_requests,
        "Timed-out while waiting for all the requests!",
    )
    .await;
}

/// Verifies that the peer states is empty
pub fn verify_empty_peer_states(peer_monitor_state: &PeerMonitorState) {
    assert!(peer_monitor_state.peer_states.read().is_empty());
}

/// Verifies that a latency ping request is received and sends a
/// response based on the given parameters.
pub async fn verify_latency_request_and_respond(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    expected_ping_counter: u64,
    respond_with_invalid_counter: bool,
    respond_with_invalid_message: bool,
    skip_sending_a_response: bool,
) {
    // Create a task that waits for the request and sends a response
    let handle_request = async move {
        // Process the latency ping request
        let network_request = mock_monitoring_server
            .next_request(network_id)
            .await
            .unwrap();
        let response = match network_request.peer_monitoring_service_request {
            PeerMonitoringServiceRequest::LatencyPing(latency_ping) => {
                // Verify the ping counter
                assert_eq!(latency_ping.ping_counter, expected_ping_counter);

                // Create the response
                if respond_with_invalid_counter {
                    // Respond with an invalid ping counter
                    PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
                        ping_counter: 1010101,
                    })
                } else if respond_with_invalid_message {
                    // Respond with the wrong message type
                    PeerMonitoringServiceResponse::ServerProtocolVersion(
                        ServerProtocolVersionResponse { version: 999 },
                    )
                } else {
                    // Send a valid response
                    PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
                        ping_counter: latency_ping.ping_counter,
                    })
                }
            },
            request => panic!("Unexpected monitoring request received: {:?}", request),
        };

        // Send the response
        if !skip_sending_a_response {
            network_request.response_sender.send(Ok(response));
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        handle_request,
        "Timed-out while waiting for a latency ping request",
    )
    .await;
}

/// Verifies that a network info request is received by the
/// server and sends a response based on the given arguments.
pub async fn verify_network_info_request_and_respond(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    network_info_response: NetworkInformationResponse,
    respond_with_invalid_distance: bool,
    respond_with_invalid_message: bool,
    respond_with_large_message: bool,
    skip_sending_a_response: bool,
) {
    // Create a task that waits for the request and sends a response
    let handle_request = async move {
        // Process the network info request
        let network_request = mock_monitoring_server
            .next_request(network_id)
            .await
            .unwrap();
        let response = match network_request.peer_monitoring_service_request {
            PeerMonitoringServiceRequest::GetNetworkInformation => {
                if respond_with_invalid_distance {
                    // Respond with an invalid distance
                    PeerMonitoringServiceResponse::NetworkInformation(create_network_info_response(
                        &create_connected_peers_map(),
                        1,
                    ))
                } else if respond_with_invalid_message {
                    // Respond with the wrong message type
                    PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
                        ping_counter: 10,
                    })
                } else if respond_with_large_message {
                    // Respond with a large message
                    PeerMonitoringServiceResponse::NetworkInformation(create_network_info_response(
                        &create_large_connected_peers_map(),
                        network_info_response.distance_from_validators,
                    ))
                } else {
                    // Send a valid response
                    PeerMonitoringServiceResponse::NetworkInformation(network_info_response)
                }
            },
            request => panic!("Unexpected monitoring request received: {:?}", request),
        };

        // Send the response
        if !skip_sending_a_response {
            network_request.response_sender.send(Ok(response));
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        handle_request,
        "Timed-out while waiting for a network info request",
    )
    .await;
}

/// Verifies that a node info request is received by the
/// server and sends a response based on the given arguments.
pub async fn verify_node_info_request_and_respond(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    mut node_info_response: NodeInformationResponse,
    respond_with_invalid_message: bool,
    respond_with_large_message: bool,
    skip_sending_a_response: bool,
) {
    // Create a task that waits for the request and sends a response
    let handle_request = async move {
        // Process the node info request
        let network_request = mock_monitoring_server
            .next_request(network_id)
            .await
            .unwrap();
        let response = match network_request.peer_monitoring_service_request {
            PeerMonitoringServiceRequest::GetNodeInformation => {
                if respond_with_invalid_message {
                    // Respond with the wrong message type
                    PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
                        ping_counter: 10,
                    })
                } else if respond_with_large_message {
                    // Respond with a large message
                    node_info_response.build_information = create_large_build_info_map();
                    PeerMonitoringServiceResponse::NodeInformation(node_info_response)
                } else {
                    // Send a valid response
                    PeerMonitoringServiceResponse::NodeInformation(node_info_response)
                }
            },
            request => panic!("Unexpected monitoring request received: {:?}", request),
        };

        // Send the response
        if !skip_sending_a_response {
            network_request.response_sender.send(Ok(response));
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        handle_request,
        "Timed-out while waiting for a node info request",
    )
    .await;
}

/// Verifies the latency state of the peer monitor
pub fn verify_peer_latency_state(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    expected_num_recorded_latency_pings: u64,
    expected_num_consecutive_failures: u64,
) {
    // Fetch the peer monitoring metadata
    let peer_states = peer_monitor_state.peer_states.read();
    let peer_state = peer_states.get(peer_network_id).unwrap();

    // Verify the latency ping state
    let latency_info_state = peer_state.get_latency_info_state().unwrap();
    assert_eq!(
        latency_info_state.get_recorded_latency_pings().len(),
        expected_num_recorded_latency_pings as usize
    );

    // Verify the number of consecutive failures
    assert_eq!(
        latency_info_state
            .get_request_tracker()
            .read()
            .get_num_consecutive_failures(),
        expected_num_consecutive_failures
    );
}

/// Verifies the state of the peer monitor
pub fn verify_peer_monitor_state(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    expected_num_recorded_latency_pings: u64,
    expected_network_info_response: NetworkInformationResponse,
    expected_node_info_response: NodeInformationResponse,
) {
    // Verify the latency ping state
    verify_peer_latency_state(
        peer_monitor_state,
        peer_network_id,
        expected_num_recorded_latency_pings,
        0,
    );

    // Verify the network state
    verify_peer_network_state(
        peer_monitor_state,
        peer_network_id,
        expected_network_info_response,
        0,
    );

    // Verify the node state
    verify_peer_node_state(
        peer_monitor_state,
        peer_network_id,
        expected_node_info_response,
        0,
    );
}

/// Verifies the network state of the peer monitor
pub fn verify_peer_network_state(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    expected_network_info_response: NetworkInformationResponse,
    expected_num_consecutive_failures: u64,
) {
    // Fetch the peer monitoring metadata
    let peer_states = peer_monitor_state.peer_states.read();
    let peer_state = peer_states.get(peer_network_id).unwrap();

    // Verify the latest network info response
    let network_info_state = peer_state.get_network_info_state().unwrap();
    let latest_network_info_response = network_info_state
        .get_latest_network_info_response()
        .unwrap();
    assert_eq!(latest_network_info_response, expected_network_info_response);

    // Verify the number of consecutive failures
    assert_eq!(
        network_info_state
            .get_request_tracker()
            .read()
            .get_num_consecutive_failures(),
        expected_num_consecutive_failures
    );
}

/// Verifies the node state of the peer monitor
pub fn verify_peer_node_state(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    expected_node_info_response: NodeInformationResponse,
    expected_num_consecutive_failures: u64,
) {
    // Fetch the peer monitoring metadata
    let peer_states = peer_monitor_state.peer_states.read();
    let peer_state = peer_states.get(peer_network_id).unwrap();

    // Verify the latest node info state
    let node_info_state = peer_state.get_node_info_state().unwrap();
    let latest_node_info_response = node_info_state.get_latest_node_info_response().unwrap();
    assert_eq!(latest_node_info_response, expected_node_info_response);

    // Verify the number of consecutive failures
    assert_eq!(
        node_info_state
            .get_request_tracker()
            .read()
            .get_num_consecutive_failures(),
        expected_num_consecutive_failures
    );
}

/// Waits for the peer monitor state to be updated with
/// a latency ping failure.
pub async fn wait_for_latency_ping_failure(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    num_expected_consecutive_failures: u64,
) {
    wait_for_request_failure(
        peer_monitor_state,
        peer_network_id,
        PeerStateKey::LatencyInfo,
        num_expected_consecutive_failures,
    )
    .await;
}

/// Waits for the peer monitoring metadata to be updated
/// with new latency information.
pub async fn wait_for_monitoring_latency_update(
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_network_id: &PeerNetworkId,
    expected_average_latency_secs: f64,
) {
    // Create a task that waits for the updated latency information
    let wait_for_update = async move {
        loop {
            // Get the peer monitoring metadata
            let peer_metadata = peers_and_metadata
                .get_metadata_for_peer(*peer_network_id)
                .unwrap();
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();

            // Check if the average ping latency matches the expected value
            if let Some(average_ping_latency_secs) =
                peer_monitoring_metadata.average_ping_latency_secs
            {
                if average_ping_latency_secs == expected_average_latency_secs {
                    return; // The average latency info was updated!
                }
            }

            // Sleep for some time before retrying
            sleep(Duration::from_millis(SLEEP_DURATION_MS)).await;
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        wait_for_update,
        "Timed-out while waiting for new latency information!",
    )
    .await;
}

/// Waits for the peer monitoring metadata to be updated
/// with new network information.
pub async fn wait_for_monitoring_network_update(
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_network_id: &PeerNetworkId,
    expected_distance_from_validators: u64,
) {
    // Create a task that waits for the updated network information
    let wait_for_update = async move {
        loop {
            // Get the peer monitoring metadata
            let peer_metadata = peers_and_metadata
                .get_metadata_for_peer(*peer_network_id)
                .unwrap();
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();

            // Check if the distance from validators matches the expected value
            if let Some(ref latest_network_info_response) =
                peer_monitoring_metadata.latest_network_info_response
            {
                if latest_network_info_response.distance_from_validators
                    == expected_distance_from_validators
                {
                    return; // The network info was updated!
                }
            }

            // Sleep for some time before retrying
            sleep(Duration::from_millis(SLEEP_DURATION_MS)).await;
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        wait_for_update,
        "Timed-out while waiting for new network information!",
    )
    .await;
}

/// Waits for the peer monitor state to be updated with
/// a network info request failure.
pub async fn wait_for_network_info_request_failure(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    num_expected_consecutive_failures: u64,
) {
    wait_for_request_failure(
        peer_monitor_state,
        peer_network_id,
        PeerStateKey::NetworkInfo,
        num_expected_consecutive_failures,
    )
    .await;
}

/// Waits for the peer monitor state to be updated with
/// a node info request failure.
pub async fn wait_for_node_info_request_failure(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    num_expected_consecutive_failures: u64,
) {
    wait_for_request_failure(
        peer_monitor_state,
        peer_network_id,
        PeerStateKey::NodeInfo,
        num_expected_consecutive_failures,
    )
    .await;
}

#[allow(clippy::await_holding_lock)] // This appears to be a false positive!
/// Waits for the peer monitor state to be updated with
/// the specified request failure.
pub async fn wait_for_request_failure(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    peer_state_key: PeerStateKey,
    num_expected_consecutive_failures: u64,
) {
    // Create a task that waits for the updated states
    let wait_for_update = async move {
        loop {
            // Fetch the request tracker for the state
            let peers_states_lock = peer_monitor_state.peer_states.read();
            let peer_state = peers_states_lock.get(peer_network_id).unwrap();
            let request_tracker = peer_state.get_request_tracker(&peer_state_key).unwrap();
            drop(peers_states_lock);

            // Check if the request tracker failures matches the expected number
            let num_consecutive_failures = request_tracker.read().get_num_consecutive_failures();
            if num_consecutive_failures == num_expected_consecutive_failures {
                return; // The peer state was updated!
            }

            // Sleep for some time before retrying
            sleep(Duration::from_millis(SLEEP_DURATION_MS)).await;
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        wait_for_update,
        "Timed-out while waiting for a request failure!",
    )
    .await;
}

#[allow(clippy::await_holding_lock)] // This appears to be a false positive!
/// Waits for the peer monitor state to be updated with
/// metadata after the given timestamp.
pub async fn wait_for_peer_state_update(
    time_before_update: Instant,
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    peer_state_keys: Vec<PeerStateKey>,
) {
    // Create a task that waits for the updated states
    let wait_for_update = async move {
        // Go through all peer states and ensure each one is updated
        for peer_state_key in peer_state_keys {
            loop {
                // Fetch the request tracker for the peer state
                let peers_states_lock = peer_monitor_state.peer_states.read();
                let peer_state = peers_states_lock.get(peer_network_id).unwrap();
                let request_tracker = peer_state.get_request_tracker(&peer_state_key).unwrap();
                drop(peers_states_lock);

                // Check if the request tracker has a response with a newer timestamp
                if let Some(last_response_time) = request_tracker.read().get_last_response_time() {
                    if last_response_time > time_before_update {
                        break; // The peer state was updated!
                    }
                };

                // Sleep for some time before retrying
                sleep(Duration::from_millis(SLEEP_DURATION_MS)).await;
            }
        }
    };

    // Spawn the task with a timeout
    spawn_with_timeout(
        wait_for_update,
        "Timed-out while waiting for a peer state update!",
    )
    .await;
}
