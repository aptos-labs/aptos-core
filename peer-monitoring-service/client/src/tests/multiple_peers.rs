// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::key_value::PeerStateKey,
    tests::{
        mock::MockMonitoringServer,
        utils::{
            config_with_latency_ping_requests, config_with_network_info_requests,
            config_with_node_info_requests, create_connected_peers_map,
            create_network_info_response, create_random_node_info_response,
            elapse_latency_update_interval, elapse_metadata_updater_interval,
            elapse_network_info_update_interval, elapse_node_info_update_interval,
            get_distance_from_validators, handle_several_latency_pings,
            initialize_and_verify_peer_states, spawn_with_timeout, start_peer_metadata_updater,
            start_peer_monitor, update_latency_info_for_peer, update_network_info_for_peer,
            verify_empty_peer_states, verify_latency_request_and_respond,
            wait_for_monitoring_latency_update, wait_for_monitoring_network_update,
            wait_for_peer_state_update,
        },
    },
    PeerState,
};
use velor_config::{
    config::{NodeConfig, PeerMonitoringServiceConfig, PeerRole},
    network_id::NetworkId,
};
use velor_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest,
    response::{LatencyPingResponse, PeerMonitoringServiceResponse},
};
use velor_time_service::TimeServiceTrait;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test(flavor = "multi_thread")]
async fn test_peer_updater_loop_multiple_peers() {
    // Create the peer monitoring client and server
    let all_network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(all_network_ids.clone());

    // Verify peers and metadata is empty
    let peers_and_metadata = peer_monitoring_client.get_peers_and_metadata();
    assert!(peers_and_metadata.get_all_peers().is_empty());

    // Add a connected validator peer
    let validator_peer =
        mock_monitoring_server.add_new_peer(NetworkId::Validator, PeerRole::Validator);

    // Add a connected VFN peer
    let vfn_peer = mock_monitoring_server.add_new_peer(NetworkId::Vfn, PeerRole::ValidatorFullNode);

    // Add a connected fullnode peer
    let fullnode_peer = mock_monitoring_server.add_new_peer(NetworkId::Public, PeerRole::Unknown);

    // Create peer states for all the peers and update
    let node_config = NodeConfig::default();
    let all_peers = vec![validator_peer, vfn_peer, fullnode_peer];
    for peer in &all_peers {
        let peer_state = PeerState::new(node_config.clone(), time_service.clone());
        peer_monitor_state
            .peer_states
            .write()
            .insert(*peer, peer_state.clone());
    }

    // Update the latency ping info for all the peers
    let response_time_secs = 3.0;
    for peer in &all_peers {
        let mut peer_states = peer_monitor_state.peer_states.write();
        let peer_state = peer_states.get_mut(peer).unwrap();
        update_latency_info_for_peer(
            peers_and_metadata.clone(),
            peer,
            peer_state,
            0,
            0,
            response_time_secs,
        );
    }

    // Spawn the peer metadata updater
    let peers_and_metadata = peer_monitoring_client.get_peers_and_metadata();
    start_peer_metadata_updater(
        &peer_monitor_state,
        peers_and_metadata.clone(),
        &time_service,
        &node_config,
    )
    .await;

    // Elapse enough time for the metadata updater to run
    let mock_time = time_service.into_mock();
    elapse_metadata_updater_interval(node_config.clone(), mock_time.clone()).await;

    // Verify the peer metadata is updated for all peers
    for peer in &all_peers {
        wait_for_monitoring_latency_update(peers_and_metadata.clone(), peer, response_time_secs)
            .await;
    }

    // Update the network for all the peers
    for peer in &all_peers {
        // Get the peer state
        let mut peer_states = peer_monitor_state.peer_states.write();
        let peer_state = peer_states.get_mut(peer).unwrap();

        // Update the network info
        let distance_from_validators = get_distance_from_validators(peer);
        update_network_info_for_peer(
            peers_and_metadata.clone(),
            peer,
            peer_state,
            create_connected_peers_map(),
            distance_from_validators,
            1.0,
        );
    }

    // Disconnect the validator and VFN peers
    mock_monitoring_server.disconnect_peer(validator_peer);
    mock_monitoring_server.disconnect_peer(vfn_peer);

    // Elapse enough time for the metadata updater to run
    elapse_metadata_updater_interval(node_config.clone(), mock_time.clone()).await;

    // Verify the peer metadata is updated for all the peers (metadata
    // should always be updated, even if some peers are disconnected).
    for peer in &all_peers {
        let distance_from_validators = get_distance_from_validators(peer);
        wait_for_monitoring_network_update(
            peers_and_metadata.clone(),
            peer,
            distance_from_validators,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_initial_states() {
    // Create the peer monitoring client and server
    let all_network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(all_network_ids.clone());

    // Spawn the peer monitoring client with a very low monitoring interval
    let node_config = NodeConfig {
        peer_monitoring_service: PeerMonitoringServiceConfig {
            peer_monitor_interval_usec: 100,
            ..Default::default()
        },
        ..Default::default()
    };
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
    let validator_peer =
        mock_monitoring_server.add_new_peer(NetworkId::Validator, PeerRole::Validator);

    // Initialize all the validator states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Validator,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Add a connected VFN peer
    let vfn_peer = mock_monitoring_server.add_new_peer(NetworkId::Vfn, PeerRole::ValidatorFullNode);

    // Initialize all the VFN states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Vfn,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &vfn_peer,
        &mock_time,
    )
    .await;

    // Add a connected public fullnode peer
    let fullnode_peer = mock_monitoring_server.add_new_peer(NetworkId::Public, PeerRole::Unknown);

    // Initialize all the VFN states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Public,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &fullnode_peer,
        &mock_time,
    )
    .await;

    // Verify no pending messages
    for network_id in &[NetworkId::Validator, NetworkId::Vfn, NetworkId::Public] {
        mock_monitoring_server
            .verify_no_pending_requests(network_id)
            .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_latency_ping() {
    // Create the peer monitoring client and server
    let all_network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(all_network_ids.clone());

    // Create a node config where only latency pings refresh
    let node_config = config_with_latency_ping_requests();

    // Spawn the peer monitoring client
    start_peer_monitor(
        peer_monitoring_client,
        &peer_monitor_state,
        &time_service,
        &node_config,
    )
    .await;

    // Add a connected public fullnode
    let fullnode_peer_1 = mock_monitoring_server.add_new_peer(NetworkId::Public, PeerRole::Unknown);

    // Initialize all the fullnode states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Public,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &fullnode_peer_1,
        &mock_time,
    )
    .await;

    // Add a connected public fullnode
    let fullnode_peer_2 = mock_monitoring_server.add_new_peer(NetworkId::Public, PeerRole::Unknown);

    // Initialize all the fullnode states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Public,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &fullnode_peer_2,
        &mock_time,
    )
    .await;

    // Handle several latency info requests for the fullnodes
    let mock_monitoring_server = Arc::new(RwLock::new(mock_monitoring_server)).clone();
    for i in 0..10 {
        // Elapse enough time for a latency ping update
        let time_before_update = mock_time.now();
        elapse_latency_update_interval(node_config.clone(), mock_time.clone()).await;

        // Create a task that waits for the requests and sends responses
        let mock_monitoring_server = mock_monitoring_server.clone();
        let peer_monitor_state = peer_monitor_state.clone();
        let handle_requests = async move {
            // Create a response for the latency pings
            let response = PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
                ping_counter: i + 1,
            });

            // Verify that a latency ping is received for each peer
            for _ in 0..2 {
                // Get the network request
                let mut mock_monitoring_server = mock_monitoring_server.write().await;
                let network_request = mock_monitoring_server
                    .next_request(&NetworkId::Public)
                    .await
                    .unwrap();

                // Verify the request type and respond
                match network_request.peer_monitoring_service_request {
                    PeerMonitoringServiceRequest::LatencyPing(_) => {
                        network_request.response_sender.send(Ok(response.clone()));
                    },
                    request => panic!("Unexpected monitoring request received: {:?}", request),
                }
            }

            // Wait for the peer states to update
            for peer_network_id in &[fullnode_peer_1, fullnode_peer_2] {
                wait_for_peer_state_update(
                    time_before_update,
                    &peer_monitor_state,
                    peer_network_id,
                    vec![PeerStateKey::LatencyInfo],
                )
                .await;
            }
        };

        // Spawn the task with a timeout
        spawn_with_timeout(
            handle_requests,
            "Timed-out while waiting for the latency ping requests",
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_network_info() {
    // Create the peer monitoring client and server
    let all_network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(all_network_ids.clone());

    // Create a node config where only network infos refresh
    let node_config = config_with_network_info_requests();

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

    // Handle several network info requests for the validators
    let mock_monitoring_server = Arc::new(RwLock::new(mock_monitoring_server)).clone();
    for _ in 0..10 {
        // Elapse enough time for a network info update
        let time_before_update = mock_time.now();
        elapse_network_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Create a task that waits for the requests and sends responses
        let mock_monitoring_server = mock_monitoring_server.clone();
        let peer_monitor_state = peer_monitor_state.clone();
        let handle_requests = async move {
            // Create a response for the network info requests
            let response = PeerMonitoringServiceResponse::NetworkInformation(
                create_network_info_response(&create_connected_peers_map(), 0),
            );

            // Verify that a network info request is received for each peer
            for _ in 0..3 {
                // Get the network request
                let mut mock_monitoring_server = mock_monitoring_server.write().await;
                let network_request = mock_monitoring_server
                    .next_request(&NetworkId::Validator)
                    .await
                    .unwrap();

                // Verify the request type and respond
                match network_request.peer_monitoring_service_request {
                    PeerMonitoringServiceRequest::GetNetworkInformation => {
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
                    vec![PeerStateKey::NetworkInfo],
                )
                .await;
            }
        };

        // Spawn the task with a timeout
        spawn_with_timeout(
            handle_requests,
            "Timed-out while waiting for the network info requests",
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_node_info() {
    // Create the peer monitoring client and server
    let all_network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(all_network_ids.clone());

    // Create a node config where only node infos refresh
    let node_config = config_with_node_info_requests();

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

    // Handle several node info requests for the validators
    let mock_monitoring_server = Arc::new(RwLock::new(mock_monitoring_server)).clone();
    for _ in 0..10 {
        // Elapse enough time for a node info update
        let time_before_update = mock_time.now();
        elapse_node_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Create a task that waits for the requests and sends responses
        let mock_monitoring_server = mock_monitoring_server.clone();
        let peer_monitor_state = peer_monitor_state.clone();
        let handle_requests = async move {
            // Create a response for the node info requests
            let response =
                PeerMonitoringServiceResponse::NodeInformation(create_random_node_info_response());

            // Verify that a node info request is received for each peer
            for _ in 0..3 {
                // Get the node request
                let mut mock_monitoring_server = mock_monitoring_server.write().await;
                let network_request = mock_monitoring_server
                    .next_request(&NetworkId::Validator)
                    .await
                    .unwrap();

                // Verify the request type and respond
                match network_request.peer_monitoring_service_request {
                    PeerMonitoringServiceRequest::GetNodeInformation => {
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
                    vec![PeerStateKey::NodeInfo],
                )
                .await;
            }
        };

        // Spawn the task with a timeout
        spawn_with_timeout(
            handle_requests,
            "Timed-out while waiting for the node info requests",
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_peer_connections() {
    // Create the peer monitoring client and server
    let all_network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(all_network_ids.clone());

    // Create a node config where only latency pings refresh
    let node_config = config_with_latency_ping_requests();

    // Spawn the peer monitoring client
    start_peer_monitor(
        peer_monitoring_client,
        &peer_monitor_state,
        &time_service,
        &node_config,
    )
    .await;

    // Add a connected validator peer
    let validator_peer =
        mock_monitoring_server.add_new_peer(NetworkId::Validator, PeerRole::Validator);

    // Initialize all the validator states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Validator,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Add a connected VFN peer
    let vfn_peer = mock_monitoring_server.add_new_peer(NetworkId::Vfn, PeerRole::ValidatorFullNode);

    // Initialize all the VFN states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Vfn,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &vfn_peer,
        &mock_time,
    )
    .await;

    // Disconnect the validator peer
    mock_monitoring_server.disconnect_peer(validator_peer);

    // Handle several latency ping requests and responses for the VFN
    handle_several_latency_pings(
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &mock_time,
        &vfn_peer,
    )
    .await;

    // Disconnect the VFN and reconnect the validator peer
    mock_monitoring_server.disconnect_peer(vfn_peer);
    mock_monitoring_server.reconnected_peer(validator_peer);

    // Reinitialize the validator states (garbage collection has removed them)
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Validator,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Handle several latency ping requests and responses for the validator peer
    handle_several_latency_pings(
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &mock_time,
        &validator_peer,
    )
    .await;

    // Elapse enough time for a latency ping update
    elapse_latency_update_interval(node_config.clone(), mock_time.clone()).await;

    // Verify no pending messages for the validator
    mock_monitoring_server
        .verify_no_pending_requests(&NetworkId::Validator)
        .await;

    // Reconnect the VFN and reinitialize the VFN peer states
    mock_monitoring_server.reconnected_peer(vfn_peer);
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Vfn,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &vfn_peer,
        &mock_time,
    )
    .await;

    // Handle several latency ping requests and responses for the peers
    for i in 0..5 {
        // Elapse enough time for a latency ping update
        let time_before_update = mock_time.now();
        elapse_latency_update_interval(node_config.clone(), mock_time.clone()).await;

        // Handle the pings for the peers (they will have different ping counters)
        for (peer_network_id, expected_ping_counter) in
            &[(validator_peer, i + 6), (vfn_peer, i + 1)]
        {
            verify_latency_request_and_respond(
                &peer_network_id.network_id(),
                &mut mock_monitoring_server,
                *expected_ping_counter,
                false,
                false,
                false,
            )
            .await;

            wait_for_peer_state_update(
                time_before_update,
                &peer_monitor_state,
                peer_network_id,
                vec![PeerStateKey::LatencyInfo],
            )
            .await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_garbage_collection() {
    // Create the peer monitoring client and server
    let all_network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(all_network_ids.clone());

    // Create a node config where only latency pings refresh
    let node_config = config_with_latency_ping_requests();

    // Spawn the peer monitoring client
    start_peer_monitor(
        peer_monitoring_client,
        &peer_monitor_state,
        &time_service,
        &node_config,
    )
    .await;

    // Add a connected validator peer
    let validator_peer =
        mock_monitoring_server.add_new_peer(NetworkId::Validator, PeerRole::Validator);

    // Initialize all the validator states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Validator,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Verify that a peer state exists for the validator peer
    assert!(peer_monitor_state.get_peer_state(&validator_peer).is_some());

    // Add a connected VFN peer
    let vfn_peer = mock_monitoring_server.add_new_peer(NetworkId::Vfn, PeerRole::ValidatorFullNode);

    // Initialize all the VFN states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Vfn,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &vfn_peer,
        &mock_time,
    )
    .await;

    // Verify that a peer state exists for both the validator and VFN peers
    assert!(peer_monitor_state.get_peer_state(&validator_peer).is_some());
    assert!(peer_monitor_state.get_peer_state(&vfn_peer).is_some());

    // Add a connected fullnode peer
    let fullnode_peer = mock_monitoring_server.add_new_peer(NetworkId::Public, PeerRole::Unknown);

    // Initialize all the fullnode states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Public,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &fullnode_peer,
        &mock_time,
    )
    .await;

    // Verify that a peer state exists for all peers
    for peer in &[validator_peer, vfn_peer, fullnode_peer] {
        assert!(peer_monitor_state.get_peer_state(peer).is_some());
    }

    // Disconnect the validator and VFN peer
    mock_monitoring_server.disconnect_peer(validator_peer);
    mock_monitoring_server.disconnect_peer(vfn_peer);

    // Handle several latency ping requests and responses for the fullnode
    handle_several_latency_pings(
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &mock_time,
        &fullnode_peer,
    )
    .await;

    // Verify that garbage collection has removed only the validator and VFN peer states
    assert!(peer_monitor_state.get_peer_state(&validator_peer).is_none());
    assert!(peer_monitor_state.get_peer_state(&vfn_peer).is_none());
    assert!(peer_monitor_state.get_peer_state(&fullnode_peer).is_some());

    // Reconnect the validator peer
    mock_monitoring_server.reconnected_peer(validator_peer);

    // Reinitialize all the validator states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Validator,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Verify that we now have peer states for only the validator and fullnode peers
    assert!(peer_monitor_state.get_peer_state(&validator_peer).is_some());
    assert!(peer_monitor_state.get_peer_state(&vfn_peer).is_none());
    assert!(peer_monitor_state.get_peer_state(&fullnode_peer).is_some());

    // Reconnect the VFN peer and disconnect the fullnode peer
    mock_monitoring_server.reconnected_peer(vfn_peer);
    mock_monitoring_server.disconnect_peer(fullnode_peer);

    // Reinitialize all the VFN states by running the peer monitor once
    let _ = initialize_and_verify_peer_states(
        &NetworkId::Vfn,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &vfn_peer,
        &mock_time,
    )
    .await;

    // Verify that we now have peer states for only the validator and VFN peers
    assert!(peer_monitor_state.get_peer_state(&validator_peer).is_some());
    assert!(peer_monitor_state.get_peer_state(&vfn_peer).is_some());
    assert!(peer_monitor_state.get_peer_state(&fullnode_peer).is_none());
}
