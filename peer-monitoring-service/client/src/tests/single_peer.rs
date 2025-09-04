// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::key_value::PeerStateKey,
    tests::{
        mock::MockMonitoringServer,
        utils::{
            config_with_latency_ping_requests, config_with_network_info_requests,
            config_with_node_info_requests, config_with_only_latency_and_network_requests,
            create_connected_peers_map, create_network_info_response,
            create_random_network_info_response, create_random_node_info_response,
            elapse_latency_update_interval, elapse_metadata_updater_interval,
            elapse_network_info_update_interval, elapse_node_info_update_interval,
            initialize_and_verify_peer_states, start_peer_metadata_updater, start_peer_monitor,
            update_latency_info_for_peer, update_network_info_for_peer,
            verify_all_requests_and_respond, verify_and_handle_latency_ping,
            verify_and_handle_network_info_request, verify_and_handle_node_info_request,
            verify_empty_peer_states, verify_latency_request_and_respond,
            verify_network_info_request_and_respond, verify_node_info_request_and_respond,
            verify_peer_latency_state, verify_peer_network_state, verify_peer_node_state,
            wait_for_latency_ping_failure, wait_for_monitoring_latency_update,
            wait_for_monitoring_network_update, wait_for_network_info_request_failure,
            wait_for_node_info_request_failure, wait_for_peer_state_update,
        },
    },
    PeerState,
};
use velor_config::{
    config::{NodeConfig, PeerRole},
    network_id::NetworkId,
};
use velor_time_service::TimeServiceTrait;
use std::cmp::min;

#[tokio::test(flavor = "multi_thread")]
async fn test_basic_peer_monitor_loop() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

    // Create a node config where only latency and network requests are refreshed
    let node_config = config_with_only_latency_and_network_requests();

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
    let (network_info_response, _) = initialize_and_verify_peer_states(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Elapse enough time for a latency ping and verify correct execution
    verify_and_handle_latency_ping(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        1,
        2,
    )
    .await;

    // Elapse enough time for a network info request (which should also
    // trigger another latency ping).
    let time_before_update = mock_time.now();
    elapse_network_info_update_interval(node_config, mock_time).await;

    // Verify that both a latency and network request are received and respond
    verify_all_requests_and_respond(
        &network_id,
        &mut mock_monitoring_server,
        2,
        Some(network_info_response.clone()),
        None,
    )
    .await;

    // Wait until the network peer state is updated by the client
    wait_for_peer_state_update(
        time_before_update,
        &peer_monitor_state,
        &validator_peer,
        vec![PeerStateKey::LatencyInfo, PeerStateKey::NetworkInfo],
    )
    .await;

    // Verify the latency ping state
    verify_peer_latency_state(&peer_monitor_state, &validator_peer, 3, 0);

    // Verify the network state
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        network_info_response,
        0,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_basic_peer_updater_loop() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Public;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

    // Verify peers and metadata is empty
    let peers_and_metadata = peer_monitoring_client.get_peers_and_metadata();
    assert!(peers_and_metadata.get_all_peers().is_empty());

    // Add a connected fullnode peer
    let fullnode_peer = mock_monitoring_server.add_new_peer(NetworkId::Public, PeerRole::Unknown);

    // Create a peer state for the fullnode
    let node_config = NodeConfig::default();
    let mut peer_state = PeerState::new(node_config.clone(), time_service.clone());
    peer_monitor_state
        .peer_states
        .write()
        .insert(fullnode_peer, peer_state.clone());

    // Update the latency ping info for the fullnode
    let response_time_secs = 1.0;
    update_latency_info_for_peer(
        peers_and_metadata,
        &fullnode_peer,
        &mut peer_state,
        0,
        0,
        response_time_secs,
    );

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

    // Verify the peer metadata is updated
    wait_for_monitoring_latency_update(
        peers_and_metadata.clone(),
        &fullnode_peer,
        response_time_secs,
    )
    .await;

    // Update the latency ping info for the fullnode several times
    for (i, (response_time_secs, new_average_secs)) in
        [(11.0, 6.0), (9.0, 7.0), (7.0, 7.0)].iter().enumerate()
    {
        // Update the latency ping info for the fullnode
        update_latency_info_for_peer(
            peers_and_metadata.clone(),
            &fullnode_peer,
            &mut peer_state,
            (i + 1) as u64,
            (i + 1) as u64,
            *response_time_secs,
        );

        // Elapse enough time for the metadata updater to run
        elapse_metadata_updater_interval(node_config.clone(), mock_time.clone()).await;

        // Verify the peer metadata is updated
        wait_for_monitoring_latency_update(
            peers_and_metadata.clone(),
            &fullnode_peer,
            *new_average_secs,
        )
        .await;
    }

    // Update the network info for the fullnode several times
    for distance_from_validators in 2..10 {
        // Update the network info for the fullnode
        update_network_info_for_peer(
            peers_and_metadata.clone(),
            &fullnode_peer,
            &mut peer_state,
            create_connected_peers_map(),
            distance_from_validators,
            1.0,
        );

        // Elapse enough time for the metadata updater to run
        elapse_metadata_updater_interval(node_config.clone(), mock_time.clone()).await;

        // Verify the peer metadata is updated
        wait_for_monitoring_network_update(
            peers_and_metadata.clone(),
            &fullnode_peer,
            distance_from_validators,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_latency_pings() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

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

    // Handle many latency ping requests and responses
    let latency_monitoring_config = &node_config.peer_monitoring_service.latency_monitoring;
    let max_num_pings_to_retain = latency_monitoring_config.max_num_latency_pings_to_retain as u64;
    for i in 0..max_num_pings_to_retain * 2 {
        verify_and_handle_latency_ping(
            &network_id,
            &mut mock_monitoring_server,
            &peer_monitor_state,
            &node_config,
            &validator_peer,
            &mock_time,
            i + 1,
            min(i + 2, max_num_pings_to_retain), // Only retain max number of pings
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_latency_ping_failures() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

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

    // Handle several latency ping requests with bad responses
    for i in 0..5 {
        // Elapse enough time for a latency ping update
        elapse_latency_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single latency request is received and send a bad response
        verify_latency_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            i + 1,
            false,
            true,
            false,
        )
        .await;

        // Wait until the latency peer state is updated with the failure
        wait_for_latency_ping_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Handle several latency ping requests with invalid counter responses
    for i in 5..10 {
        // Elapse enough time for a latency ping update
        elapse_latency_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single latency request is received and send an invalid counter response
        verify_latency_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            i + 1,
            true,
            false,
            false,
        )
        .await;

        // Wait until the latency peer state is updated with the failure
        wait_for_latency_ping_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Verify the new latency state of the peer monitor
    verify_peer_latency_state(&peer_monitor_state, &validator_peer, 1, 10);

    // Elapse enough time for a latency ping and perform a successful execution
    verify_and_handle_latency_ping(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        11,
        2,
    )
    .await;

    // Verify the new latency state of the peer monitor (the number
    // of failures should have been reset).
    verify_peer_latency_state(&peer_monitor_state, &validator_peer, 2, 0);

    // Handle several latency ping requests without responses
    for i in 11..16 {
        // Elapse enough time for a latency ping update
        elapse_latency_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single latency request is received and don't send a response
        verify_latency_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            i + 1,
            false,
            false,
            true,
        )
        .await;

        // Wait until the latency peer state is updated with the failure
        wait_for_latency_ping_failure(&peer_monitor_state, &validator_peer, i - 10).await;
    }

    // Verify the new latency state of the peer monitor
    verify_peer_latency_state(&peer_monitor_state, &validator_peer, 2, 5);

    // Elapse enough time for a latency ping and perform a successful execution
    verify_and_handle_latency_ping(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        17,
        3,
    )
    .await;

    // Verify the new latency state of the peer monitor (the number
    // of failures should have been reset).
    verify_peer_latency_state(&peer_monitor_state, &validator_peer, 3, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_network_info_requests() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

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

    // Handle many network info requests and responses
    for _ in 0..20 {
        verify_and_handle_network_info_request(
            &network_id,
            &mut mock_monitoring_server,
            &peer_monitor_state,
            &node_config,
            &validator_peer,
            &mock_time,
            create_random_network_info_response(),
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_network_info_request_failures() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

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
    let validator_peer = mock_monitoring_server.add_new_peer(network_id, PeerRole::Validator);

    // Initialize all the peer states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let (network_info_response, _) = initialize_and_verify_peer_states(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Handle several network info requests with bad responses
    for i in 0..5 {
        // Elapse enough time for a network info update
        elapse_network_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single network info request is received and send a bad response
        verify_network_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            create_random_network_info_response(),
            false,
            true,
            false,
            false,
        )
        .await;

        // Wait until the network info state is updated with the failure
        wait_for_network_info_request_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Verify the new network info state of the peer monitor
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        network_info_response.clone(),
        5,
    );

    // Handle several network info requests with invalid depth responses
    for i in 5..10 {
        // Elapse enough time for a network info update
        elapse_network_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single network info request is received and send an invalid peer depth response
        verify_network_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            create_random_network_info_response(),
            true,
            false,
            false,
            false,
        )
        .await;

        // Wait until the network info state is updated with the failure
        wait_for_network_info_request_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Handle several network info requests with responses that are too large
    for i in 10..15 {
        // Elapse enough time for a network info update
        elapse_network_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single network info request is received and send a response that is too large
        verify_network_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            create_random_network_info_response(),
            false,
            false,
            true,
            false,
        )
        .await;

        // Wait until the network info state is updated with the failure
        wait_for_network_info_request_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Verify the new network info state of the peer monitor
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        network_info_response.clone(),
        15,
    );

    // Elapse enough time for a network info request and perform a successful execution
    let connected_peers = create_connected_peers_map();
    let network_info_response = create_network_info_response(
        &connected_peers,
        network_info_response.distance_from_validators,
    );
    verify_and_handle_network_info_request(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        network_info_response.clone(),
    )
    .await;

    // Verify the new network info state of the peer monitor (the number
    // of failures should have been reset).
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        network_info_response.clone(),
        0,
    );

    // Handle several network info requests without responses
    for i in 16..21 {
        // Elapse enough time for a network info update
        elapse_network_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single network info request is received and don't send a response
        verify_network_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            create_random_network_info_response(),
            false,
            false,
            false,
            true,
        )
        .await;

        // Wait until the network info state is updated with the failure
        wait_for_network_info_request_failure(&peer_monitor_state, &validator_peer, i - 15).await;
    }

    // Verify the new network info state of the peer monitor
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        network_info_response.clone(),
        5,
    );

    // Elapse enough time for a network info request and perform a successful execution
    verify_and_handle_network_info_request(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        network_info_response.clone(),
    )
    .await;

    // Verify the new network info state of the peer monitor (the number
    // of failures should have been reset).
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        network_info_response,
        0,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_node_info_requests() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

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

    // Handle many node info requests and responses
    for _ in 0..20 {
        verify_and_handle_node_info_request(
            &network_id,
            &mut mock_monitoring_server,
            &peer_monitor_state,
            &node_config,
            &validator_peer,
            &mock_time,
            create_random_node_info_response(),
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_node_info_request_failures() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

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
    let validator_peer = mock_monitoring_server.add_new_peer(network_id, PeerRole::Validator);

    // Initialize all the peer states by running the peer monitor once
    let mock_time = time_service.into_mock();
    let (_, node_info_response) = initialize_and_verify_peer_states(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
    )
    .await;

    // Handle several node info requests with bad responses
    for i in 0..5 {
        // Elapse enough time for a node info update
        elapse_node_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single node info request is received and send a bad response
        // Create the test data
        verify_node_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            create_random_node_info_response(),
            true,
            false,
            false,
        )
        .await;

        // Wait until the node info state is updated with the failure
        wait_for_node_info_request_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Handle several node info requests with responses that are too large
    for i in 5..10 {
        // Elapse enough time for a node info update
        elapse_node_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single node info request is received and send a response that is too large
        verify_node_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            create_random_node_info_response(),
            false,
            true,
            false,
        )
        .await;

        // Wait until the node info state is updated with the failure
        wait_for_node_info_request_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Handle several node info requests without responses
    for i in 10..15 {
        // Elapse enough time for a node info update
        elapse_node_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single node info request is received and don't send a response
        verify_node_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            create_random_node_info_response(),
            false,
            false,
            true,
        )
        .await;

        // Wait until the node info state is updated with the failure
        wait_for_node_info_request_failure(&peer_monitor_state, &validator_peer, i + 1).await;
    }

    // Verify the new node info state of the peer monitor
    verify_peer_node_state(
        &peer_monitor_state,
        &validator_peer,
        node_info_response.clone(),
        15,
    );

    // Elapse enough time for a node info request and perform a successful execution
    verify_and_handle_node_info_request(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        node_info_response.clone(),
    )
    .await;

    // Verify the new node info state of the peer monitor (the number
    // of failures should have been reset).
    verify_peer_node_state(&peer_monitor_state, &validator_peer, node_info_response, 0);
}
