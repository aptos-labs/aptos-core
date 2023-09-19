// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::key_value::{PeerStateKey, StateValueInterface},
    tests::{
        mock::MockMonitoringServer,
        utils::{
            disabled_latency_monitoring_config, disabled_network_monitoring_config,
            disabled_node_monitoring_config, initialize_and_verify_peer_states, spawn_with_timeout,
            start_peer_monitor, verify_empty_peer_states, wait_for_peer_state_update,
            wait_for_request_failure,
        },
    },
    PeerMonitorState,
};
use aptos_config::{
    config::{NodeConfig, PeerMonitoringServiceConfig, PeerRole, PerformanceMonitoringConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_infallible::RwLock;
use aptos_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest,
    response::{LatencyPingResponse, PeerMonitoringServiceResponse, PerformanceMonitoringResponse},
};
use aptos_time_service::{MockTimeService, TimeServiceTrait};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread")]
async fn test_performance_monitoring_multiple_peers() {
    // Create the peer monitoring client and server
    let all_network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(all_network_ids.clone());

    // Create a node config where only performance monitoring requests refresh
    let node_config = config_with_performance_requests();

    // Spawn the peer monitoring client
    start_peer_monitor(
        peer_monitoring_client,
        &peer_monitor_state,
        &time_service,
        &node_config,
    )
    .await;

    // Add a connected validator peer
    let validator_peer_1 =
        mock_monitoring_server.add_new_peer(NetworkId::Validator, PeerRole::Validator);

    // Initialize all the validator states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Validator,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer_1,
        &mock_time,
    )
    .await;

    // Add another connected validator peer
    let validator_peer_2 =
        mock_monitoring_server.add_new_peer(NetworkId::Validator, PeerRole::Validator);

    // Initialize all the validator states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Validator,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer_2,
        &mock_time,
    )
    .await;

    // Add another connected validator peer
    let validator_peer_3 =
        mock_monitoring_server.add_new_peer(NetworkId::Validator, PeerRole::Validator);

    // Initialize all the validator states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Validator,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer_3,
        &mock_time,
    )
    .await;

    // Handle several performance requests for the validators
    let mock_monitoring_server = Arc::new(RwLock::new(mock_monitoring_server)).clone();
    for _ in 0..10 {
        // Elapse enough time for a performance update
        let time_before_update = mock_time.now();
        elapse_performance_update_interval(node_config.clone(), mock_time.clone()).await;

        // Create a task that waits for the requests and sends responses
        let mock_monitoring_server = mock_monitoring_server.clone();
        let peer_monitor_state = peer_monitor_state.clone();
        let handle_requests = async move {
            // Verify that a performance monitoring request is received for each peer
            for _ in 0..3 {
                // Get the performance request
                let network_request = mock_monitoring_server
                    .write()
                    .next_request(&NetworkId::Validator)
                    .await
                    .unwrap();

                // Verify the request type and respond
                match network_request.peer_monitoring_service_request {
                    PeerMonitoringServiceRequest::PerformanceMonitoringRequest(request) => {
                        // Create and send the performance monitoring response
                        let response = PeerMonitoringServiceResponse::PerformanceMonitoring(
                            PerformanceMonitoringResponse {
                                response_counter: request.request_counter,
                            },
                        );
                        network_request.response_sender.send(Ok(response.clone()));
                    },
                    request => panic!("Unexpected monitoring request received: {:?}", request),
                }
            }

            // Wait for the peer states to update
            for peer_network_id in &[validator_peer_1, validator_peer_2, validator_peer_3] {
                wait_for_peer_state_update(
                    time_before_update,
                    &peer_monitor_state,
                    peer_network_id,
                    vec![PeerStateKey::PerformanceMonitoring],
                )
                .await;
            }
        };

        // Spawn the task with a timeout
        spawn_with_timeout(
            handle_requests,
            "Timed-out while waiting for the performance monitoring requests",
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_performance_monitoring_requests() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

    // Create a node config where only performance monitoring requests refresh
    let node_config = config_with_performance_requests();

    // Spawn the peer monitoring client
    start_peer_monitor(
        peer_monitoring_client,
        &peer_monitor_state,
        &time_service,
        &node_config,
    )
    .await;

    // Verify the initial state of the peer monitor
    verify_empty_peer_states(&peer_monitor_state);

    // Add a connected validator peer
    let validator_peer = mock_monitoring_server.add_new_peer(network_id, PeerRole::Validator);

    // Initialize all the peer states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let _ = initialize_and_verify_peer_states(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Handle many performance requests and responses
    for i in 0..20 {
        verify_and_handle_performance_request(
            &network_id,
            &mut mock_monitoring_server,
            &peer_monitor_state,
            &node_config,
            &validator_peer,
            &mock_time,
            i + 1,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_performance_monitoring_request_failures() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

    // Create a node config where only performance monitoring requests refresh
    let node_config = config_with_performance_requests();

    // Spawn the peer monitoring client
    start_peer_monitor(
        peer_monitoring_client,
        &peer_monitor_state,
        &time_service,
        &node_config,
    )
    .await;

    // Add a connected validator peer
    let validator_peer = mock_monitoring_server.add_new_peer(network_id, PeerRole::Validator);

    // Initialize all the peer states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let _ = initialize_and_verify_peer_states(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Handle several performance monitoring requests with bad responses
    for i in 0..5 {
        // Elapse enough time for a performance update
        elapse_performance_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single performance monitoring request is received and send a bad response
        // Create the test data
        verify_performance_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            true,
            false,
        )
        .await;

        // Wait until the performance state is updated with the failure
        wait_for_performance_request_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Handle several performance monitoring requests without responses
    for i in 5..10 {
        // Elapse enough time for a performance update
        elapse_performance_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single performance monitoring request is received and send a bad response
        // Create the test data
        verify_performance_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            false,
            true,
        )
        .await;

        // Wait until the performance state is updated with the failure
        wait_for_performance_request_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Verify the new performance state of the peer monitor
    verify_performance_monitoring_state(&peer_monitor_state, &validator_peer, 0, 10);

    // Elapse enough time for a performance update
    verify_and_handle_performance_request(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        11,
    )
    .await;

    // Verify the new performance monitoring state of the peer monitor (the number
    // of failures should have been reset).
    verify_performance_monitoring_state(&peer_monitor_state, &validator_peer, 11, 0);
}

/// Returns a config where only performance infos are refreshed
fn config_with_performance_requests() -> NodeConfig {
    NodeConfig {
        peer_monitoring_service: PeerMonitoringServiceConfig {
            latency_monitoring: disabled_latency_monitoring_config(),
            network_monitoring: disabled_network_monitoring_config(),
            node_monitoring: disabled_node_monitoring_config(),
            performance_monitoring: PerformanceMonitoringConfig {
                direct_send_interval_usec: 10_000_000, // 10 seconds
                rpc_interval_usec: 10_000_000,         // 10 seconds
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Elapses enough time for a performance update to occur
async fn elapse_performance_update_interval(node_config: NodeConfig, mock_time: MockTimeService) {
    let performance_monitoring_config = node_config.peer_monitoring_service.performance_monitoring;
    let rpc_interval_ms = performance_monitoring_config.rpc_interval_usec / 1000;
    mock_time.advance_ms_async(rpc_interval_ms + 1).await;
}

/// Elapses enough time for a performance request and handles the response
async fn verify_and_handle_performance_request(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    peer_monitor_state: &PeerMonitorState,
    node_config: &NodeConfig,
    peer_network_id: &PeerNetworkId,
    mock_time: &MockTimeService,
    expected_num_sent_requests: u64,
) {
    // Elapse enough time for a performance update
    let time_before_update = mock_time.now();
    elapse_performance_update_interval(node_config.clone(), mock_time.clone()).await;

    // Verify that a single performance request is received and respond
    verify_performance_request_and_respond(network_id, mock_monitoring_server, false, false).await;

    // Wait until the performance monitoring state is updated by the client
    wait_for_peer_state_update(
        time_before_update,
        peer_monitor_state,
        peer_network_id,
        vec![PeerStateKey::PerformanceMonitoring],
    )
    .await;

    // Verify the performance monitoring state
    verify_performance_monitoring_state(
        peer_monitor_state,
        peer_network_id,
        expected_num_sent_requests,
        0,
    );
}

/// Verifies the performance monitoring state of the peer monitor
fn verify_performance_monitoring_state(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    expected_num_sent_requests: u64,
    expected_num_consecutive_failures: u64,
) {
    // Fetch the peer monitoring metadata
    let peer_states = peer_monitor_state.peer_states.read();
    let peer_state = peer_states.get(peer_network_id).unwrap();

    // Verify the performance monitoring state
    let performance_monitoring_state = peer_state.get_performance_monitoring_state().unwrap();
    let latest_performance_response = performance_monitoring_state
        .get_latest_performance_response()
        .unwrap();
    assert_eq!(
        latest_performance_response.response_counter,
        expected_num_sent_requests,
    );

    // Verify the number of consecutive failures
    assert_eq!(
        performance_monitoring_state
            .get_request_tracker()
            .read()
            .get_num_consecutive_failures(),
        expected_num_consecutive_failures
    );
}

/// Verifies that a performance monitoring request is received by the
/// server and sends a response based on the given arguments.
async fn verify_performance_request_and_respond(
    network_id: &NetworkId,
    mock_monitoring_server: &mut MockMonitoringServer,
    respond_with_invalid_message: bool,
    skip_sending_a_response: bool,
) {
    // Create a task that waits for the request and sends a response
    let handle_request = async move {
        // Process the performance request
        let network_request = mock_monitoring_server
            .next_request(network_id)
            .await
            .unwrap();
        let response = match network_request.peer_monitoring_service_request {
            PeerMonitoringServiceRequest::PerformanceMonitoringRequest(request) => {
                if respond_with_invalid_message {
                    // Respond with the wrong message type
                    PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
                        ping_counter: 10,
                    })
                } else {
                    // Send a valid response
                    let performance_monitoring_response = PerformanceMonitoringResponse {
                        response_counter: request.request_counter,
                    };
                    PeerMonitoringServiceResponse::PerformanceMonitoring(
                        performance_monitoring_response,
                    )
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
        "Timed-out while waiting for a performance monitoring request",
    )
    .await;
}

/// Waits for the peer monitor state to be updated with
/// a performance request failure.
async fn wait_for_performance_request_failure(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
    num_expected_consecutive_failures: u64,
) {
    wait_for_request_failure(
        peer_monitor_state,
        peer_network_id,
        PeerStateKey::PerformanceMonitoring,
        num_expected_consecutive_failures,
    )
    .await;
}
