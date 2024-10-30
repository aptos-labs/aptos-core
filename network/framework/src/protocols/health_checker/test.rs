// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    peer_manager::{self, ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network::{NetworkSender, NewNetworkEvents, NewNetworkSender, ReceivedMessage},
        wire::{
            handshake::v1::{ProtocolId::HealthCheckerRpc, ProtocolIdSet},
            messaging::v1::{NetworkMessage, RpcRequest},
        },
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_time_service::TimeService;
use futures::future;
use maplit::hashmap;
use std::sync::Arc;

const PING_INTERVAL: Duration = Duration::from_secs(1);

struct TestHarness {
    peer_mgr_notifs_tx: aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
    connection_notifs_tx: tokio::sync::mpsc::Sender<ConnectionNotification>,
    peers_and_metadata: Arc<PeersAndMetadata>,
}

impl TestHarness {
    fn new_permissive() -> (Self, HealthChecker<NetworkClient<HealthCheckerMsg>>) {
        ::aptos_logger::Logger::init_for_testing();
        let mock_time = TimeService::mock();

        let (peer_mgr_reqs_tx, ..) = aptos_channel::new(QueueStyle::FIFO, 1, None);
        let (connection_reqs_tx, ..) = aptos_channel::new(QueueStyle::FIFO, 1, None);
        let (peer_mgr_notifs_tx, peer_mgr_notifs_rx) =
            aptos_channel::new(QueueStyle::FIFO, 1, None);
        let (connection_notifs_tx, connection_notifs_rx) = tokio::sync::mpsc::channel(10);

        let network_sender = NetworkSender::new(
            PeerManagerRequestSender::new(peer_mgr_reqs_tx),
            ConnectionRequestSender::new(connection_reqs_tx),
        );
        let hc_network_rx = HealthCheckerNetworkEvents::new(peer_mgr_notifs_rx, None, true);

        let network_context = NetworkContext::mock();
        let peers_and_metadata = PeersAndMetadata::new(&[network_context.network_id()]);
        let network_client = NetworkClient::new(
            vec![],
            vec![HealthCheckerRpc],
            hashmap! {network_context.network_id() => network_sender},
            peers_and_metadata.clone(),
        );

        let mut health_checker = HealthChecker::new(
            network_context,
            mock_time.clone(),
            HealthCheckNetworkInterface::new(network_client, hc_network_rx),
            PING_INTERVAL,
        );
        health_checker.set_connection_source(connection_notifs_rx);

        (
            Self {
                peer_mgr_notifs_tx,
                connection_notifs_tx,
                peers_and_metadata,
            },
            health_checker,
        )
    }

    fn new_strict() -> (Self, HealthChecker<NetworkClient<HealthCheckerMsg>>) {
        Self::new_permissive()
    }

    async fn send_inbound_ping(
        &mut self,
        peer_id: PeerId,
        ping: u32,
    ) -> oneshot::Receiver<Result<Bytes, RpcError>> {
        let protocol_id = ProtocolId::HealthCheckerRpc;
        let data = bcs::to_bytes(&HealthCheckerMsg::Ping(Ping(ping))).unwrap();
        let (res_tx, res_rx) = oneshot::channel();
        let key = (peer_id, ProtocolId::HealthCheckerRpc);
        let (delivered_tx, delivered_rx) = oneshot::channel();
        self.peer_mgr_notifs_tx
            .push_with_feedback(
                key,
                ReceivedMessage {
                    message: NetworkMessage::RpcRequest(RpcRequest {
                        protocol_id,
                        request_id: 0,
                        priority: 0,
                        raw_request: data,
                    }),
                    sender: PeerNetworkId::new(NetworkId::Validator, peer_id),
                    receive_timestamp_micros: 0,
                    rpc_replier: Some(Arc::new(res_tx)),
                },
                Some(delivered_tx),
            )
            .unwrap();
        delivered_rx.await.unwrap();
        res_rx
    }

    async fn send_new_peer_notification(&mut self, peer_id: PeerId) {
        let network_context = NetworkContext::mock();
        let notif = peer_manager::ConnectionNotification::NewPeer(
            ConnectionMetadata::mock(peer_id),
            network_context.network_id(),
        );
        self.connection_notifs_tx.send(notif).await.unwrap();

        // hacky `yield` to let thread on other side run, fast enough to make the test not suck, long enough it should almost always work
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Insert a new connection metadata into the peers and metadata
        let mut connection_metadata = ConnectionMetadata::mock(peer_id);
        connection_metadata.application_protocols =
            ProtocolIdSet::from_iter(vec![HealthCheckerRpc]);
        self.peers_and_metadata
            .insert_connection_metadata(
                PeerNetworkId::new(network_context.network_id(), peer_id),
                connection_metadata,
            )
            .unwrap();
    }
}

async fn expect_pong(res_rx: oneshot::Receiver<Result<Bytes, RpcError>>) {
    let res_data = res_rx.await.unwrap().unwrap();
    match bcs::from_bytes(&res_data).unwrap() {
        HealthCheckerMsg::Pong(_) => {},
        msg => panic!("Unexpected HealthCheckerMsg: {:?}", msg),
    };
}

#[tokio::test]
async fn inbound() {
    let (mut harness, health_checker) = TestHarness::new_strict();

    let test = async move {
        // Notify HealthChecker of new connected node.
        let peer_id = PeerId::new([0x42; PeerId::LENGTH]);
        harness.send_new_peer_notification(peer_id).await;

        // Receive ping from peer.
        let res_rx = harness.send_inbound_ping(peer_id, 0).await;

        // HealthChecker should respond with a pong.
        expect_pong(res_rx).await;
    };
    future::join(health_checker.start(), test).await;
}
