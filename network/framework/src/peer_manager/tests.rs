// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    application::storage::PeersAndMetadata,
    constants,
    peer::DisconnectReason,
    peer_manager::{
        conn_notifs_channel, conn_notifs_channel::Receiver, error::PeerManagerError,
        ConnectionNotification, ConnectionRequest, PeerManager, PeerManagerRequest,
        TransportNotification,
    },
    protocols::wire::{
        handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        messaging::v1::{
            ErrorCode, MultiplexMessage, MultiplexMessageSink, MultiplexMessageStream,
            NetworkMessage,
        },
    },
    transport,
    transport::{Connection, ConnectionId, ConnectionMetadata},
    ProtocolId,
};
use anyhow::anyhow;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::{AccessControlPolicy, PeerRole, MAX_INBOUND_CONNECTIONS},
    network_id::{NetworkContext, NetworkId},
};
use aptos_infallible::Mutex;
use aptos_memsocket::MemorySocket;
use aptos_netcore::transport::{
    boxed::BoxedTransport, memory::MemoryTransport, ConnectionOrigin, TransportExt,
};
use aptos_time_service::TimeService;
use aptos_types::{account_address::AccountAddress, network_address::NetworkAddress, PeerId};
use bytes::Bytes;
use futures::{channel::oneshot, io::AsyncWriteExt, stream::StreamExt};
use maplit::hashset;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::Arc,
};
use tokio::runtime::{Handle, Runtime};
use tokio_util::compat::{
    FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt,
};

// Builds a concrete typed transport (instead of using impl Trait) for testing PeerManager.
// Specifically this transport is compatible with the `build_test_connection` test helper making
// it easy to build connections without going through the whole transport pipeline.
pub fn build_test_transport(
) -> BoxedTransport<Connection<MemorySocket>, impl ::std::error::Error + Sync + Send + 'static> {
    let memory_transport = MemoryTransport;

    memory_transport
        .and_then(move |socket, addr, origin| async move {
            Ok(Connection {
                socket,
                metadata: ConnectionMetadata::new(
                    PeerId::random(),
                    ConnectionId::default(),
                    addr,
                    origin,
                    MessagingProtocolVersion::V1,
                    ProtocolIdSet::mock(),
                    PeerRole::Unknown,
                ),
            })
        })
        .boxed()
}

fn build_test_connection() -> (MemorySocket, MemorySocket) {
    MemorySocket::new_pair()
}

fn ordered_peer_ids(num: usize) -> Vec<PeerId> {
    let mut ids = Vec::new();
    for _ in 0..num {
        ids.push(PeerId::random());
    }
    ids.sort();
    ids
}

fn build_test_peer_manager(
    executor: Handle,
    peer_id: PeerId,
) -> (
    PeerManager<
        BoxedTransport<Connection<MemorySocket>, impl std::error::Error + Sync + Send + 'static>,
        MemorySocket,
    >,
    aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerRequest>,
    aptos_channel::Sender<PeerId, ConnectionRequest>,
    conn_notifs_channel::Receiver,
) {
    let (peer_manager_request_tx, peer_manager_request_rx) =
        aptos_channel::new(QueueStyle::FIFO, 1, None);
    let (connection_reqs_tx, connection_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 1, None);
    let (hello_tx, _hello_rx) = aptos_channel::new(QueueStyle::FIFO, 1, None);
    let (conn_status_tx, conn_status_rx) = conn_notifs_channel::new();

    let network_id = NetworkId::Validator;
    let peer_manager = PeerManager::new(
        executor,
        TimeService::mock(),
        build_test_transport(),
        NetworkContext::mock_with_peer_id(peer_id),
        "/memory/0".parse().unwrap(),
        PeersAndMetadata::new(&[network_id]),
        peer_manager_request_rx,
        connection_reqs_rx,
        [(ProtocolId::DiscoveryDirectSend, hello_tx)]
            .iter()
            .cloned()
            .collect(),
        vec![conn_status_tx],
        constants::NETWORK_CHANNEL_SIZE,
        constants::MAX_FRAME_SIZE,
        constants::MAX_MESSAGE_SIZE,
        MAX_INBOUND_CONNECTIONS,
        None,       /* access_control_policy */
        Vec::new(), /* priority_peers */
    );

    (
        peer_manager,
        peer_manager_request_tx,
        connection_reqs_tx,
        conn_status_rx,
    )
}

async fn ping_pong(connection: &mut MemorySocket) -> Result<(), PeerManagerError> {
    let (read_half, write_half) = tokio::io::split(connection.compat());
    let mut msg_tx =
        MultiplexMessageSink::new(write_half.compat_write(), constants::MAX_FRAME_SIZE);
    let mut msg_rx = MultiplexMessageStream::new(read_half.compat(), constants::MAX_FRAME_SIZE);

    // Send a garbage frame to trigger an expected Error response message
    msg_tx
        .send_raw_frame(Bytes::from_static(&[255, 111]))
        .await?;
    let error_msg = msg_rx
        .next()
        .await
        .ok_or_else(|| PeerManagerError::Error(anyhow!("Failed to read pong msg")))??;
    assert_eq!(
        error_msg,
        MultiplexMessage::Message(NetworkMessage::Error(ErrorCode::parsing_error(255, 111)))
    );
    Ok(())
}

async fn assert_peer_disconnected_event(
    peer_id: PeerId,
    origin: ConnectionOrigin,
    reason: DisconnectReason,
    peer_manager: &mut PeerManager<
        BoxedTransport<Connection<MemorySocket>, impl std::error::Error + Sync + Send + 'static>,
        MemorySocket,
    >,
) {
    let connection_event = peer_manager.transport_notifs_rx.select_next_some().await;
    match &connection_event {
        TransportNotification::Disconnected(actual_metadata, actual_reason) => {
            assert_eq!(actual_metadata.remote_peer_id, peer_id);
            assert_eq!(*actual_reason, reason);
            assert_eq!(actual_metadata.origin, origin);
            peer_manager.handle_connection_event(connection_event);
        },
        event => {
            panic!("Expected a LostPeer event, received: {:?}", event);
        },
    }
}

// This helper function is used to help identify that the expected connection was dropped due
// to simultaneous dial tie-breaking.  It also checks the correct events were sent from the
// Peer actors to PeerManager's internal_event_rx.
async fn check_correct_connection_is_live(
    mut live_connection: MemorySocket,
    mut dropped_connection: MemorySocket,
    live_connection_origin: ConnectionOrigin,
    dropped_connection_origin: ConnectionOrigin,
    expected_peer_id: PeerId,
    requested_shutdown: bool,
    peer_manager: &mut PeerManager<
        BoxedTransport<Connection<MemorySocket>, impl std::error::Error + Sync + Send + 'static>,
        MemorySocket,
    >,
) {
    // If PeerManager needed to kill the existing connection we'll see a Requested shutdown
    // event
    if requested_shutdown {
        assert_peer_disconnected_event(
            expected_peer_id,
            dropped_connection_origin,
            DisconnectReason::RequestedByPeerManager,
            peer_manager,
        )
        .await;
    }
    // TODO: There's a race here since the connection may not have actually been closed yet.
    // We should not be able to send a ping on the dropped connection.
    let f_open_stream_on_dropped_conn: Result<(), PeerManagerError> = async move {
        // Send ping and wait for pong.
        ping_pong(&mut dropped_connection).await?;
        Ok(())
    }
    .await;
    assert!(f_open_stream_on_dropped_conn.is_err());

    let f_open_stream_on_live_conn: Result<(), PeerManagerError> = async move {
        // Send ping and wait for pong.
        ping_pong(&mut live_connection).await?;
        // Close the connection.
        live_connection.close().await?;
        Ok(())
    }
    .await;
    assert!(f_open_stream_on_live_conn.is_ok());
    assert_peer_disconnected_event(
        expected_peer_id,
        live_connection_origin,
        DisconnectReason::ConnectionClosed,
        peer_manager,
    )
    .await;
}

fn create_connection<TSocket: transport::TSocket>(
    socket: TSocket,
    peer_id: PeerId,
    addr: NetworkAddress,
    origin: ConnectionOrigin,
    connection_id: ConnectionId,
) -> Connection<TSocket> {
    Connection {
        socket,
        metadata: ConnectionMetadata::new(
            peer_id,
            connection_id,
            addr,
            origin,
            MessagingProtocolVersion::V1,
            ProtocolIdSet::mock(),
            PeerRole::Unknown,
        ),
    }
}

#[test]
fn peer_manager_simultaneous_dial_two_inbound() {
    let runtime = create_test_runtime();

    // Create a list of ordered PeerIds so we can ensure how PeerIds will be compared.
    let ids = ordered_peer_ids(2);
    let (mut peer_manager, _request_tx, _connection_reqs_tx, _conn_statux_rx) =
        build_test_peer_manager(runtime.handle().clone(), ids[1]);

    let test = async move {
        //
        // Two inbound connections
        //
        let (outbound1, inbound1) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            inbound1,
            ids[0],
            Some("/ip6/::1/tcp/8080".parse().unwrap()),
            ConnectionOrigin::Inbound,
            0,
        );

        let (outbound2, inbound2) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            inbound2,
            ids[0],
            Some("/ip6/::1/tcp/8081".parse().unwrap()),
            ConnectionOrigin::Inbound,
            1,
        );

        // outbound1 should have been dropped since it was the older inbound connection
        check_correct_connection_is_live(
            outbound2,
            outbound1,
            ConnectionOrigin::Inbound,
            ConnectionOrigin::Inbound,
            ids[0],
            true,
            &mut peer_manager,
        )
        .await;
    };

    runtime.block_on(test);
}

#[test]
fn peer_manager_simultaneous_dial_inbound_outbound_remote_id_larger() {
    ::aptos_logger::Logger::init_for_testing();
    let runtime = create_test_runtime();

    // Create a list of ordered PeerIds so we can ensure how PeerIds will be compared.
    let ids = ordered_peer_ids(2);
    let (mut peer_manager, _request_tx, _connection_reqs_tx, _conn_status_rx) =
        build_test_peer_manager(runtime.handle().clone(), ids[0]);

    let test = async move {
        //
        // Inbound first, outbound second with own_peer_id < remote_peer_id
        //
        let (outbound1, inbound1) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            inbound1,
            ids[1],
            None,
            ConnectionOrigin::Inbound,
            0,
        );

        let (outbound2, inbound2) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            outbound2,
            ids[1],
            None,
            ConnectionOrigin::Outbound,
            1,
        );

        // inbound2 should be dropped because for outbound1 the remote peer has a greater
        // PeerId and is the "dialer"
        check_correct_connection_is_live(
            outbound1,
            inbound2,
            ConnectionOrigin::Inbound,
            ConnectionOrigin::Outbound,
            ids[1],
            false,
            &mut peer_manager,
        )
        .await;
    };

    runtime.block_on(test);
}

#[test]
fn peer_manager_simultaneous_dial_inbound_outbound_own_id_larger() {
    ::aptos_logger::Logger::init_for_testing();
    let runtime = create_test_runtime();

    // Create a list of ordered PeerIds so we can ensure how PeerIds will be compared.
    let ids = ordered_peer_ids(2);
    let (mut peer_manager, _request_tx, _connection_reqs_tx, _conn_status_rx) =
        build_test_peer_manager(runtime.handle().clone(), ids[1]);

    let test = async move {
        //
        // Inbound first, outbound second with remote_peer_id < own_peer_id
        //
        let (outbound1, inbound1) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            inbound1,
            ids[0],
            None,
            ConnectionOrigin::Inbound,
            0,
        );

        let (outbound2, inbound2) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            outbound2,
            ids[0],
            None,
            ConnectionOrigin::Outbound,
            1,
        );

        // outbound1 should be dropped because for inbound2 PeerManager's PeerId is greater and
        // is the "dialer"
        check_correct_connection_is_live(
            inbound2,
            outbound1,
            ConnectionOrigin::Outbound,
            ConnectionOrigin::Inbound,
            ids[0],
            true,
            &mut peer_manager,
        )
        .await;
    };

    runtime.block_on(test);
}

#[test]
fn peer_manager_simultaneous_dial_outbound_inbound_remote_id_larger() {
    ::aptos_logger::Logger::init_for_testing();
    let runtime = create_test_runtime();

    // Create a list of ordered PeerIds so we can ensure how PeerIds will be compared.
    let ids = ordered_peer_ids(2);
    let (mut peer_manager, _request_tx, _connection_reqs_tx, _conn_status_rx) =
        build_test_peer_manager(runtime.handle().clone(), ids[0]);

    let test = async move {
        //
        // Outbound first, inbound second with own_peer_id < remote_peer_id
        //
        let (outbound1, inbound1) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            outbound1,
            ids[1],
            None,
            ConnectionOrigin::Outbound,
            0,
        );

        let (outbound2, inbound2) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            inbound2,
            ids[1],
            None,
            ConnectionOrigin::Inbound,
            1,
        );

        // inbound1 should be dropped because for outbound2 the remote peer has a greater
        // PeerID and is the "dialer"
        check_correct_connection_is_live(
            outbound2,
            inbound1,
            ConnectionOrigin::Inbound,
            ConnectionOrigin::Outbound,
            ids[1],
            true,
            &mut peer_manager,
        )
        .await;
    };

    runtime.block_on(test);
}

#[test]
fn peer_manager_simultaneous_dial_outbound_inbound_own_id_larger() {
    ::aptos_logger::Logger::init_for_testing();
    let runtime = create_test_runtime();

    // Create a list of ordered PeerIds so we can ensure how PeerIds will be compared.
    let ids = ordered_peer_ids(2);
    let (mut peer_manager, _request_tx, _connection_reqs_tx, _conn_status_rx) =
        build_test_peer_manager(runtime.handle().clone(), ids[1]);

    let test = async move {
        //
        // Outbound first, inbound second with remote_peer_id < own_peer_id
        //
        let (outbound1, inbound1) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            outbound1,
            ids[0],
            None,
            ConnectionOrigin::Outbound,
            0,
        );

        let (outbound2, inbound2) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            inbound2,
            ids[0],
            None,
            ConnectionOrigin::Inbound,
            1,
        );

        // outbound2 should be dropped because for inbound1 PeerManager's PeerId is greater and
        // is the "dialer"
        check_correct_connection_is_live(
            inbound1,
            outbound2,
            ConnectionOrigin::Outbound,
            ConnectionOrigin::Inbound,
            ids[0],
            false,
            &mut peer_manager,
        )
        .await;
    };

    runtime.block_on(test);
}

#[test]
fn peer_manager_simultaneous_dial_two_outbound() {
    ::aptos_logger::Logger::init_for_testing();
    let runtime = create_test_runtime();

    // Create a list of ordered PeerIds so we can ensure how PeerIds will be compared.
    let ids = ordered_peer_ids(2);
    let (mut peer_manager, _request_tx, _connection_reqs_tx, _conn_status_rx) =
        build_test_peer_manager(runtime.handle().clone(), ids[1]);

    let test = async move {
        //
        // Two Outbound connections
        //
        let (outbound1, inbound1) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            outbound1,
            ids[0],
            None,
            ConnectionOrigin::Outbound,
            0,
        );

        let (outbound2, inbound2) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            outbound2,
            ids[0],
            None,
            ConnectionOrigin::Outbound,
            1,
        );
        // inbound1 should have been dropped since it was the older outbound connection
        check_correct_connection_is_live(
            inbound2,
            inbound1,
            ConnectionOrigin::Outbound,
            ConnectionOrigin::Outbound,
            ids[0],
            true,
            &mut peer_manager,
        )
        .await;
    };
    runtime.block_on(test);
}

#[test]
fn peer_manager_simultaneous_dial_disconnect_event() {
    let runtime = create_test_runtime();

    // Create a list of ordered PeerIds so we can ensure how PeerIds will be compared.
    let ids = ordered_peer_ids(2);
    let (mut peer_manager, _request_tx, _connection_reqs_tx, _conn_status_rx) =
        build_test_peer_manager(runtime.handle().clone(), ids[1]);

    let test = async move {
        let (outbound, _inbound) = build_test_connection();
        add_peer_to_manager(
            &mut peer_manager,
            outbound,
            ids[0],
            None,
            ConnectionOrigin::Outbound,
            1,
        );

        // Create a PeerDisconnect event with an older connection_id.  This would happen if the
        // Disconnected event from a closed connection arrives after the new connection has been
        // added to active_peers.
        let event = TransportNotification::Disconnected(
            ConnectionMetadata::new(
                ids[0],
                ConnectionId::from(0),
                NetworkAddress::mock(),
                ConnectionOrigin::Inbound,
                MessagingProtocolVersion::V1,
                ProtocolIdSet::mock(),
                PeerRole::Unknown,
            ),
            DisconnectReason::ConnectionClosed,
        );
        peer_manager.handle_connection_event(event);
        // The active connection should still remain.
        assert!(peer_manager.active_peers.contains_key(&ids[0]));
    };

    runtime.block_on(test);
}

#[test]
fn test_dial_disconnect() {
    ::aptos_logger::Logger::init_for_testing();
    let runtime = create_test_runtime();

    // Create a list of ordered PeerIds so we can ensure how PeerIds will be compared.
    let ids = ordered_peer_ids(2);
    let (mut peer_manager, _request_tx, _connection_reqs_tx, mut conn_status_rx) =
        build_test_peer_manager(runtime.handle().clone(), ids[1]);

    let test = async move {
        let (outbound, _inbound) = build_test_connection();
        // Trigger add_peer function PeerManager.
        add_peer_to_manager(
            &mut peer_manager,
            outbound,
            ids[0],
            None,
            ConnectionOrigin::Outbound,
            0,
        );

        // Expect NewPeer notification from PeerManager.
        let conn_notif = conn_status_rx.next().await.unwrap();
        assert!(matches!(conn_notif, ConnectionNotification::NewPeer(_, _)));

        // Send DisconnectPeer request to PeerManager.
        let (disconnect_resp_tx, disconnect_resp_rx) = oneshot::channel();
        peer_manager
            .handle_outbound_connection_request(ConnectionRequest::DisconnectPeer(
                ids[0],
                DisconnectReason::ConnectionClosed,
                disconnect_resp_tx,
            ))
            .await;

        // Send disconnected event from Peer to PeerManaager
        let event = TransportNotification::Disconnected(
            ConnectionMetadata::new(
                ids[0],
                ConnectionId::from(0),
                NetworkAddress::mock(),
                ConnectionOrigin::Outbound,
                MessagingProtocolVersion::V1,
                ProtocolIdSet::mock(),
                PeerRole::Unknown,
            ),
            DisconnectReason::RequestedByPeerManager,
        );
        peer_manager.handle_connection_event(event);

        // Expect LostPeer notification from PeerManager.
        let conn_notif = conn_status_rx.next().await.unwrap();
        assert!(matches!(conn_notif, ConnectionNotification::LostPeer(_, _)));

        // Sender of disconnect request should receive acknowledgement once connection is closed.
        disconnect_resp_rx.await.unwrap().unwrap();
    };

    runtime.block_on(test);
}

fn add_peer_to_manager<TSocket: transport::TSocket>(
    peer_manager: &mut PeerManager<
        BoxedTransport<Connection<TSocket>, impl Error + Sync + Send + 'static>,
        TSocket,
    >,
    socket: TSocket,
    peer_id: PeerId,
    network_address: Option<NetworkAddress>,
    connection_origin: ConnectionOrigin,
    connection_id: u32,
) {
    peer_manager
        .add_peer(create_connection(
            socket,
            peer_id,
            network_address.unwrap_or_else(NetworkAddress::mock),
            connection_origin,
            ConnectionId::from(connection_id),
        ))
        .unwrap();
}

#[test]
fn test_peer_manager_allow_list_accepts_allowed_peer() {
    // Create the test runtime
    let runtime = create_test_runtime();

    // Create an allow list with a single allowed peer
    let allowed_peer = PeerId::random();
    let allow_list = hashset! {allowed_peer};
    let access_control_policy = AccessControlPolicy::AllowList(allow_list);

    // Create the peer manager with the allow list
    let peer_manager = create_peer_manager_with_policy(&runtime, Some(access_control_policy));

    // Check that the allowed peer is accepted
    let result = peer_manager.check_peer_access_lists(&allowed_peer);
    assert!(result.is_ok());
}

#[test]
fn test_peer_manager_allow_list_rejects_non_allowed_peer() {
    // Create the test runtime
    let runtime = create_test_runtime();

    // Create an allow list with a single allowed peer
    let allowed_peer = PeerId::random();
    let non_allowed_peer = PeerId::random();
    let allow_list = hashset! {allowed_peer};
    let access_control_policy = AccessControlPolicy::AllowList(allow_list);

    // Create the peer manager with the allow list
    let peer_manager = create_peer_manager_with_policy(&runtime, Some(access_control_policy));

    // Try to check a non-allowed peer
    let result = peer_manager.check_peer_access_lists(&non_allowed_peer);
    assert!(result.is_err());
}

#[test]
fn test_peer_manager_block_list_rejects_blocked_peer() {
    // Create the test runtime
    let runtime = create_test_runtime();

    // Create a block list with a single blocked peer
    let blocked_peer = PeerId::random();
    let block_list = hashset! {blocked_peer};
    let access_control_policy = AccessControlPolicy::BlockList(block_list);

    // Create the peer manager with the block list
    let peer_manager = create_peer_manager_with_policy(&runtime, Some(access_control_policy));

    // Try to check a blocked peer
    let result = peer_manager.check_peer_access_lists(&blocked_peer);
    assert!(result.is_err());
}

#[test]
fn test_peer_manager_block_list_accepts_non_blocked_peer() {
    // Create the test runtime
    let runtime = create_test_runtime();

    // Create a block list with a single blocked peer
    let blocked_peer = PeerId::random();
    let non_blocked_peer = PeerId::random();
    let block_list = hashset! {blocked_peer};
    let access_control_policy = AccessControlPolicy::BlockList(block_list);

    // Create the peer manager with the block list
    let peer_manager = create_peer_manager_with_policy(&runtime, Some(access_control_policy));

    // Check a non-blocked peer
    let result = peer_manager.check_peer_access_lists(&non_blocked_peer);
    assert!(result.is_ok());
}

#[test]
fn test_peer_manager_no_policy_accepts_all_peers() {
    // Create the test runtime
    let runtime = create_test_runtime();

    // Create the peer manager with no access control policy
    let peer_manager = create_peer_manager_with_policy(&runtime, None);

    // Any peer should be allowed when there's no policy
    let random_peer1 = PeerId::random();
    let random_peer2 = PeerId::random();
    assert!(peer_manager.check_peer_access_lists(&random_peer1).is_ok());
    assert!(peer_manager.check_peer_access_lists(&random_peer2).is_ok());
}

/// Creates a peer manager with the specified access control policy
fn create_peer_manager_with_policy(
    runtime: &Runtime,
    policy: Option<AccessControlPolicy>,
) -> PeerManager<
    BoxedTransport<Connection<MemorySocket>, impl Error + Sync + Send + 'static>,
    MemorySocket,
> {
    // Create the network channels
    let (_, peer_manager_request_rx) =
        aptos_channel::new(QueueStyle::FIFO, constants::NETWORK_CHANNEL_SIZE, None);
    let (_, connection_reqs_rx) =
        aptos_channel::new(QueueStyle::FIFO, constants::NETWORK_CHANNEL_SIZE, None);
    let (conn_status_tx, _) = conn_notifs_channel::new();

    // Create the peer manager
    PeerManager::new(
        runtime.handle().clone(),
        TimeService::mock(),
        build_test_transport(),
        NetworkContext::mock_with_peer_id(PeerId::random()),
        "/memory/0".parse().unwrap(),
        PeersAndMetadata::new(&[NetworkId::Validator]),
        peer_manager_request_rx,
        connection_reqs_rx,
        HashMap::new(),
        vec![conn_status_tx],
        constants::NETWORK_CHANNEL_SIZE,
        constants::MAX_FRAME_SIZE,
        constants::MAX_MESSAGE_SIZE,
        MAX_INBOUND_CONNECTIONS,
        policy.map(std::sync::Arc::new),
        Vec::new(), /* priority_peers */
    )
}

/// Creates and returns a new tokio runtime for testing
fn create_test_runtime() -> Runtime {
    Runtime::new().unwrap()
}

#[test]
fn test_priority_peer_accepted_below_limit() {
    // Create a priority peer
    let runtime = create_test_runtime();
    let peer_ids = ordered_peer_ids(5);
    let priority_peer = peer_ids[0];

    // Create a peer manager with the priority peer and a 100 inbound connection limit
    let (mut peer_manager, _, _, _) = create_peer_manager_with_priority_peers(
        runtime.handle().clone(),
        PeerId::random(),
        vec![priority_peer],
        100,
    );

    // Create an inbound connection for the priority peer
    let (inbound, _) = build_test_connection();
    let connection = Connection {
        socket: inbound,
        metadata: ConnectionMetadata::new(
            priority_peer,
            ConnectionId::default(),
            "/memory/0".parse().unwrap(),
            ConnectionOrigin::Inbound,
            MessagingProtocolVersion::V1,
            ProtocolIdSet::mock(),
            PeerRole::Unknown,
        ),
    };

    // Add the priority peer connection
    peer_manager.handle_new_connection_event(connection);

    // Verify it was accepted
    assert!(peer_manager.active_peers.contains_key(&priority_peer));
}

#[test]
fn test_priority_peer_evicts_non_priority_peer() {
    // Create priority and non-priority peers
    let runtime = create_test_runtime();
    let peer_ids = ordered_peer_ids(5);
    let priority_peer = peer_ids[0];
    let non_priority_peer_1 = peer_ids[1];
    let non_priority_peer_2 = peer_ids[2];

    // Create a peer manager with the priority peer and a limit of 2 inbound connections
    let (mut peer_manager, _, _, mut connection_event_rx) = create_peer_manager_with_priority_peers(
        runtime.handle().clone(),
        PeerId::random(),
        vec![priority_peer],
        2,
    );

    runtime.block_on(async move {
        // Add a non-priority peer connection to fill one slot
        add_test_connection_to_manager(non_priority_peer_1, &mut peer_manager, 0);

        // Wait for a new peer notification
        wait_for_new_peer_notification(&mut connection_event_rx, non_priority_peer_1).await;

        // Add another non-priority peer connection to fill the second slot
        add_test_connection_to_manager(non_priority_peer_2, &mut peer_manager, 1);

        // Wait for a new peer notification
        wait_for_new_peer_notification(&mut connection_event_rx, non_priority_peer_2).await;

        // Verify both non-priority peers are connected
        assert_eq!(peer_manager.active_peers.len(), 2);
        assert!(peer_manager.active_peers.contains_key(&non_priority_peer_1));
        assert!(peer_manager.active_peers.contains_key(&non_priority_peer_2));

        // Now try to add a priority peer (it should evict one of the non-priority peers)
        let (_, inbound_priority) = build_test_connection();
        let priority_connection = Connection {
            socket: inbound_priority,
            metadata: ConnectionMetadata::new(
                priority_peer,
                ConnectionId::from(2),
                "/memory/0".parse().unwrap(),
                ConnectionOrigin::Inbound,
                MessagingProtocolVersion::V1,
                ProtocolIdSet::mock(),
                PeerRole::Unknown,
            ),
        };
        peer_manager.handle_new_connection_event(priority_connection);

        // Wait for a new peer notification
        wait_for_new_peer_notification(&mut connection_event_rx, priority_peer).await;

        // Verify priority peer is now connected and one non-priority peer was evicted
        assert_eq!(peer_manager.active_peers.len(), 2);
        assert!(peer_manager.active_peers.contains_key(&priority_peer));

        // Verify exactly one non-priority peer remains
        let non_priority_peers = [non_priority_peer_1, non_priority_peer_2];
        let non_priority_remaining = peer_manager
            .active_peers
            .keys()
            .filter(|&&pid| non_priority_peers.contains(&pid))
            .count();
        assert_eq!(non_priority_remaining, 1);
    });
}

#[test]
fn test_priority_peer_no_eviction_when_all_priority() {
    // Create priority peers
    let runtime = create_test_runtime();
    let peer_ids = ordered_peer_ids(5);
    let priority_peer_1 = peer_ids[0];
    let priority_peer_2 = peer_ids[1];
    let priority_peer_3 = peer_ids[2];

    // Create a peer manager with 3 priority peers and a limit of 2 inbound connections
    let (mut peer_manager, _, _, mut connection_event_rx) = create_peer_manager_with_priority_peers(
        runtime.handle().clone(),
        PeerId::random(),
        vec![priority_peer_1, priority_peer_2, priority_peer_3],
        2,
    );

    runtime.block_on(async move {
        // Add a priority peer connection to fill one slot
        add_test_connection_to_manager(priority_peer_1, &mut peer_manager, 0);

        // Wait for a new peer notification
        wait_for_new_peer_notification(&mut connection_event_rx, priority_peer_1).await;

        // Add another priority peer connection to fill the second slot
        add_test_connection_to_manager(priority_peer_2, &mut peer_manager, 1);

        // Wait for a new peer notification
        wait_for_new_peer_notification(&mut connection_event_rx, priority_peer_2).await;

        // Verify both priority peers are connected
        assert_eq!(peer_manager.active_peers.len(), 2);
        assert!(peer_manager.active_peers.contains_key(&priority_peer_1));
        assert!(peer_manager.active_peers.contains_key(&priority_peer_2));

        // Now try to add a third priority peer (it should be rejected)
        let (_, inbound_priority) = build_test_connection();
        let priority_connection = Connection {
            socket: inbound_priority,
            metadata: ConnectionMetadata::new(
                priority_peer_3,
                ConnectionId::from(2),
                "/memory/0".parse().unwrap(),
                ConnectionOrigin::Inbound,
                MessagingProtocolVersion::V1,
                ProtocolIdSet::mock(),
                PeerRole::Unknown,
            ),
        };
        peer_manager.handle_new_connection_event(priority_connection);

        // The connection should be rejected
        assert_eq!(peer_manager.active_peers.len(), 2);
        assert!(peer_manager.active_peers.contains_key(&priority_peer_1));
        assert!(peer_manager.active_peers.contains_key(&priority_peer_2));
        assert!(!peer_manager.active_peers.contains_key(&priority_peer_3));
    });
}

#[test]
fn test_eviction_randomness() {
    // Run the test multiple times to ensure that eviction is random among non-priority peers
    let evicted_peers = Arc::new(Mutex::new(HashSet::new()));
    for _ in 0..100 {
        // Create priority and non-priority peers
        let runtime = create_test_runtime();
        let peer_ids = ordered_peer_ids(6);
        let priority_peer = peer_ids[0];
        let non_priority_peer_1 = peer_ids[1];
        let non_priority_peer_2 = peer_ids[2];
        let non_priority_peer_3 = peer_ids[3];

        // Create a peer manager with the priority peer and a limit of 3 inbound connections
        let (mut peer_manager, _, _, mut connection_event_rx) =
            create_peer_manager_with_priority_peers(
                runtime.handle().clone(),
                PeerId::random(),
                vec![priority_peer],
                3,
            );

        let evicted_peers_clone = evicted_peers.clone();
        runtime.block_on(async move {
            // Add a non-priority peer connection to fill one slot
            add_test_connection_to_manager(non_priority_peer_1, &mut peer_manager, 0);
            wait_for_new_peer_notification(&mut connection_event_rx, non_priority_peer_1).await;

            // Add a second non-priority peer connection to fill the second slot
            add_test_connection_to_manager(non_priority_peer_2, &mut peer_manager, 1);
            wait_for_new_peer_notification(&mut connection_event_rx, non_priority_peer_2).await;

            // Add a third non-priority peer connection to fill the third slot
            add_test_connection_to_manager(non_priority_peer_3, &mut peer_manager, 2);
            wait_for_new_peer_notification(&mut connection_event_rx, non_priority_peer_3).await;

            // Now add a priority peer to trigger eviction
            let (_, inbound_priority) = build_test_connection();
            let priority_connection = Connection {
                socket: inbound_priority,
                metadata: ConnectionMetadata::new(
                    priority_peer,
                    ConnectionId::from(3),
                    "/memory/0".parse().unwrap(),
                    ConnectionOrigin::Inbound,
                    MessagingProtocolVersion::V1,
                    ProtocolIdSet::mock(),
                    PeerRole::Unknown,
                ),
            };
            peer_manager.handle_new_connection_event(priority_connection);

            // Wait for a new peer notification for the priority peer
            wait_for_new_peer_notification(&mut connection_event_rx, priority_peer).await;

            // Keep track of which peer was evicted
            let evicted_peer = vec![
                non_priority_peer_1,
                non_priority_peer_2,
                non_priority_peer_3,
            ]
            .into_iter()
            .find(|pid| !peer_manager.active_peers.contains_key(pid))
            .unwrap();
            evicted_peers_clone.lock().insert(evicted_peer);
        });
    }

    // With 100 trials and 3 evictable peers, we should see at least 2 different peers evicted
    let evicted_peers = evicted_peers.lock();
    assert!(
        evicted_peers.len() >= 2,
        "Expected to see at least 2 different peers evicted, but only saw: {:?}",
        evicted_peers
    );
}

#[test]
fn test_eviction_disconnect_cleanup() {
    // Create priority and non-priority peers
    let runtime = create_test_runtime();
    let peer_ids = ordered_peer_ids(4);
    let priority_peer = peer_ids[0];
    let non_priority_peer = peer_ids[1];

    // Create a peer manager with the priority peer and a limit of 1 inbound connection
    let (mut peer_manager, _, _, mut conn_status_rx) = create_peer_manager_with_priority_peers(
        runtime.handle().clone(),
        PeerId::random(),
        vec![priority_peer],
        1,
    );

    runtime.block_on(async move {
        // Add a non-priority peer
        add_test_connection_to_manager(non_priority_peer, &mut peer_manager, 0);
        wait_for_new_peer_notification(&mut conn_status_rx, non_priority_peer).await;

        // Verify the peer is in active_peers
        assert!(peer_manager.active_peers.contains_key(&non_priority_peer));
        let initial_peer_count = peer_manager.active_peers.len();

        // Add priority peer to trigger eviction
        let (_, inbound_priority) = build_test_connection();
        let priority_connection = Connection {
            socket: inbound_priority,
            metadata: ConnectionMetadata::new(
                priority_peer,
                ConnectionId::from(1),
                "/memory/0".parse().unwrap(),
                ConnectionOrigin::Inbound,
                MessagingProtocolVersion::V1,
                ProtocolIdSet::mock(),
                PeerRole::Unknown,
            ),
        };
        peer_manager.handle_new_connection_event(priority_connection);

        // Wait for new peer notification for the priority peer
        wait_for_new_peer_notification(&mut conn_status_rx, priority_peer).await;

        // Verify the evicted peer is completely removed
        assert!(!peer_manager.active_peers.contains_key(&non_priority_peer));
        assert!(peer_manager.active_peers.contains_key(&priority_peer));
        assert_eq!(peer_manager.active_peers.len(), initial_peer_count);
    });
}

#[test]
fn test_non_priority_peer_rejected_at_limit() {
    // Create priority and non-priority peers
    let runtime = create_test_runtime();
    let peer_ids = ordered_peer_ids(4);
    let priority_peer = peer_ids[0];
    let non_priority_peer1 = peer_ids[1];
    let non_priority_peer2 = peer_ids[2];

    // Create a peer manager with the priority peer and a limit of 1 inbound connection
    let (mut peer_manager, _, _, mut conn_status_rx) = create_peer_manager_with_priority_peers(
        runtime.handle().clone(),
        PeerId::random(),
        vec![priority_peer],
        1,
    );

    runtime.block_on(async move {
        // Add a non-priority peer to fill the limit
        add_test_connection_to_manager(non_priority_peer1, &mut peer_manager, 0);
        wait_for_new_peer_notification(&mut conn_status_rx, non_priority_peer1).await;

        // Try to add another non-priority peer (should be rejected since the limit is reached and it is not a priority peer)
        let (_outbound2, inbound2) = build_test_connection();
        let non_priority_connection = Connection {
            socket: inbound2,
            metadata: ConnectionMetadata::new(
                non_priority_peer2,
                ConnectionId::from(1),
                "/memory/0".parse().unwrap(),
                ConnectionOrigin::Inbound,
                MessagingProtocolVersion::V1,
                ProtocolIdSet::mock(),
                PeerRole::Unknown,
            ),
        };
        peer_manager.handle_new_connection_event(non_priority_connection);

        // The second non-priority peer should be rejected
        assert_eq!(peer_manager.active_peers.len(), 1);
        assert!(peer_manager.active_peers.contains_key(&non_priority_peer1));
        assert!(!peer_manager.active_peers.contains_key(&non_priority_peer2));
    });
}

/// Helper function to add a test connection for a given peer ID to the peer manager
fn add_test_connection_to_manager(
    peer_id: AccountAddress,
    peer_manager: &mut PeerManager<
        BoxedTransport<Connection<MemorySocket>, impl Error + Sync + Send + 'static>,
        MemorySocket,
    >,
    connection_id: u32,
) {
    let (_, inbound) = build_test_connection();
    add_peer_to_manager(
        peer_manager,
        inbound,
        peer_id,
        None,
        ConnectionOrigin::Inbound,
        connection_id,
    );
}

// Helper function to build a test peer manager with priority inbound peers, and
// a specified inbound connection limit. Returns the peer manager along with the
// channels for sending requests and receiving connection events.
fn create_peer_manager_with_priority_peers(
    executor: Handle,
    peer_id: PeerId,
    priority_inbound_peers: Vec<PeerId>,
    inbound_connection_limit: usize,
) -> (
    PeerManager<
        BoxedTransport<Connection<MemorySocket>, impl Error + Sync + Send + 'static>,
        MemorySocket,
    >,
    aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerRequest>,
    aptos_channel::Sender<PeerId, ConnectionRequest>,
    conn_notifs_channel::Receiver,
) {
    // Create the network channels
    let (peer_manager_request_tx, peer_manager_request_rx) =
        aptos_channel::new(QueueStyle::FIFO, 1, None);
    let (connection_reqs_tx, connection_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 1, None);
    let (upstream_handler_tx, _) = aptos_channel::new(QueueStyle::FIFO, 1, None);
    let (connection_event_tx, connection_event_rx) = conn_notifs_channel::new();

    // Create the peer manager with the specified priority inbound peers and limit
    let peer_manager = PeerManager::new(
        executor,
        TimeService::mock(),
        build_test_transport(),
        NetworkContext::mock_with_peer_id(peer_id),
        "/memory/0".parse().unwrap(),
        PeersAndMetadata::new(&[NetworkId::Validator]),
        peer_manager_request_rx,
        connection_reqs_rx,
        [(ProtocolId::DiscoveryDirectSend, upstream_handler_tx)]
            .iter()
            .cloned()
            .collect(),
        vec![connection_event_tx],
        constants::NETWORK_CHANNEL_SIZE,
        constants::MAX_FRAME_SIZE,
        constants::MAX_MESSAGE_SIZE,
        inbound_connection_limit,
        None, /* access_control_policy */
        priority_inbound_peers,
    );

    (
        peer_manager,
        peer_manager_request_tx,
        connection_reqs_tx,
        connection_event_rx,
    )
}

/// Helper function to wait for a NewPeer notification for a specific peer ID
async fn wait_for_new_peer_notification(
    connection_event_rx: &mut Receiver,
    expected_peer_id: AccountAddress,
) {
    match connection_event_rx.select_next_some().await {
        ConnectionNotification::NewPeer(metadata, _) => {
            assert_eq!(metadata.remote_peer_id, expected_peer_id);
        },
        notification => panic!(
            "Expected new peer notification, but got: {:?}",
            notification
        ),
    }
}
