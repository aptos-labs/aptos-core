// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::application::error::Error;
use crate::protocols::network::Message;
use crate::protocols::network::NetworkSender;
use crate::protocols::wire::handshake::v1::ProtocolId;
use crate::{
    application::storage::PeerMetadataStorage, error::NetworkError, protocols::network::RpcError,
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_types::network_address::NetworkAddress;
use aptos_types::PeerId;
use async_trait::async_trait;
use itertools::Itertools;
use std::sync::Arc;
use std::{collections::HashMap, fmt::Debug, time::Duration};

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

    /// Returns a handle to the global `PeerMetadataStorage`
    fn get_peer_metadata_storage(&self) -> Arc<PeerMetadataStorage>;

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
    direct_send_protocol_id: Option<ProtocolId>,
    rpc_protocol_id: Option<ProtocolId>,
    network_senders: HashMap<NetworkId, NetworkSender<Message>>,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
}

impl<Message: NetworkMessageTrait> NetworkClient<Message> {
    pub fn new(
        direct_send_protocol_id: Option<ProtocolId>,
        rpc_protocol_id: Option<ProtocolId>,
        network_senders: HashMap<NetworkId, NetworkSender<Message>>,
        peer_metadata_storage: Arc<PeerMetadataStorage>,
    ) -> Self {
        Self {
            direct_send_protocol_id,
            rpc_protocol_id,
            network_senders,
            peer_metadata_storage,
        }
    }

    fn get_direct_send_protocol_id(&self) -> Result<ProtocolId, Error> {
        self.direct_send_protocol_id
            .ok_or_else(|| Error::UnexpectedError("Direct send protocol ID not found!".into()))
    }

    fn get_rpc_protocol_id(&self) -> Result<ProtocolId, Error> {
        self.rpc_protocol_id
            .ok_or_else(|| Error::UnexpectedError("RPC protocol ID not found!".into()))
    }

    fn get_sender_for_network_id(
        &self,
        network_id: &NetworkId,
    ) -> Result<&NetworkSender<Message>, Error> {
        self.network_senders.get(network_id).ok_or_else(|| {
            Error::UnexpectedError(format!("Unknown network ID specified: {:?}", network_id))
        })
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

    fn get_peer_metadata_storage(&self) -> Arc<PeerMetadataStorage> {
        self.peer_metadata_storage.clone()
    }

    fn send_to_peer(&self, message: Message, peer: PeerNetworkId) -> Result<(), Error> {
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        let protocol_id = self.get_direct_send_protocol_id()?;
        Ok(network_sender.send_to(peer.peer_id(), protocol_id, message)?)
    }

    fn send_to_peers(&self, message: Message, peers: &[PeerNetworkId]) -> Result<(), Error> {
        let protocol_id = self.get_direct_send_protocol_id()?;
        for (network_id, peers) in &peers
            .iter()
            .group_by(|peer_network_id| peer_network_id.network_id())
        {
            let network_sender = self.get_sender_for_network_id(&network_id)?;
            let peer_ids = peers.map(|peer_network_id| peer_network_id.peer_id());
            network_sender.send_to_many(peer_ids, protocol_id, message.clone())?;
        }

        Ok(())
    }

    async fn send_to_peer_rpc(
        &self,
        message: Message,
        rpc_timeout: Duration,
        peer: PeerNetworkId,
    ) -> Result<Message, Error> {
        let protocol_id = self.get_rpc_protocol_id()?;
        let network_sender = self.get_sender_for_network_id(&peer.network_id())?;
        Ok(network_sender
            .send_rpc(peer.peer_id(), protocol_id, message, rpc_timeout)
            .await?)
    }
}

/// A simplified version of `NetworkSender` that doesn't use `ProtocolId` in the input
/// It was already being implemented for every application, but is now standardized
#[async_trait]
pub trait ApplicationNetworkSender<TMessage: Send>: Clone {
    fn send_to(&self, _recipient: PeerId, _message: TMessage) -> Result<(), NetworkError> {
        unimplemented!()
    }

    fn send_to_many(
        &self,
        _recipients: impl Iterator<Item = PeerId>,
        _message: TMessage,
    ) -> Result<(), NetworkError> {
        unimplemented!()
    }

    async fn send_rpc(
        &self,
        recipient: PeerId,
        req_msg: TMessage,
        timeout: Duration,
    ) -> Result<TMessage, RpcError>;
}
