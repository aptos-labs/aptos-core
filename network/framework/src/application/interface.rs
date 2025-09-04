// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{error::Error, storage::PeersAndMetadata},
    peer::DisconnectReason,
    protocols::{
        network::{Message, NetworkEvents, NetworkSender},
        wire::handshake::v1::{ProtocolId, ProtocolIdSet},
    },
};
use velor_config::network_id::{NetworkId, PeerNetworkId};
use velor_logger::{prelude::*, sample, sample::SampleRate};
use velor_types::{network_address::NetworkAddress, PeerId};
use async_trait::async_trait;
use bytes::Bytes;
use itertools::Itertools;
use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};

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
    async fn disconnect_from_peer(
        &self,
        _peer: PeerNetworkId,
        _disconnect_reason: DisconnectReason,
    ) -> Result<(), Error>;

    /// Returns a list of available peers (i.e., those that are
    /// currently connected and support the relevant protocols
    /// for the client).
    fn get_available_peers(&self) -> Result<Vec<PeerNetworkId>, Error>;

    /// Returns a handle to the global `PeersAndMetadata` container
    fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata>;

    /// Sends the given message to the specified peer. Note: this
    /// method does not guarantee message delivery or handle responses.
    fn send_to_peer(&self, _message: Message, _peer: PeerNetworkId) -> Result<(), Error>;

    /// Sends the given message bytes to the specified peer. Note: this
    /// method does not guarantee message delivery or handle responses.
    fn send_to_peer_raw(&self, _message: Bytes, _peer: PeerNetworkId) -> Result<(), Error>;

    /// Sends the given message to each peer in the specified peer list.
    /// Note: this method does not guarantee message delivery or handle responses.
    fn send_to_peers(&self, _message: Message, _peers: Vec<PeerNetworkId>) -> Result<(), Error>;

    /// Sends the given message to the specified peer with the corresponding
    /// timeout. Awaits a response from the peer, or hits the timeout
    /// (whichever occurs first).
    async fn send_to_peer_rpc(
        &self,
        _message: Message,
        _rpc_timeout: Duration,
        _peer: PeerNetworkId,
    ) -> Result<Message, Error>;

    async fn send_to_peer_rpc_raw(
        &self,
        _message: Bytes,
        _rpc_timeout: Duration,
        _peer: PeerNetworkId,
    ) -> Result<Message, Error>;

    fn to_bytes_by_protocol(
        &self,
        _peers: Vec<PeerNetworkId>,
        _message: Message,
    ) -> anyhow::Result<HashMap<PeerNetworkId, Bytes>>;

    fn sort_peers_by_latency(&self, _network: NetworkId, _peers: &mut [PeerId]);
}

/// A network component that can be used by client applications (e.g., consensus,
/// state sync and mempool, etc.) to interact with the network and other peers.
#[derive(Clone, Debug)]
pub struct NetworkClient<Message> {
    direct_send_protocols_and_preferences: Vec<ProtocolId>, // Protocols are sorted by preference (highest to lowest)
    rpc_protocols_and_preferences: Vec<ProtocolId>, // Protocols are sorted by preference (highest to lowest)
    network_senders: HashMap<NetworkId, NetworkSender<Message>>,
    peers_and_metadata: Arc<PeersAndMetadata>,
}

impl<Message: NetworkMessageTrait + Clone> NetworkClient<Message> {
    pub fn new(
        direct_send_protocols_and_preferences: Vec<ProtocolId>,
        rpc_protocols_and_preferences: Vec<ProtocolId>,
        network_senders: HashMap<NetworkId, NetworkSender<Message>>,
        peers_and_metadata: Arc<PeersAndMetadata>,
    ) -> Self {
        Self {
            direct_send_protocols_and_preferences,
            rpc_protocols_and_preferences,
            network_senders,
            peers_and_metadata,
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

    fn group_peers_by_protocol(
        &self,
        peers: Vec<PeerNetworkId>,
    ) -> HashMap<ProtocolId, Vec<PeerNetworkId>> {
        // Sort peers by protocol
        let mut peers_per_protocol = HashMap::new();
        let mut peers_without_a_protocol = vec![];
        for peer in peers {
            match self
                .get_preferred_protocol_for_peer(&peer, &self.direct_send_protocols_and_preferences)
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
                    "[sampled] Unavailable peers (without a common network protocol): {:?}",
                    peers_without_a_protocol
                )
            );
        }

        peers_per_protocol
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

    async fn disconnect_from_peer(
        &self,
        peer: PeerNetworkId,
        disconnect_reason: DisconnectReason,
    ) -> Result<(), Error> {
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        Ok(network_sender
            .disconnect_peer(peer.peer_id(), disconnect_reason)
            .await?)
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

    fn send_to_peer_raw(&self, message: Bytes, peer: PeerNetworkId) -> Result<(), Error> {
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        let direct_send_protocol_id = self
            .get_preferred_protocol_for_peer(&peer, &self.direct_send_protocols_and_preferences)?;
        Ok(network_sender.send_to_raw(peer.peer_id(), direct_send_protocol_id, message)?)
    }

    fn send_to_peers(&self, message: Message, peers: Vec<PeerNetworkId>) -> Result<(), Error> {
        let peers_per_protocol = self.group_peers_by_protocol(peers);

        // Send to all peers in each protocol group and network
        for (protocol_id, peers) in peers_per_protocol {
            for (network_id, peers) in &peers
                .iter()
                .chunk_by(|peer_network_id| peer_network_id.network_id())
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
            .send_rpc(peer.peer_id(), rpc_protocol_id, message, rpc_timeout)
            .await?)
    }

    async fn send_to_peer_rpc_raw(
        &self,
        message: Bytes,
        rpc_timeout: Duration,
        peer: PeerNetworkId,
    ) -> Result<Message, Error> {
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        let rpc_protocol_id =
            self.get_preferred_protocol_for_peer(&peer, &self.rpc_protocols_and_preferences)?;
        Ok(network_sender
            .send_rpc_raw(peer.peer_id(), rpc_protocol_id, message, rpc_timeout)
            .await?)
    }

    fn to_bytes_by_protocol(
        &self,
        peers: Vec<PeerNetworkId>,
        message: Message,
    ) -> anyhow::Result<HashMap<PeerNetworkId, Bytes>> {
        let peers_per_protocol = self.group_peers_by_protocol(peers);
        // Convert to bytes per protocol
        let mut bytes_per_peer = HashMap::new();
        for (protocol_id, peers) in peers_per_protocol {
            let bytes: Bytes = protocol_id.to_bytes(&message)?.into();
            for peer in peers {
                bytes_per_peer.insert(peer, bytes.clone());
            }
        }

        Ok(bytes_per_peer)
    }

    fn sort_peers_by_latency(&self, network_id: NetworkId, peers: &mut [PeerId]) {
        self.peers_and_metadata
            .sort_peers_by_latency(network_id, peers)
    }
}

/// A network component that can be used by server applications (e.g., consensus,
/// state sync and mempool, etc.) to respond to network events and network clients.
pub struct NetworkServiceEvents<Message> {
    network_and_events: HashMap<NetworkId, NetworkEvents<Message>>,
}

impl<Message> NetworkServiceEvents<Message> {
    pub fn new(network_and_events: HashMap<NetworkId, NetworkEvents<Message>>) -> Self {
        Self { network_and_events }
    }

    /// Consumes and returns the network and events map
    pub fn into_network_and_events(self) -> HashMap<NetworkId, NetworkEvents<Message>> {
        self.network_and_events
    }
}
