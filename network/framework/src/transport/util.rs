// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use crate::application::interface::OutboundRpcMatcher;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use crate::protocols::network::{Closer, PeerStub};
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use crate::protocols::wire::messaging::v1::NetworkMessage;
use crate::{
    application::{storage::PeersAndMetadata, ApplicationCollector},
    counters, peer,
    protocols::network::OutboundPeerConnections,
    transport::AptosNetTransport,
};
use aptos_config::{
    config::NetworkConfig,
    network_id::{NetworkContext, PeerNetworkId},
};
use aptos_logger::{info, warn};
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use aptos_netcore::transport::memory::MemoryTransport;
use aptos_netcore::transport::{tcp::TcpTransport, ConnectionOrigin, Transport};
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use aptos_time_service::TimeService;
use aptos_time_service::TimeServiceTrait;
use aptos_types::network_address::NetworkAddress;
use futures::AsyncWriteExt;
use std::{
    io,
    io::{Error, ErrorKind},
    sync::Arc,
};
use tokio::runtime::Handle;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use tokio::sync::oneshot;

#[derive(Clone)]
pub enum AptosNetTransportActual {
    Tcp(AptosNetTransport<TcpTransport>),
    #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
    Memory(AptosNetTransport<MemoryTransport>),
    #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
    Mock(MockTransport),
}

impl AptosNetTransportActual {
    pub async fn dial(
        &mut self,
        remote_peer_network_id: PeerNetworkId,
        network_address: NetworkAddress,
        config: &NetworkConfig,
        apps: Arc<ApplicationCollector>,
        handle: Handle,
        peers_and_metadata: Arc<PeersAndMetadata>,
        peer_senders: Arc<OutboundPeerConnections>,
        network_context: NetworkContext,
    ) -> io::Result<()> {
        match self {
            AptosNetTransportActual::Tcp(tt) => {
                connect_outbound(
                    tt,
                    remote_peer_network_id,
                    network_address,
                    config,
                    apps,
                    handle.clone(),
                    peers_and_metadata,
                    peer_senders,
                    network_context,
                )
                .await
            },
            #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
            AptosNetTransportActual::Memory(tt) => {
                connect_outbound(
                    tt,
                    remote_peer_network_id,
                    network_address,
                    config,
                    apps,
                    handle.clone(),
                    peers_and_metadata,
                    peer_senders,
                    network_context,
                )
                .await
            },
            #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
            AptosNetTransportActual::Mock(mt) => {
                let (result_sender, mock_dial_result) = oneshot::channel::<io::Result<()>>();
                let msg = MockTransportEvent::Dial(MockTransportDial {
                    remote_peer_network_id,
                    network_address,
                    result_sender,
                });
                mt.call
                    .send(msg)
                    .await
                    .map_err(|_| io::Error::from(io::ErrorKind::NotConnected))?;
                let result = mock_dial_result.await.unwrap();
                if result.is_ok() {
                    // peers_and_metadata.insert_connection_metadata()
                    let (sender, _to_send) = tokio::sync::mpsc::channel::<(NetworkMessage, u64)>(1);
                    let (sender_high_prio, _to_send_high_prio) =
                        tokio::sync::mpsc::channel::<(NetworkMessage, u64)>(1);
                    let stub = PeerStub::new(
                        sender,
                        sender_high_prio,
                        OutboundRpcMatcher::new(mt.time_service.clone()),
                        Closer::new(),
                    );
                    peer_senders.insert(remote_peer_network_id, stub);
                }
                result
            },
        }
    }
}

#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
#[derive(Clone)]
pub struct MockTransport {
    // TODO: this will become a channel of enum of struct if we have to support more than dial()
    pub call: tokio::sync::mpsc::Sender<MockTransportEvent>,
    pub time_service: TimeService,
}

#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
pub enum MockTransportEvent {
    Dial(MockTransportDial),
}

#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
#[derive(Debug)]
pub struct MockTransportDial {
    pub remote_peer_network_id: PeerNetworkId,
    pub network_address: NetworkAddress,
    pub result_sender: oneshot::Sender<io::Result<()>>,
}

#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
pub fn new_mock_transport(time_service: TimeService) -> (
    AptosNetTransportActual,
    tokio::sync::mpsc::Receiver<MockTransportEvent>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let mt = MockTransport { call: tx, time_service };
    (AptosNetTransportActual::Mock(mt), rx)
}

async fn connect_outbound<TTransport, TSocket>(
    transport: &AptosNetTransport<TTransport>,
    remote_peer_network_id: PeerNetworkId,
    addr: NetworkAddress,
    config: &NetworkConfig,
    apps: Arc<ApplicationCollector>,
    handle: Handle,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_senders: Arc<OutboundPeerConnections>,
    network_context: NetworkContext,
) -> io::Result<()>
where
    TSocket: crate::transport::TSocket,
    TTransport: Transport<Output = TSocket, Error = io::Error> + Send + 'static,
{
    info!("dial connect_outbound {:?}", addr);
    let peer_id = remote_peer_network_id.peer_id();
    // TODO: rebuild connection init time counter
    let outbound = match transport.dial(peer_id, addr.clone()) {
        Ok(outbound) => outbound,
        Err(err) => {
            warn!(
                addr = addr,
                peer = remote_peer_network_id,
                "dial err 1: {:?}",
                err,
            );
            // TODO: counter?
            return Err(err);
        },
    };
    // tcp (or mem) connected, start protocol upgrade
    let upgrade_start = transport.time_service.now();
    let mut connection = match outbound.await {
        Ok(connection) => {
            // Connection<TSocket>
            let elapsed_time = (transport.time_service.now() - upgrade_start).as_secs_f64();
            counters::connection_upgrade_time(
                &network_context,
                ConnectionOrigin::Outbound,
                counters::SUCCEEDED_LABEL,
            )
            .observe(elapsed_time);
            connection
        },
        Err(err) => {
            let elapsed_time = (transport.time_service.now() - upgrade_start).as_secs_f64();
            counters::connection_upgrade_time(
                &network_context,
                ConnectionOrigin::Outbound,
                counters::FAILED_LABEL,
            )
            .observe(elapsed_time);
            warn!(
                addr = addr,
                peer = remote_peer_network_id,
                "dial err 2: {:?}",
                err,
            );
            // TODO: counter?
            return Err(err);
        },
    };
    let dialed_peer_id = connection.metadata.remote_peer_id;
    if dialed_peer_id != peer_id {
        warn!(
            "dial {:?} did not reach peer {:?} but peer {:?}",
            addr, peer_id, dialed_peer_id
        );
        _ = connection.socket.close().await; // discard secondary close error
        return Err(Error::new(ErrorKind::InvalidData, "peer_id mismatch"));
    }
    info!("dial starting peer {:?}", addr);
    peer::start_peer(
        config,
        connection.socket,
        connection.metadata,
        apps,
        handle,
        remote_peer_network_id,
        peers_and_metadata,
        peer_senders,
        network_context,
        transport.time_service.clone(),
    );
    Ok(())
}
