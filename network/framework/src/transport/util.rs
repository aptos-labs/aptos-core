// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::io;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use tokio::runtime::Handle;
use aptos_config::config::NetworkConfig;
use aptos_config::network_id::{NetworkContext, PeerNetworkId};
use aptos_logger::{info, warn};
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use aptos_netcore::transport::memory::MemoryTransport;
use aptos_netcore::transport::tcp::TcpTransport;
use aptos_netcore::transport::Transport;
use aptos_types::network_address::NetworkAddress;
use crate::application::ApplicationCollector;
use crate::application::storage::PeersAndMetadata;
use crate::peer;
use crate::protocols::network::OutboundPeerConnections;
use crate::transport::AptosNetTransport;
use futures::AsyncWriteExt;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use tokio::sync::oneshot;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use crate::application::interface::OutboundRpcMatcher;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use crate::protocols::wire::messaging::v1::NetworkMessage;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use crate::protocols::network::{Closer,PeerStub};

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
                connect_outbound(tt, remote_peer_network_id, network_address, config, apps, handle.clone(), peers_and_metadata, peer_senders, network_context).await
            }
            #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
            AptosNetTransportActual::Memory(tt) => {
                connect_outbound(tt, remote_peer_network_id, network_address, config, apps, handle.clone(), peers_and_metadata, peer_senders, network_context).await
            }
            #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
            AptosNetTransportActual::Mock(mt) => {
                let (result_sender, mock_dial_result) = oneshot::channel::<io::Result<()>>();
                let msg = MockTransportEvent::Dial(MockTransportDial{
                    remote_peer_network_id,
                    network_address,
                    result_sender,
                });
                mt.call.send(msg).await.map_err(|_| io::Error::from(io::ErrorKind::NotConnected))?;
                let result = mock_dial_result.await.unwrap();
                if result.is_ok() {
                    // peers_and_metadata.insert_connection_metadata()
                    let (sender, _to_send) = tokio::sync::mpsc::channel::<(NetworkMessage,u64)>(1);
                    let (sender_high_prio, _to_send_high_prio) = tokio::sync::mpsc::channel::<(NetworkMessage,u64)>(1);
                    let stub = PeerStub::new(
                        sender,
                        sender_high_prio,
                        OutboundRpcMatcher::new(),
                        Closer::new(),
                    );
                    peer_senders.insert(remote_peer_network_id,stub);
                }
                result
            }
        }
    }
}

#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
#[derive(Clone)]
pub struct MockTransport {
    // TODO: this will becom a channel of enum of struct if we have to support more than dial()
    pub call: tokio::sync::mpsc::Sender<MockTransportEvent>,
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
pub fn new_mock_transport() -> (AptosNetTransportActual, tokio::sync::mpsc::Receiver<MockTransportEvent>) {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let mt = MockTransport{
        call:tx,
    };
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
        Ok(outbound) => {
            outbound
        }
        Err(err) => {
            warn!("dial err: {:?}", err);
            // TODO: counter
            return Err(err);
        }
    };
    let mut connection = match outbound.await {
        Ok(connection) => { // Connection<TSocket>
            connection
        }
        Err(err) => {
            warn!("dial err 2: {:?}", err);
            // TODO: counter
            return Err(err);
        }
    };
    let dialed_peer_id = connection.metadata.remote_peer_id;
    if dialed_peer_id != peer_id {
        warn!("dial {:?} did not reach peer {:?} but peer {:?}", addr, peer_id, dialed_peer_id);
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
    );
    Ok(())
}
