// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::key_value::PeerStateKey,
    tests::{
        mock::MockMonitoringServer,
        utils::{
            elapse_latency_update_interval, elapse_metadata_updater_interval,
            elapse_network_info_update_interval, get_config_without_latency_pings,
            get_config_without_network_info_requests, initialize_and_verify_peer_states,
            start_peer_metadata_updater, start_peer_monitor, update_latency_info_for_peer,
            update_network_info_for_peer, verify_all_requests_and_respond,
            verify_and_handle_latency_ping, verify_and_handle_network_info_request,
            verify_empty_peer_states, verify_latency_request_and_respond,
            verify_network_info_request_and_respond, verify_peer_latency_state,
            verify_peer_monitor_state, verify_peer_network_state, wait_for_latency_ping_failure,
            wait_for_monitoring_latency_update, wait_for_monitoring_network_update,
            wait_for_network_info_request_failure, wait_for_peer_state_update,
        },
    },
    PeerState,
};
use aptos_config::{
    config::{NodeConfig, PeerRole},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_network::{application::metadata::PeerMetadata, transport::ConnectionMetadata};
use aptos_time_service::TimeServiceTrait;
use aptos_types::PeerId;
use maplit::hashmap;
use std::cmp::min;

#[tokio::test(flavor = "multi_thread")]
async fn test_basic_peer_monitor_loop() {
    // Create the peer monitoring client and server
    let network_id = NetworkId::Validator;
    let (peer_monitoring_client, mut mock_monitoring_server, peer_monitor_state, time_service) =
        MockMonitoringServer::new(vec![network_id]);

    // Spawn the peer monitoring client
    let node_config = NodeConfig::default();
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
    let (connected_peers_and_metadata, distance_from_validators) =
        initialize_and_verify_peer_states(
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
        &connected_peers_and_metadata,
        distance_from_validators,
    )
    .await;

    // Wait until the network peer state is updated by the client
    wait_for_peer_state_update(
        time_before_update,
        &peer_monitor_state,
        &validator_peer,
        PeerStateKey::get_all_keys(),
    )
    .await;

    // Verify the new state of the peer monitor
    verify_peer_monitor_state(
        &peer_monitor_state,
        &validator_peer,
        &connected_peers_and_metadata,
        distance_from_validators,
        3,
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
    assert!(peers_and_metadata.get_all_peers().unwrap().is_empty());

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
        let connected_peers_and_metadata = hashmap! { PeerNetworkId::random() => PeerMetadata::new(ConnectionMetadata::mock(PeerId::random())) };
        update_network_info_for_peer(
            peers_and_metadata.clone(),
            &fullnode_peer,
            &mut peer_state,
            connected_peers_and_metadata,
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

    // Create a node config where network info requests don't refresh
    let node_config = get_config_without_network_info_requests();

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

    // Create a node config where network info requests don't refresh
    let node_config = get_config_without_network_info_requests();

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

    // Create a node config where latency pings don't refresh
    let node_config = get_config_without_latency_pings();

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
    let distance_from_validators = 0;
    for _ in 0..20 {
        let connected_peers_and_metadata = hashmap! { PeerNetworkId::random() => PeerMetadata::new(ConnectionMetadata::mock(PeerId::random())) };
        verify_and_handle_network_info_request(
            &network_id,
            &mut mock_monitoring_server,
            &peer_monitor_state,
            &node_config,
            &validator_peer,
            &mock_time,
            &connected_peers_and_metadata,
            distance_from_validators,
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

    // Create a node config where latency pings don't refresh
    let node_config = get_config_without_latency_pings();

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
    let (connected_peers_and_metadata, distance_from_validators) =
        initialize_and_verify_peer_states(
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
        let connected_peers_and_metadata = hashmap! { PeerNetworkId::random() => PeerMetadata::new(ConnectionMetadata::mock(PeerId::random())) };
        verify_network_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            &connected_peers_and_metadata,
            distance_from_validators,
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
        &connected_peers_and_metadata,
        distance_from_validators,
        5,
    );

    // Handle several network info requests with invalid depth responses responses
    for i in 5..10 {
        // Elapse enough time for a network info update
        elapse_network_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single network info request is received and send an invalid peer depth response
        let connected_peers_and_metadata = hashmap! { PeerNetworkId::random() => PeerMetadata::new(ConnectionMetadata::mock(PeerId::random())) };
        verify_network_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            &connected_peers_and_metadata,
            distance_from_validators,
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
        &connected_peers_and_metadata,
        distance_from_validators,
        10,
    );

    // Elapse enough time for a network info request and perform a successful execution
    let connected_peers_and_metadata = hashmap! { PeerNetworkId::random() => PeerMetadata::new(ConnectionMetadata::mock(PeerId::random())) };
    verify_and_handle_network_info_request(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        &connected_peers_and_metadata,
        distance_from_validators,
    )
    .await;

    // Verify the new network info state of the peer monitor (the number
    // of failures should have been reset).
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        &connected_peers_and_metadata,
        distance_from_validators,
        0,
    );

    // Handle several network info requests without responses
    for i in 11..16 {
        // Elapse enough time for a network info update
        elapse_network_info_update_interval(node_config.clone(), mock_time.clone()).await;

        // Verify that a single network info request is received and don't send a response
        let connected_peers_and_metadata = hashmap! { PeerNetworkId::random() => PeerMetadata::new(ConnectionMetadata::mock(PeerId::random())) };
        verify_network_info_request_and_respond(
            &network_id,
            &mut mock_monitoring_server,
            &connected_peers_and_metadata,
            distance_from_validators,
            false,
            false,
            true,
        )
        .await;

        // Wait until the network info state is updated with the failure
        wait_for_network_info_request_failure(&peer_monitor_state, &validator_peer, i - 10).await;
    }

    // Verify the new network info state of the peer monitor
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        &connected_peers_and_metadata,
        distance_from_validators,
        5,
    );

    // Elapse enough time for a latency ping and perform a successful execution
    let connected_peers_and_metadata = hashmap! { PeerNetworkId::random() => PeerMetadata::new(ConnectionMetadata::mock(PeerId::random())) };
    verify_and_handle_network_info_request(
        &network_id,
        &mut mock_monitoring_server,
        &peer_monitor_state,
        &node_config,
        &validator_peer,
        &mock_time,
        &connected_peers_and_metadata,
        distance_from_validators,
    )
    .await;

    // Verify the new network info state of the peer monitor (the number
    // of failures should have been reset).
    verify_peer_network_state(
        &peer_monitor_state,
        &validator_peer,
        &connected_peers_and_metadata,
        distance_from_validators,
        0,
    );
}
