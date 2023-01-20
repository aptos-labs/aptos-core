// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{error::Error, storage::PeersAndMetadata},
    protocols::{
        network::{Message, NetworkEvents, NetworkSender},
        wire::handshake::v1::{ProtocolId, ProtocolIdSet},
    },
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::{prelude::*, sample, sample::SampleRate};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::network_address::NetworkAddress;
use async_trait::async_trait;
use futures_util::StreamExt;
use itertools::Itertools;
use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};
use tokio::{runtime::Handle, task::JoinHandle};

/// A simple definition to handle all the trait bounds for messages.
// TODO: we should remove the duplication across the different files
pub trait NetworkMessageTrait: Clone + Message + Send + Sync + 'static {}
impl<T: Clone + Message + Send + Sync + 'static> NetworkMessageTrait for T {}

/// A simple enum to represent the different types of messages
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MessageType {
    DirectSendMessage,
    RpcMessage,
}

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
    fn send_to_peer(&self, _message: Message, _peer: PeerNetworkId) -> Result<(), Error>;

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
    peers_and_metadata: Arc<PeersAndMetadata>,

    // A simple cache of preferred protocols for each peer. This avoids
    // having to perform redundant computation each time we need to send
    // a message. The cache is invalidated periodically and rebuilt lazily.
    preferred_protocol_for_peer_cache:
        Arc<RwLock<HashMap<(PeerNetworkId, MessageType), ProtocolId>>>,
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
            preferred_protocol_for_peer_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Spawns the thread that periodically invalidates the
    /// preferred protocol cache so that it can be refreshed.
    /// This is required to support peers who change their
    /// preferred/supported protocols and reconnect to us.
    pub fn spawn_preferred_protocol_cache_invalidator(
        &self,
        cache_invalidation_frequency_secs: u64,
        time_service: TimeService,
        runtime: Handle,
    ) -> JoinHandle<()> {
        let preferred_protocol_for_peer_cache = self.preferred_protocol_for_peer_cache.clone();

        // Spawn the cache invalidator thread
        runtime.spawn(async move {
            // Create a ticker for the invalidation interval
            let duration = Duration::from_secs(cache_invalidation_frequency_secs);
            let ticker = time_service.interval(duration);
            futures::pin_mut!(ticker);

            info!("Starting the preferred protocol cache invalidator!");
            loop {
                // Wait for the next round before polling
                ticker.next().await;

                // Invalidate the cache
                trace!("Clearing the preferred protocol cache!");
                preferred_protocol_for_peer_cache.write().clear();
            }
        })
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
        message_type: MessageType,
    ) -> Result<ProtocolId, Error> {
        // Check if we've already cached the protocol for the peer
        let cache_key = (*peer, message_type);
        if let Some(preferred_protocol) = self
            .preferred_protocol_for_peer_cache
            .read()
            .get(&cache_key)
        {
            return Ok(*preferred_protocol);
        }

        // Otherwise, calculate the preferred protocol and update the cache before returning
        let all_protocols = match message_type {
            MessageType::DirectSendMessage => &self.direct_send_protocols_and_preferences,
            MessageType::RpcMessage => &self.rpc_protocols_and_preferences,
        };
        let preferred_protocol = self.calculate_preferred_protocol_for_peer(peer, all_protocols)?;
        let _ = self
            .preferred_protocol_for_peer_cache
            .write()
            .insert(cache_key, preferred_protocol);
        Ok(preferred_protocol)
    }

    /// Calculates the preferred protocol for the specified peer. The preferred
    /// protocols should be sorted from most to least preferable.
    fn calculate_preferred_protocol_for_peer(
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

    /// Returns a handle to the preferred protocol for peer cache.
    /// Only required for testing.
    #[cfg(test)]
    pub fn get_preferred_protocol_for_peer_cache(
        &self,
    ) -> &RwLock<HashMap<(PeerNetworkId, MessageType), ProtocolId>> {
        &self.preferred_protocol_for_peer_cache
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
        let direct_send_protocol_id =
            self.get_preferred_protocol_for_peer(&peer, MessageType::DirectSendMessage)?;
        Ok(network_sender.send_to(peer.peer_id(), direct_send_protocol_id, message)?)
    }

    fn send_to_peers(&self, message: Message, peers: &[PeerNetworkId]) -> Result<(), Error> {
        // Sort peers by protocol
        let mut peers_per_protocol = HashMap::new();
        let mut peers_without_a_protocol = vec![];
        for peer in peers {
            match self.get_preferred_protocol_for_peer(peer, MessageType::DirectSendMessage) {
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
            self.get_preferred_protocol_for_peer(&peer, MessageType::RpcMessage)?;
        Ok(network_sender
            .send_rpc(peer.peer_id(), rpc_protocol_id, message, rpc_timeout)
            .await?)
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
