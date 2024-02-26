// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{error::Error, storage::PeersAndMetadata},
    counters,
    protocols::{
        network::{Closer, Message, NetworkSender}, // NetworkEvents
        wire::handshake::v1::{ProtocolId, ProtocolIdSet},
    },
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_logger::{prelude::*, sample, sample::SampleRate};
use aptos_types::network_address::NetworkAddress;
use async_trait::async_trait;
use itertools::Itertools;
use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};
use std::collections::BTreeMap;
use std::sync::RwLock;
use anyhow::anyhow;
use bytes::Bytes;
use futures::channel::oneshot;
use aptos_config::config::RoleType;
use crate::protocols::network::RpcError;
use crate::protocols::wire::messaging::v1::RequestId;

/// A simple definition to handle all the trait bounds for messages.
// TODO: we should remove the duplication across the different files
pub trait NetworkMessageTrait: Clone + Message + Send + Sync + 'static {}
impl<T: Clone + Message + Send + Sync + 'static> NetworkMessageTrait for T {}

/// A simple interface offered by the networking stack to each client application (e.g., consensus,
/// state sync, mempool, etc.). This interface provides basic support for sending messages,
/// disconnecting from peers, notifying the network stack of new peers and managing application
/// specific metadata for each peer (e.g., peer scores and liveness).
// TODO: Add API calls for managing metadata, updating state, etc.
#[async_trait]
pub trait NetworkClientInterface<Message: NetworkMessageTrait>: Clone + Send + Sync {
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
    /// May return immediately if local outbound queue is full.
    fn send_to_peer(&self, _message: Message, _peer: PeerNetworkId) -> Result<(), Error>;

    /// Sends the given message to the specified peer. Note: this
    /// method does not guarantee message delivery or handle responses.
    /// Wait until message in enqueued or an error occurs.
    async fn send_to_peer_blocking(&self, _message: Message, _peer: PeerNetworkId) -> Result<(), Error>;

    /// Sends the given message to each peer in the specified peer list.
    /// Note: this method does not guarantee message delivery or handle responses.
    fn send_to_peers(&self, _message: Message, _peers: &[PeerNetworkId]) -> Result<(), Error>;

    /// Sends the given message to the specified peer with the corresponding
    /// timeout. Awaits a response from the peer, or hits the timeout
    /// (whichever occurs first).
    async fn send_to_peer_rpc(
        &self,
        _message: Message,
        _rpc_timeout: Duration,
        _peer: PeerNetworkId,
    ) -> Result<Message, Error>;
}

/// A network component that can be used by client applications (e.g., consensus,
/// state sync and mempool, etc.) to interact with the network and other peers.
#[derive(Clone, Debug)]
pub struct NetworkClient<Message> {
    direct_send_protocols_and_preferences: Vec<ProtocolId>, // Protocols are sorted by preference (highest to lowest)
    rpc_protocols_and_preferences: Vec<ProtocolId>, // Protocols are sorted by preference (highest to lowest)
    network_senders: HashMap<NetworkId, NetworkSender<Message>>,
    pub peers_and_metadata: Arc<PeersAndMetadata>,
}

impl<Message: NetworkMessageTrait + Clone> NetworkClient<Message> {
    pub fn new(
        direct_send_protocols_and_preferences: Vec<ProtocolId>,
        rpc_protocols_and_preferences: Vec<ProtocolId>,
        network_senders: HashMap<NetworkId, NetworkSender<Message>>,
        peers_and_metadata: Arc<PeersAndMetadata>,
        // open_outbound_rpc: OutboundRpcMatcher,
    ) -> Self {
        Self {
            direct_send_protocols_and_preferences,
            rpc_protocols_and_preferences,
            network_senders,
            peers_and_metadata,
            // open_outbound_rpc,
        }
    }

    /// Returns the network sender for the specified network ID
    fn get_sender_for_network_id(
        &self,
        network_id: &NetworkId,
    ) -> Result<&NetworkSender<Message>, Error> {
        self.network_senders.get(network_id).ok_or_else(|| {
            Error::UnexpectedError(format!(
                "Unknown network ID specified for sender: {:?}",
                network_id
            ))
        })
    }

    /// Identify the supported protocols from the specified peer's connection
    fn get_supported_protocols(&self, peer: &PeerNetworkId) -> Result<ProtocolIdSet, Error> {
        //let peers_and_metadata = self.get_peers_and_metadata();
        self.peers_and_metadata
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
        Err(Error::NetworkError(anyhow!(
            "None of the preferred protocols are supported by this peer! \
            Peer: {:?}, supported protocols: {:?}",
            peer, protocols_supported_by_peer
        ).into()))
    }
}

#[async_trait]
impl<Message: NetworkMessageTrait> NetworkClientInterface<Message> for NetworkClient<Message> {
    async fn add_peers_to_discovery(
        &self,
        _peers: &[(PeerNetworkId, NetworkAddress)],
    ) -> Result<(), Error> {
        unimplemented!("Adding peers to discovery is not yet supported!");
    }

    async fn disconnect_from_peer(&self, peer: PeerNetworkId) -> Result<(), Error> {
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        Ok(network_sender.disconnect_peer(peer.peer_id()).await?)
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

    fn send_to_peer(&self, message: Message, peer: PeerNetworkId) -> Result<(), Error> {
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        let direct_send_protocol_id = self
            .get_preferred_protocol_for_peer(&peer, &self.direct_send_protocols_and_preferences)?;
        Ok(network_sender.send_to(peer.peer_id(), direct_send_protocol_id, message)?)
    }

    async fn send_to_peer_blocking(&self, message: Message, peer: PeerNetworkId) -> Result<(), Error> {
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        let direct_send_protocol_id = self
            .get_preferred_protocol_for_peer(&peer, &self.direct_send_protocols_and_preferences)?;
        Ok(network_sender.send_async(peer.peer_id(), direct_send_protocol_id, message).await?)
    }

    fn send_to_peers(&self, message: Message, peers: &[PeerNetworkId]) -> Result<(), Error> {
        // Sort peers by protocol
        let mut peers_per_protocol = HashMap::new();
        let mut peers_without_a_protocol = vec![];
        for peer in peers {
            match self
                .get_preferred_protocol_for_peer(peer, &self.direct_send_protocols_and_preferences)
            {
                Ok(protocol) => peers_per_protocol
                    .entry(protocol)
                    .or_insert_with(Vec::new)
                    .push(peer),
                Err(_) => peers_without_a_protocol.push(peer),
            }
        }

        // We only periodically log any unavailable peers (to prevent log spamming)
        if !peers_without_a_protocol.is_empty() {
            sample!(
                SampleRate::Duration(Duration::from_secs(10)),
                warn!(
                    "Unavailable peers (without a common network protocol): {:?}",
                    peers_without_a_protocol
                )
            );
        }

        // Send to all peers in each protocol group and network
        for (protocol_id, peers) in peers_per_protocol {
            for (network_id, peers) in &peers
                .iter()
                .group_by(|peer_network_id| peer_network_id.network_id())
            {
                let network_sender = self.get_sender_for_network_id(&network_id)?;
                let peer_ids = peers.map(|peer_network_id| peer_network_id.peer_id());
                network_sender.send_to_many(peer_ids, protocol_id, message.clone())?;
            }
        }
        Ok(())
    }

    async fn send_to_peer_rpc(
        &self,
        message: Message,
        rpc_timeout: Duration,
        peer: PeerNetworkId,
    ) -> Result<Message, Error> {
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        let rpc_protocol_id =
            self.get_preferred_protocol_for_peer(&peer, &self.rpc_protocols_and_preferences)?;
        Ok(network_sender
            .send_rpc(peer.peer_id(), rpc_protocol_id, message, rpc_timeout, protocol_is_high_priority(rpc_protocol_id))
            .await?)
    }
}
//
// /// A network component that can be used by server applications (e.g., consensus,
// /// state sync and mempool, etc.) to respond to network events and network clients.
// pub struct NetworkServiceEvents<Message> {
//     network_and_events: HashMap<NetworkId, NetworkEvents<Message>>,
// }
//
// impl<Message> NetworkServiceEvents<Message> {
//     pub fn new(network_and_events: HashMap<NetworkId, NetworkEvents<Message>>) -> Self {
//         Self { network_and_events }
//     }
//
//     /// Consumes and returns the network and events map
//     pub fn into_network_and_events(self) -> HashMap<NetworkId, NetworkEvents<Message>> {
//         self.network_and_events
//     }
// }

pub type NetworkEvents<Message> = crate::protocols::network::NetworkEvents<Message>;

#[derive(Debug)]
pub struct OpenRpcRequestState {
    pub id: RequestId,
    // send on this to deliver a reply back to an open NetworkSender.send_rpc()
    pub sender: oneshot::Sender<Result<(Bytes, u64), RpcError>>,
    pub protocol_id: ProtocolId,
    pub started: tokio::time::Instant, // for metrics
    pub deadline: tokio::time::Instant,
    pub network_id: NetworkId, // for metrics
    pub role_type: RoleType, // for metrics
}

/// OutboundRpcMatcher contains an Arc-RwLock of oneshot reply channels
#[derive(Clone,Debug)]
pub struct OutboundRpcMatcher {
    open_outbound_rpc: Arc<RwLock<BTreeMap<RequestId, OpenRpcRequestState>>>,
}

impl OutboundRpcMatcher {
    pub fn new() -> Self {
        Self {
            open_outbound_rpc: Arc::new(RwLock::new(BTreeMap::new()))
        }
    }

    /// Get an OpenRpcRequestState so we can reply to it.
    /// May return None if request_id was already handled, timed out, or never existed.
    pub fn remove(&self, request_id: &RequestId) -> Option<OpenRpcRequestState> {
        self.open_outbound_rpc.write().unwrap().remove(request_id)
    }

    pub fn insert(
        &self,
        request_id: RequestId,
        sender: oneshot::Sender<Result<(Bytes, u64), RpcError>>,
        protocol_id: ProtocolId,
        started: tokio::time::Instant,
        deadline: tokio::time::Instant,
        network_id: NetworkId,
        role_type: RoleType,
    ) {
        let val = OpenRpcRequestState{
            id: request_id,
            sender,
            protocol_id,
            started,
            deadline,
            network_id,
            role_type,
        };
        self.open_outbound_rpc.write().unwrap().insert(request_id, val);
    }

    /// Periodic cleanup task, run ~ 10Hz
    /// Assume normal flow is for RPCs to _not_ timeout.
    pub async fn cleanup(self, period: Duration, mut closed: Closer) {
        loop {
            tokio::select!{
                () = tokio::time::sleep(period) => {}
                _ = closed.wait() => {return}
            }
            self.cleanup_internal();
        }
    }

    fn cleanup_internal(&self) {
        let mut they = self.open_outbound_rpc.write().unwrap();
        let mut to_delete = vec![];
        let now = tokio::time::Instant::now();
        {
            for (k, v) in they.iter() {
                if now > v.deadline {
                    to_delete.push(*k);
                }
            }
        }
        if !to_delete.is_empty() {
            for k in to_delete.into_iter() {
                let reqo = they.remove(&k);
                if let Some(rpc_state) = reqo {
                    // we were waiting for a reply and never got one
                    counters::rpc_messages(rpc_state.network_id, rpc_state.protocol_id.as_str(), rpc_state.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, "timeoutI").inc();
                }
            }
        }
    }
}

impl Default for OutboundRpcMatcher {
    fn default() -> Self {
        Self::new()
    }
}

pub fn protocol_is_high_priority(protocol_id: ProtocolId) -> bool {
    match protocol_id {
        ProtocolId::ConsensusRpcBcs => {true}
        ProtocolId::ConsensusDirectSendBcs => {true}
        // ProtocolId::MempoolDirectSend => {}
        ProtocolId::StateSyncDirectSend => {true}
        // ProtocolId::DiscoveryDirectSend => {}
        // ProtocolId::HealthCheckerRpc => {}
        ProtocolId::ConsensusDirectSendJson => {true}
        ProtocolId::ConsensusRpcJson => {true}
        ProtocolId::StorageServiceRpc => {true}
        // ProtocolId::MempoolRpc => {}
        // ProtocolId::PeerMonitoringServiceRpc => {}
        ProtocolId::ConsensusRpcCompressed => {true}
        ProtocolId::ConsensusDirectSendCompressed => {true}
        // ProtocolId::NetbenchDirectSend => {}
        // ProtocolId::NetbenchRpc => {}
        _ => {false}
    }
}
