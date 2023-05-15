// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{error::Error, storage::PeersAndMetadata},
    protocols::{
        network::{Message, NetworkEvents, NetworkSender},
        wire::handshake::v1::{ProtocolId, ProtocolIdSet},
    },
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_logger::{prelude::*, sample, sample::SampleRate};
use aptos_types::network_address::NetworkAddress;
use async_trait::async_trait;
use itertools::Itertools;
use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Bytes;
use futures_util::future::FusedFuture;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::protocols::network::NetworkEvents2;

/// A simple definition to handle all the trait bounds for messages.
// TODO: we should remove the duplication across the different files
//pub trait NetworkMessageTrait: Clone + Message + Send + Sync + 'static {}
//impl<T: Clone + Message + Send + Sync + 'static> NetworkMessageTrait for T {}

/// A simple interface offered by the networking stack to each client application (e.g., consensus,
/// state sync, mempool, etc.). This interface provides basic support for sending messages,
/// disconnecting from peers, notifying the network stack of new peers and managing application
/// specific metadata for each peer (e.g., peer scores and liveness).
// TODO: Add API calls for managing metadata, updating state, etc.
#[async_trait]
pub trait NetworkClientInterface: Clone + Send + Sync {
    /// Adds the given peer list to the set of discovered peers
    /// that can potentially be dialed for future connections.
    async fn add_peers_to_discovery(
        &self,
        _peers: &[(PeerNetworkId, NetworkAddress)],
    ) -> Result<(), Error>;

    /// Requests that the network connection for the specified peer
    /// is disconnected.
    // TODO: support disconnect reasons.
    async fn disconnect_from_peer(&self, _peer: PeerNetworkId) -> Result<(), Error>;

    /// Returns a list of available peers (i.e., those that are
    /// currently connected and support the relevant protocols
    /// for the client).
    fn get_available_peers(&self) -> Result<Vec<PeerNetworkId>, Error>;

    /// Returns a handle to the global `PeersAndMetadata` container
    fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata>;

    /// Sends the given message to the specified peer. Note: this
    /// method does not guarantee message delivery or handle responses.
    fn send_to_peer<T: Serialize + Sync>(&self, protocol_id: ProtocolId, _message: &T, _peer: PeerNetworkId) -> Result<(), Error>;

    /// Sends the given message to each peer in the specified peer list.
    /// Note: this method does not guarantee message delivery or handle responses.
    fn send_to_peers<T: Serialize + Sync>(&self, protocol_id: ProtocolId, _message: &T, _peers: &[PeerNetworkId]) -> Result<(), Error>;

    /// Sends the given message to the specified peer with the corresponding
    /// timeout. Awaits a response from the peer, or hits the timeout
    /// (whichever occurs first).
    async fn send_to_peer_rpc<T: Serialize + Sync, O: DeserializeOwned>(
        &self,
        protocol_id: ProtocolId,
        _message: &T,
        _rpc_timeout: Duration,
        _peer: PeerNetworkId,
    ) -> Result<O, Error>;
}

/// A network component that can be used by client applications (e.g., consensus,
/// state sync and mempool, etc.) to interact with the network and other peers.
#[derive(Clone, Debug)]
pub struct NetworkClient {
    direct_send_protocols_and_preferences: Vec<ProtocolId>, // Protocols are sorted by preference (highest to lowest)
    rpc_protocols_and_preferences: Vec<ProtocolId>, // Protocols are sorted by preference (highest to lowest)
    // network_senders: HashMap<NetworkId, NetworkSender>,
    peers_and_metadata: Arc<PeersAndMetadata>,
}

impl NetworkClient {
    pub fn new(
        direct_send_protocols_and_preferences: Vec<ProtocolId>,
        rpc_protocols_and_preferences: Vec<ProtocolId>,
        // network_senders: HashMap<NetworkId, NetworkSender>,
        peers_and_metadata: Arc<PeersAndMetadata>,
    ) -> Self {
        Self {
            direct_send_protocols_and_preferences,
            rpc_protocols_and_preferences,
            // network_senders,
            peers_and_metadata,
        }
    }

    /// Returns the network sender for the specified network ID
    // fn get_sender_for_network_id(
    //     &self,
    //     network_id: &NetworkId,
    // ) -> Result<&NetworkSender, Error> {
    //     self.network_senders.get(network_id).ok_or_else(|| {
    //         Error::UnexpectedError(format!(
    //             "Unknown network ID specified for sender: {:?}",
    //             network_id
    //         ))
    //     })
    // }

    /// Identify the supported protocols from the specified peer's connection
    fn get_supported_protocols(&self, peer: &PeerNetworkId) -> Result<ProtocolIdSet, Error> {
        let peers_and_metadata = self.get_peers_and_metadata();
        peers_and_metadata
            .get_metadata_for_peer(*peer)
            .map(|peer_metadata| peer_metadata.get_supported_protocols())
    }

    /// Selects the preferred protocol for the specified peer. The preferred protocols
    /// should be sorted from most to least preferable.
    fn get_preferred_protocol_for_peer(
        &self,
        peer: &PeerNetworkId,
        preferred_protocols: &[ProtocolId],
    ) -> Result<ProtocolId, Error> {
        let protocols_supported_by_peer = self.get_supported_protocols(peer)?;
        for protocol in preferred_protocols {
            if protocols_supported_by_peer.contains(*protocol) {
                return Ok(*protocol);
            }
        }
        Err(Error::NetworkError(format!(
            "None of the preferred protocols are supported by this peer! \
            Peer: {:?}, supported protocols: {:?}",
            peer, protocols_supported_by_peer
        )))
    }

    fn send_bytes_to_peer(&self, protocol_id: ProtocolId, message: Bytes, peer: &PeerNetworkId) -> Result<(), Error> {
        // TODO: put on per-peer-per-network outbound queue for a per-peer send thread
        // TODO: manage maximum number of pending messages and maximum total pending bytes on a peer outbound queue
        // TODO? (optionally, for some queues) when the queue is full push a new message and drop the _oldest_ message. The old message may be obsolete already, only send the newest data.
        Err(Error::UnexpectedError(format!("TODO: implement network client send_bytes_to_peer")))
    }

    fn send_rpc_to_peer(&self, protocol_id: ProtocolId, message: Bytes, rpc_timeout: Duration, peer: &PeerNetworkId) -> RpcResult {
        // TODO: outbound RPC could get complicated. timeout could happen to something in the queue and not even sent yet.
        // A timer task needs to keep a priority queue ordered by next timeout.
        // Lookup by rpc_id and lookup by timeout both need to be cleared when either is invoked.
        RpcResult{}
    }
}

pub struct RpcResult {
    // TODO: stuff?
}

impl Future for RpcResult {
    type Output = Bytes;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending // TODO: implement RpcResult FusedFuture
    }
}

impl FusedFuture for RpcResult {
    fn is_terminated(&self) -> bool {
        true // TODO: implement RpcResult FusedFuture
    }
}

#[async_trait]
impl NetworkClientInterface for NetworkClient {
    async fn add_peers_to_discovery(
        &self,
        _peers: &[(PeerNetworkId, NetworkAddress)],
    ) -> Result<(), Error> {
        unimplemented!("Adding peers to discovery is not yet supported!");
    }

    async fn disconnect_from_peer(&self, peer: PeerNetworkId) -> Result<(), Error> {
        // TODO: fix, reimplement
        // let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        // Ok(network_sender.disconnect_peer(peer.peer_id()).await?)
        Err(Error::UnexpectedError(format!("TODO: reimplement network client disconnect_from_peer")))
    }

    fn get_available_peers(&self) -> Result<Vec<PeerNetworkId>, Error> {
        let supported_protocol_ids: Vec<ProtocolId> = self
            .direct_send_protocols_and_preferences
            .iter()
            .chain(self.rpc_protocols_and_preferences.iter())
            .cloned()
            .collect();
        self.peers_and_metadata
            .get_connected_supported_peers(&supported_protocol_ids)
    }

    fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.peers_and_metadata.clone()
    }

    fn send_to_peer<T: Serialize + Sync>(&self, protocol_id: ProtocolId, message: &T, peer: PeerNetworkId) -> Result<(), Error> {
        let blob = protocol_id.to_bytes(message).map_err(|e| Error::UnexpectedError(format!("encode err: {}", e)))?;
        self.send_bytes_to_peer(protocol_id,blob.into(), &peer)
    }

    fn send_to_peers<T: Serialize + Sync>(&self, protocol_id: ProtocolId, message: &T, peers: &[PeerNetworkId]) -> Result<(), Error> {
        // Sort peers by protocol
        let blob = protocol_id.to_bytes(message).map_err(|e| Error::UnexpectedError(format!("encode err: {}", e)))?;
        //let mut peers_per_protocol = HashMap::new();
        //let mut peers_without_a_protocol = vec![];
        //let mut errors = vec![];
        for peer in peers {
            let network_id = peer.network_id();
            if let Ok(prots) = self.get_supported_protocols(peer) {
                if prots.contains(protocol_id) {
                    if let Err(err) = self.send_bytes_to_peer(protocol_id, blob.clone().into(), peer) {
                        // TODO: count send errors for metrics?
                        // TODO: log errors?
                    }
                    // if let Ok(network_sender) = self.get_sender_for_network_id(&network_id) {
                    //     if let Err(err) = network_sender.send_to(peer.peer_id(), protocol_id, blob.clone().into()) {
                    //         // TODO: count send errors for metrics?
                    //         // TODO: log errors?
                    //         //errors.push(err);
                    //     }
                    // } // TODO: wat? how could this happen?
                } // TODO: count unsendable messages?
            } // TODO: wat? peer disconnected while we weren't looking?
            // match self
            //     .get_preferred_protocol_for_peer(peer, &self.direct_send_protocols_and_preferences)
            // {
            //     Ok(protocol) => peers_per_protocol
            //         .entry(protocol)
            //         .or_insert_with(Vec::new)
            //         .push(peer),
            //     Err(_) => peers_without_a_protocol.push(peer),
            // }
        }
        //
        // // We only periodically log any unavailable peers (to prevent log spamming)
        // if !peers_without_a_protocol.is_empty() {
        //     sample!(
        //         SampleRate::Duration(Duration::from_secs(10)),
        //         warn!(
        //             "Unavailable peers (without a common network protocol): {:?}",
        //             peers_without_a_protocol
        //         )
        //     );
        // }
        //
        // // Send to all peers in each protocol group and network
        // for (protocol_id, peers) in peers_per_protocol {
        //     for (network_id, peers) in &peers
        //         .iter()
        //         .group_by(|peer_network_id| peer_network_id.network_id())
        //     {
        //         let network_sender = self.get_sender_for_network_id(&network_id)?;
        //         let peer_ids = peers.map(|peer_network_id| peer_network_id.peer_id());
        //         network_sender.send_to_many(peer_ids, protocol_id, message.clone())?;
        //     }
        // }
        Ok(())
    }

    async fn send_to_peer_rpc<T: Serialize + Sync, O: DeserializeOwned>(
        &self,
        protocol_id: ProtocolId,
        message: &T,
        rpc_timeout: Duration,
        peer: PeerNetworkId,
    ) -> Result<O, Error> {
        let blob = protocol_id.to_bytes(message).map_err(|e| Error::UnexpectedError(format!("encode err: {}", e)))?;
        //let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        // let rpc_protocol_id =
        //     self.get_preferred_protocol_for_peer(&peer, &self.rpc_protocols_and_preferences)?;
        // let result = network_sender
        //     .send_rpc(peer.peer_id(), protocol_id, blob.into(), rpc_timeout)
        //     .await?;
        let result = self.send_rpc_to_peer(protocol_id, blob.into(), rpc_timeout, &peer).await;
        protocol_id.from_bytes(result.as_ref()).map_err(|e| Error::NetworkError(format!("decode err: {}", e)))
    }
}

/// A network component that can be used by server applications (e.g., consensus,
/// state sync and mempool, etc.) to respond to network events and network clients.
pub struct NetworkServiceEvents {
    network_and_events: HashMap<NetworkId, NetworkEvents2>,
}

impl NetworkServiceEvents {
    pub fn new(network_and_events: HashMap<NetworkId, NetworkEvents2>) -> Self {
        Self { network_and_events }
    }
    //
    // /// Consumes and returns the network and events map
    // pub fn into_network_and_events(self) -> HashMap<NetworkId, NetworkEvents<_>> {
    //     self.network_and_events
    // }
}
