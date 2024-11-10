// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    constants::{
        INBOUND_RPC_TIMEOUT_MS, MAX_CONCURRENT_INBOUND_RPCS, MAX_CONCURRENT_OUTBOUND_RPCS,
        MAX_FRAME_SIZE, MAX_MESSAGE_SIZE, NETWORK_CHANNEL_SIZE,
    },
    peer::{DisconnectReason, Peer, PeerRequest},
    peer_manager::TransportNotification,
    protocols::{
        direct_send::Message,
        network::{ReceivedMessage, SerializedRequest},
        rpc::{error::RpcError, OutboundRpcRequest},
        wire::{
            handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
            messaging::v1::{
                DirectSendMsg, MultiplexMessage, MultiplexMessageSink, MultiplexMessageStream,
                NetworkMessage, RpcRequest, RpcResponse,
            },
        },
    },
    transport::{Connection, ConnectionId, ConnectionMetadata},
    ProtocolId,
};
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::{config::PeerRole, network_id::NetworkContext};
use aptos_logger::info;
use aptos_memsocket::MemorySocket;
use aptos_netcore::transport::ConnectionOrigin;
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{network_address::NetworkAddress, PeerId};
use bytes::Bytes;
use futures::{
    channel::oneshot,
    future::{self, FutureExt},
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    stream::{StreamExt, TryStreamExt},
    SinkExt,
};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::runtime::{Handle, Runtime};
use tokio_util::compat::{
    FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt,
};

static PROTOCOL: ProtocolId = ProtocolId::MempoolDirectSend;

fn build_test_peer(
    executor: Handle,
    time_service: TimeService,
    origin: ConnectionOrigin,
    upstream_handlers: Arc<
        HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
    >,
) -> (
    Peer<MemorySocket>,
    PeerHandle,
    MemorySocket,
    aptos_channels::Receiver<TransportNotification<MemorySocket>>,
) {
    let (a, b) = MemorySocket::new_pair();
    let peer_id = PeerId::random();
    let connection = Connection {
        metadata: ConnectionMetadata::new(
            peer_id,
            ConnectionId::default(),
            NetworkAddress::from_str("/ip4/127.0.0.1/tcp/8081").unwrap(),
            origin,
            MessagingProtocolVersion::V1,
            ProtocolIdSet::empty(),
            PeerRole::Unknown,
        ),
        socket: a,
    };

    let (connection_notifs_tx, connection_notifs_rx) = aptos_channels::new_test(1);
    let (peer_reqs_tx, peer_reqs_rx) =
        aptos_channel::new(QueueStyle::FIFO, NETWORK_CHANNEL_SIZE, None);

    let peer = Peer::new(
        NetworkContext::mock(),
        executor,
        time_service,
        connection,
        connection_notifs_tx,
        peer_reqs_rx,
        upstream_handlers,
        Duration::from_millis(INBOUND_RPC_TIMEOUT_MS),
        MAX_CONCURRENT_INBOUND_RPCS,
        MAX_CONCURRENT_OUTBOUND_RPCS,
        MAX_FRAME_SIZE,
        MAX_MESSAGE_SIZE,
    );
    let peer_handle = PeerHandle(peer_reqs_tx);

    (peer, peer_handle, b, connection_notifs_rx)
}

fn build_test_connected_peers(
    executor: Handle,
    time_service: TimeService,
    upstream_handlers_a: Arc<
        HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
    >,
    upstream_handlers_b: Arc<
        HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
    >,
) -> (
    (
        Peer<MemorySocket>,
        PeerHandle,
        aptos_channels::Receiver<TransportNotification<MemorySocket>>,
    ),
    (
        Peer<MemorySocket>,
        PeerHandle,
        aptos_channels::Receiver<TransportNotification<MemorySocket>>,
    ),
) {
    let (peer_a, peer_handle_a, connection_a, connection_notifs_rx_a) = build_test_peer(
        executor.clone(),
        time_service.clone(),
        ConnectionOrigin::Inbound,
        upstream_handlers_a,
    );
    let (mut peer_b, peer_handle_b, _connection_b, connection_notifs_rx_b) = build_test_peer(
        executor,
        time_service,
        ConnectionOrigin::Outbound,
        upstream_handlers_b,
    );

    // Make sure both peers are connected
    peer_b.connection = Some(connection_a);
    (
        (peer_a, peer_handle_a, connection_notifs_rx_a),
        (peer_b, peer_handle_b, connection_notifs_rx_b),
    )
}

fn build_network_sink_stream(
    connection: &mut MemorySocket,
) -> (
    MultiplexMessageSink<impl AsyncWrite + '_>,
    MultiplexMessageStream<impl AsyncRead + '_>,
) {
    let (read_half, write_half) = tokio::io::split(connection.compat());
    let sink = MultiplexMessageSink::new(write_half.compat_write(), MAX_FRAME_SIZE);
    let stream = MultiplexMessageStream::new(read_half.compat(), MAX_FRAME_SIZE);
    (sink, stream)
}

async fn assert_disconnected_event(
    peer_id: PeerId,
    reason: DisconnectReason,
    connection_notifs_rx: &mut aptos_channels::Receiver<TransportNotification<MemorySocket>>,
) {
    match connection_notifs_rx.next().await {
        Some(TransportNotification::Disconnected(metadata, actual_reason)) => {
            assert_eq!(metadata.remote_peer_id, peer_id);
            assert_eq!(actual_reason, reason);
        },
        event => panic!("Expected a Disconnected, received: {:?}", event),
    }
}

#[derive(Clone)]
struct PeerHandle(aptos_channel::Sender<ProtocolId, PeerRequest>);

impl PeerHandle {
    fn send_direct_send(&mut self, message: Message) {
        self.0
            .push(message.protocol_id(), PeerRequest::SendDirectSend(message))
            .unwrap()
    }

    async fn send_rpc_request(
        &mut self,
        protocol_id: ProtocolId,
        data: Bytes,
        timeout: Duration,
    ) -> Result<Bytes, RpcError> {
        let (res_tx, res_rx) = oneshot::channel();
        let request = OutboundRpcRequest::new(protocol_id, data, res_tx, timeout);
        self.0.push(protocol_id, PeerRequest::SendRpc(request))?;
        let response_data = res_rx.await??;
        Ok(response_data)
    }
}

// Sending an outbound DirectSend should write it to the wire.
#[test]
fn peer_send_message() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let upstream_handlers = Arc::new(HashMap::new());
    let (peer, mut peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut client_sink, mut client_stream) = build_network_sink_stream(&mut connection);

    let send_msg = Message::new(PROTOCOL, Bytes::from("hello world"));
    let recv_msg = MultiplexMessage::Message(NetworkMessage::DirectSendMsg(DirectSendMsg {
        protocol_id: PROTOCOL,
        priority: 0,
        raw_msg: Vec::from("hello world"),
    }));

    let client = async {
        // Client should receive the direct send messages.
        for _ in 0..30 {
            let msg = client_stream.next().await.unwrap().unwrap();
            assert_eq!(msg, recv_msg);
        }
        // Client then closes the connection.
        client_sink.close().await.unwrap();
    };

    let server = async {
        // Server sends some direct send messages.
        for _ in 0..30 {
            peer_handle.send_direct_send(send_msg.clone());
        }
    };
    rt.block_on(future::join3(peer.start(), server, client));
}

fn test_upstream_handlers() -> (
    Arc<HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>>,
    aptos_channel::Receiver<(PeerId, ProtocolId), ReceivedMessage>,
) {
    let mut upstream_handlers = HashMap::new();
    let (sender, receiver) = aptos_channel::new(QueueStyle::FIFO, 100, None);
    upstream_handlers.insert(PROTOCOL, sender);
    let upstream_handlers = Arc::new(upstream_handlers);
    (upstream_handlers, receiver)
}

// Reading an inbound DirectSendMsg off the wire should notify the PeerManager of
// an inbound DirectSend.
#[test]
fn peer_recv_message() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let (upstream_handlers, mut receiver) = test_upstream_handlers();
    let (peer, _peer_handle, connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );

    let send_msg = MultiplexMessage::Message(NetworkMessage::DirectSendMsg(DirectSendMsg {
        protocol_id: PROTOCOL,
        priority: 0,
        raw_msg: Vec::from("hello world"),
    }));
    let recv_msg = NetworkMessage::DirectSendMsg(DirectSendMsg {
        protocol_id: PROTOCOL,
        priority: 0,
        raw_msg: Vec::from("hello world"),
    });

    let client = async move {
        info!("client start");
        let mut connection = MultiplexMessageSink::new(connection, MAX_FRAME_SIZE);
        for _ in 0..30 {
            // The client should then send the network message.
            connection.send(&send_msg).await.unwrap();
        }
        info!("client sent");
        // Client then closes connection.
        connection.close().await.unwrap();
        info!("client exiting");
    };

    let server = async move {
        info!("server start");
        for _ in 0..30 {
            // Wait to receive notification of DirectSendMsg from Peer.
            let received = receiver.next().await.unwrap();
            assert_eq!(recv_msg, received.message);
        }
        info!("server exiting");
    };
    info!("waiting");
    rt.block_on(future::join3(peer.start(), server, client));
    info!("done");
}

// Two connected Peer actors should be able to send/recv a DirectSend from each
// other and then shutdown gracefully.
#[test]
fn peers_send_message_concurrent() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let (upstream_handlers_a, mut prot_a_rx) = test_upstream_handlers();
    let (upstream_handlers_b, mut prot_b_rx) = test_upstream_handlers();
    let (
        (peer_a, mut peer_handle_a, mut connection_notifs_rx_a),
        (peer_b, mut peer_handle_b, mut connection_notifs_rx_b),
    ) = build_test_connected_peers(
        rt.handle().clone(),
        TimeService::mock(),
        upstream_handlers_a,
        upstream_handlers_b,
    );

    let remote_peer_id_a = peer_a.remote_peer_id();
    let remote_peer_id_b = peer_b.remote_peer_id();

    let test = async move {
        let msg_a = Message::new(PROTOCOL, Bytes::from("hello world"));
        let msg_b = Message::new(PROTOCOL, Bytes::from("namaste"));

        // Peer A -> msg_a -> Peer B
        peer_handle_a.send_direct_send(msg_a.clone());
        // Peer A <- msg_b <- Peer B
        peer_handle_b.send_direct_send(msg_b.clone());

        // Check that each peer received the other's message
        let notif_a = prot_a_rx.next().await;
        let notif_b = prot_b_rx.next().await;
        assert_eq!(
            notif_a.unwrap().message,
            NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: PROTOCOL,
                priority: 0,
                raw_msg: msg_b.data().clone().into(),
            })
        );
        assert_eq!(
            notif_b.unwrap().message,
            NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: PROTOCOL,
                priority: 0,
                raw_msg: msg_a.data().clone().into(),
            })
        );

        // Shut one peers and the other should shutdown due to ConnectionLost
        drop(peer_handle_a);

        // Check that we received both shutdown events
        assert_disconnected_event(
            remote_peer_id_a,
            DisconnectReason::RequestedByPeerManager,
            &mut connection_notifs_rx_a,
        )
        .await;
        assert_disconnected_event(
            remote_peer_id_b,
            DisconnectReason::ConnectionClosed,
            &mut connection_notifs_rx_b,
        )
        .await;
    };

    rt.block_on(future::join3(peer_a.start(), peer_b.start(), test));
}

#[test]
fn peer_recv_rpc() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let (upstream_handlers, mut prot_rx) = test_upstream_handlers();
    let (peer, _peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut client_sink, mut client_stream) = build_network_sink_stream(&mut connection);

    let send_msg = MultiplexMessage::Message(NetworkMessage::RpcRequest(RpcRequest {
        request_id: 123,
        protocol_id: PROTOCOL,
        priority: 0,
        raw_request: Vec::from("hello world"),
    }));
    let resp_msg = MultiplexMessage::Message(NetworkMessage::RpcResponse(RpcResponse {
        request_id: 123,
        priority: 0,
        raw_response: Vec::from("goodbye world"),
    }));

    let client = async move {
        for _ in 0..30 {
            // Client should send the rpc request.
            client_sink.send(&send_msg).await.unwrap();
            // Client should then receive the expected rpc response.
            let received = client_stream.next().await.unwrap().unwrap();
            assert_eq!(received, resp_msg);
        }
        // Client then closes connection.
        client_sink.close().await.unwrap();
    };
    let server = async move {
        for _ in 0..30 {
            // Wait to receive RpcRequest from Peer.
            let received = prot_rx.next().await.unwrap();
            let ReceivedMessage {
                message,
                sender: _sender,
                receive_timestamp_micros: _rx_at,
                rpc_replier,
            } = received;
            assert_eq!(
                message,
                NetworkMessage::RpcRequest(RpcRequest {
                    protocol_id: PROTOCOL,
                    request_id: 123,
                    priority: 0,
                    raw_request: Vec::from("hello world"),
                })
            );

            // Send response to rpc.
            match message {
                NetworkMessage::RpcRequest(_req) => {
                    let response = Ok(Bytes::from("goodbye world"));
                    let rpc_replier = Arc::into_inner(rpc_replier.expect("rpc without replier"))
                        .expect("Arc unpack fail");
                    rpc_replier.send(response).expect("rpc reply send fail")
                },
                msg => panic!("Unexpected NetworkMessage: {:?}", msg),
            }
        }
    };
    rt.block_on(future::join3(peer.start(), server, client));
}

#[test]
fn peer_recv_rpc_concurrent() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let (upstream_handlers, mut prot_rx) = test_upstream_handlers();
    let (peer, _peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut client_sink, mut client_stream) = build_network_sink_stream(&mut connection);

    let send_msg = MultiplexMessage::Message(NetworkMessage::RpcRequest(RpcRequest {
        request_id: 123,
        protocol_id: PROTOCOL,
        priority: 0,
        raw_request: Vec::from("hello world"),
    }));
    let resp_msg = MultiplexMessage::Message(NetworkMessage::RpcResponse(RpcResponse {
        request_id: 123,
        priority: 0,
        raw_response: Vec::from("goodbye world"),
    }));

    let client = async move {
        // The client should send many rpc requests.
        for _ in 0..30 {
            client_sink.send(&send_msg).await.unwrap();
        }

        // The client should then receive the expected rpc responses.
        for _ in 0..30 {
            let received = client_stream.next().await.unwrap().unwrap();
            assert_eq!(received, resp_msg);
        }

        // Client then closes connection.
        client_sink.close().await.unwrap();
    };
    let server = async move {
        let mut res_txs = vec![];

        // Wait to receive RpcRequests from Peer.
        for _ in 0..30 {
            let received = prot_rx.next().await.unwrap();
            match &received.message {
                NetworkMessage::RpcRequest(req) => {
                    assert_eq!(Vec::from("hello world"), req.raw_request);
                    let arcsender = received.rpc_replier.unwrap();
                    let sender = Arc::into_inner(arcsender).unwrap();
                    res_txs.push(sender)
                },
                _ => panic!("Unexpected NetworkMessage: {:?}", received),
            };
        }

        // Send all rpc responses to client.
        for res_tx in res_txs.into_iter() {
            let response = Bytes::from("goodbye world");
            res_tx.send(Ok(response)).unwrap();
        }
    };
    rt.block_on(future::join3(peer.start(), server, client));
}

#[test]
fn peer_recv_rpc_timeout() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let mock_time = MockTimeService::new();
    let (upstream_handlers, mut prot_rx) = test_upstream_handlers();
    let (peer, _peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        mock_time.clone().into(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut client_sink, client_stream) = build_network_sink_stream(&mut connection);

    let send_msg = MultiplexMessage::Message(NetworkMessage::RpcRequest(RpcRequest {
        request_id: 123,
        protocol_id: PROTOCOL,
        priority: 0,
        raw_request: Vec::from("hello world"),
    }));

    let test = async move {
        // Client sends the rpc request.
        client_sink.send(&send_msg).await.unwrap();

        // Server receives the rpc request from client.
        let received = prot_rx.next().await.unwrap();

        // Pull out the request completion handle.
        let mut res_tx = match &received.message {
            NetworkMessage::RpcRequest(req) => {
                assert_eq!(Vec::from("hello world"), req.raw_request);
                let arcsender = received.rpc_replier.unwrap();
                Arc::into_inner(arcsender).unwrap()
            },
            _ => panic!("Unexpected NetworkMessage: {:?}", received),
        };

        // The rpc response channel should still be open since we haven't timed out yet.
        assert!(!res_tx.is_canceled());

        // Advancing time should trigger the timeout.
        mock_time.advance_ms_async(INBOUND_RPC_TIMEOUT_MS).await;

        // The rpc response channel should be canceled from the timeout.
        assert!(res_tx.is_canceled());
        res_tx.cancellation().await;

        // Client then half-closes write side.
        client_sink.close().await.unwrap();

        // Client shouldn't have received any messages.
        let messages = client_stream.try_collect::<Vec<_>>().await.unwrap();
        assert_eq!(messages, vec![]);
    };
    rt.block_on(future::join(peer.start(), test));
}

#[test]
fn peer_recv_rpc_cancel() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let (upstream_handlers, mut prot_rx) = test_upstream_handlers();
    let (peer, _peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut client_sink, client_stream) = build_network_sink_stream(&mut connection);

    let send_msg = MultiplexMessage::Message(NetworkMessage::RpcRequest(RpcRequest {
        request_id: 123,
        protocol_id: PROTOCOL,
        priority: 0,
        raw_request: Vec::from("hello world"),
    }));

    let test = async move {
        // Client sends the rpc request.
        client_sink.send(&send_msg).await.unwrap();

        // Server receives the rpc request from client.
        let received = prot_rx.next().await.unwrap();

        // Pull out the request completion handle.
        let res_tx = match &received.message {
            NetworkMessage::RpcRequest(req) => {
                assert_eq!(Vec::from("hello world"), req.raw_request);
                let arcsender = received.rpc_replier.unwrap();
                Arc::into_inner(arcsender).unwrap()
            },
            _ => panic!("Unexpected NetworkMessage: {:?}", received),
        };

        // The rpc response channel should still be open since we haven't timed out yet.
        assert!(!res_tx.is_canceled());

        // Server drops the response completion handle to cancel the request.
        drop(res_tx);

        // Client then half-closes write side.
        client_sink.close().await.unwrap();

        // Client shouldn't have received any messages.
        let messages = client_stream.try_collect::<Vec<_>>().await.unwrap();
        assert_eq!(messages, vec![]);
    };
    rt.block_on(future::join(peer.start(), test));
}

#[test]
fn peer_send_rpc() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let upstream_handlers = Arc::new(HashMap::new());
    let (peer, mut peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut server_sink, mut server_stream) = build_network_sink_stream(&mut connection);
    let timeout = Duration::from_millis(10_000);

    let mut request_ids = HashSet::new();

    let client = async move {
        for _ in 0..30 {
            // Send RpcRequest to server and await response data.
            let response = peer_handle
                .send_rpc_request(PROTOCOL, Bytes::from(&b"hello world"[..]), timeout)
                .await
                .unwrap();
            assert_eq!(response, Bytes::from(&b"goodbye world"[..]));
        }
        // Client then closes connection.
    };
    let server = async move {
        for _ in 0..30 {
            // Server should then receive the expected rpc request.
            let received = server_stream.next().await.unwrap().unwrap();
            let received = match received {
                MultiplexMessage::Message(NetworkMessage::RpcRequest(request)) => request,
                _ => panic!("Expected RpcRequest; unexpected: {:?}", received),
            };

            assert_eq!(received.protocol_id, PROTOCOL);
            assert_eq!(received.priority, 0);
            assert_eq!(received.raw_request, b"hello world");

            assert!(
                request_ids.insert(received.request_id),
                "should not receive requests with duplicate request ids: {}",
                received.request_id,
            );

            let response = MultiplexMessage::Message(NetworkMessage::RpcResponse(RpcResponse {
                request_id: received.request_id,
                priority: 0,
                raw_response: Vec::from(&b"goodbye world"[..]),
            }));

            // Server should send the rpc request.
            server_sink.send(&response).await.unwrap();
        }
        assert!(server_stream.next().await.is_none());
    };
    rt.block_on(future::join3(peer.start(), server, client));
}

#[test]
fn peer_send_rpc_concurrent() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let upstream_handlers = Arc::new(HashMap::new());
    let (peer, peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut server_sink, mut server_stream) = build_network_sink_stream(&mut connection);
    let timeout = Duration::from_millis(10_000);

    let mut request_ids = HashSet::new();

    let client = async move {
        // Send a batch of RpcRequest to server and await response data.
        let mut send_recv_futures = Vec::new();
        for _ in 0..30 {
            let mut peer_handle = peer_handle.clone();
            let send_recv = async move {
                let response = peer_handle
                    .send_rpc_request(PROTOCOL, Bytes::from(&b"hello world"[..]), timeout)
                    .await
                    .unwrap();
                assert_eq!(response, Bytes::from(&b"goodbye world"[..]));
            };
            send_recv_futures.push(send_recv.boxed());
        }

        // Wait for all the responses.
        future::join_all(send_recv_futures).await;

        // Client then closes connection.
    };
    let server = async move {
        for _ in 0..30 {
            // Server should then receive the expected rpc request.
            let received = server_stream.next().await.unwrap().unwrap();

            let received = match received {
                MultiplexMessage::Message(NetworkMessage::RpcRequest(request)) => request,
                _ => panic!("Expected RpcRequest; unexpected: {:?}", received),
            };

            assert_eq!(received.protocol_id, PROTOCOL);
            assert_eq!(received.priority, 0);
            assert_eq!(received.raw_request, b"hello world");

            assert!(
                request_ids.insert(received.request_id),
                "should not receive requests with duplicate request ids: {}",
                received.request_id,
            );

            let response = MultiplexMessage::Message(NetworkMessage::RpcResponse(RpcResponse {
                request_id: received.request_id,
                priority: 0,
                raw_response: Vec::from(&b"goodbye world"[..]),
            }));

            // Server should send the rpc request.
            server_sink.send(&response).await.unwrap();
        }
        assert!(server_stream.next().await.is_none());
    };
    rt.block_on(future::join3(peer.start(), server, client));
}

#[test]
fn peer_send_rpc_cancel() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let upstream_handlers = Arc::new(HashMap::new());
    let (peer, peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut server_sink, mut server_stream) = build_network_sink_stream(&mut connection);
    let timeout = Duration::from_millis(10_000);

    let test = async move {
        // Client sends rpc request.
        let (response_tx, mut response_rx) = oneshot::channel();
        let outbound_rpc_request = OutboundRpcRequest::new(
            PROTOCOL,
            Bytes::from(&b"hello world"[..]),
            response_tx,
            timeout,
        );
        let request = PeerRequest::SendRpc(outbound_rpc_request);
        peer_handle.0.push(PROTOCOL, request).unwrap();

        // Server receives the rpc request from client.
        let received = server_stream.next().await.unwrap().unwrap();
        let received = match received {
            MultiplexMessage::Message(NetworkMessage::RpcRequest(request)) => request,
            _ => panic!("Expected RpcRequest; unexpected: {:?}", received),
        };

        assert_eq!(received.protocol_id, PROTOCOL);
        assert_eq!(received.priority, 0);
        assert_eq!(received.raw_request, b"hello world");

        // Request should still be live. Ok(_) means the sender is not dropped.
        // Ok(None) means there is no response yet.
        assert!(matches!(response_rx.try_recv(), Ok(None)));

        // Client cancels the request.
        drop(response_rx);

        // Server sending an expired response is fine.
        let response = MultiplexMessage::Message(NetworkMessage::RpcResponse(RpcResponse {
            request_id: received.request_id,
            priority: 0,
            raw_response: Vec::from(&b"goodbye world"[..]),
        }));
        server_sink.send(&response).await.unwrap();

        // Make sure the peer actor actually saw the message.
        tokio::task::yield_now().await;

        // Keep the peer_handle alive until the end to avoid prematurely closing
        // the connection.
        drop(peer_handle);
    };
    rt.block_on(future::join(peer.start(), test));
}

#[test]
fn peer_send_rpc_timeout() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let mock_time = MockTimeService::new();
    let upstream_handlers = Arc::new(HashMap::new());
    let (peer, peer_handle, mut connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        mock_time.clone().into(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let (mut server_sink, mut server_stream) = build_network_sink_stream(&mut connection);
    let timeout = Duration::from_millis(10_000);

    let test = async move {
        // Client sends rpc request.
        let (response_tx, mut response_rx) = oneshot::channel();
        let outbound_rpc_request = OutboundRpcRequest::new(
            PROTOCOL,
            Bytes::from(&b"hello world"[..]),
            response_tx,
            timeout,
        );
        let request = PeerRequest::SendRpc(outbound_rpc_request);
        peer_handle.0.push(PROTOCOL, request).unwrap();

        // Server receives the rpc request from client.
        let received = server_stream.next().await.unwrap().unwrap();
        let received = match received {
            MultiplexMessage::Message(NetworkMessage::RpcRequest(request)) => request,
            _ => panic!("Expected RpcRequest; unexpected: {:?}", received),
        };

        assert_eq!(received.protocol_id, PROTOCOL);
        assert_eq!(received.priority, 0);
        assert_eq!(received.raw_request, b"hello world");

        // Request should still be live. Ok(_) means the sender is not dropped.
        // Ok(None) means there is no response yet.
        assert!(matches!(response_rx.try_recv(), Ok(None)));

        // Advancing time should cause the client request timeout to elapse.
        mock_time.advance_async(timeout).await;

        // Client cancels the request.
        assert!(matches!(response_rx.await, Ok(Err(RpcError::TimedOut))));

        // Server sending an expired response is fine.
        let response = MultiplexMessage::Message(NetworkMessage::RpcResponse(RpcResponse {
            request_id: received.request_id,
            priority: 0,
            raw_response: Vec::from(&b"goodbye world"[..]),
        }));
        server_sink.send(&response).await.unwrap();

        // Make sure the peer actor actually saw the message.
        tokio::task::yield_now().await;

        // Keep the peer_handle alive until the end to avoid prematurely closing
        // the connection.
        drop(peer_handle);
    };
    rt.block_on(future::join(peer.start(), test));
}

// PeerManager can request a Peer to shutdown.
#[test]
fn peer_disconnect_request() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let upstream_handlers = Arc::new(HashMap::new());
    let (peer, peer_handle, _connection, mut connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let remote_peer_id = peer.remote_peer_id();

    let test = async move {
        drop(peer_handle);
        assert_disconnected_event(
            remote_peer_id,
            DisconnectReason::RequestedByPeerManager,
            &mut connection_notifs_rx,
        )
        .await;
    };

    rt.block_on(future::join(peer.start(), test));
}

// Peer will shutdown if the underlying connection is lost.
#[test]
fn peer_disconnect_connection_lost() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let upstream_handlers = Arc::new(HashMap::new());
    let (peer, _peer_handle, mut connection, mut connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );
    let remote_peer_id = peer.remote_peer_id();

    let test = async move {
        connection.close().await.unwrap();
        assert_disconnected_event(
            remote_peer_id,
            DisconnectReason::ConnectionClosed,
            &mut connection_notifs_rx,
        )
        .await;
    };
    rt.block_on(future::join(peer.start(), test));
}

#[test]
fn peer_terminates_when_request_tx_has_dropped() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let upstream_handlers = Arc::new(HashMap::new());
    let (peer, peer_handle, _connection, _connection_notifs_rx) = build_test_peer(
        rt.handle().clone(),
        TimeService::mock(),
        ConnectionOrigin::Inbound,
        upstream_handlers,
    );

    let drop = async move {
        // Drop peer handle.
        drop(peer_handle);
    };
    rt.block_on(future::join(peer.start(), drop));
}

#[test]
fn peers_send_multiplex() {
    ::aptos_logger::Logger::init_for_testing();
    let rt = Runtime::new().unwrap();
    let (upstream_handlers_a, mut prot_a_rx) = test_upstream_handlers();
    let (upstream_handlers_b, mut prot_b_rx) = test_upstream_handlers();
    let (
        (peer_a, mut peer_handle_a, mut connection_notifs_rx_a),
        (peer_b, mut peer_handle_b, mut connection_notifs_rx_b),
    ) = build_test_connected_peers(
        rt.handle().clone(),
        TimeService::mock(),
        upstream_handlers_a,
        upstream_handlers_b,
    );

    let remote_peer_id_a = peer_a.remote_peer_id();
    let remote_peer_id_b = peer_b.remote_peer_id();

    let test = async move {
        let msg_a = Message::new(
            PROTOCOL,
            Bytes::from(vec![0; MAX_MESSAGE_SIZE]), // stream message
        );
        let msg_b = Message::new(
            PROTOCOL,
            Bytes::from(vec![1; 1024]), // normal message
        );

        // Peer A -> msg_a -> Peer B
        peer_handle_a.send_direct_send(msg_a.clone());
        // Peer A <- msg_b <- Peer B
        peer_handle_b.send_direct_send(msg_b.clone());

        // Check that each peer received the other's message
        let notif_a = prot_a_rx.next().await;
        let notif_b = prot_b_rx.next().await;
        assert_eq!(
            notif_a.unwrap().message,
            NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: PROTOCOL,
                priority: 0,
                raw_msg: msg_b.data().clone().into(),
            })
        );
        assert_eq!(
            notif_b.unwrap().message,
            NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: PROTOCOL,
                priority: 0,
                raw_msg: msg_a.data().clone().into(),
            })
        );

        // Shut one peers and the other should shutdown due to ConnectionLost
        drop(peer_handle_a);

        // Check that we received both shutdown events
        assert_disconnected_event(
            remote_peer_id_a,
            DisconnectReason::RequestedByPeerManager,
            &mut connection_notifs_rx_a,
        )
        .await;
        assert_disconnected_event(
            remote_peer_id_b,
            DisconnectReason::ConnectionClosed,
            &mut connection_notifs_rx_b,
        )
        .await;
    };

    rt.block_on(future::join3(peer_a.start(), peer_b.start(), test));
}
