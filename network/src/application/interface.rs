// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::application::error::Error;
use crate::protocols::network::Message;
use crate::protocols::network::NetworkSender;
use crate::protocols::wire::handshake::v1::ProtocolId;
use crate::{
    application::{
        storage::{LockingHashMap, PeerMetadataStorage},
        types::{PeerInfo, PeerState},
    },
    error::NetworkError,
    protocols::network::RpcError,
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_types::network_address::NetworkAddress;
use aptos_types::PeerId;
use async_trait::async_trait;
use itertools::Itertools;
use std::sync::Arc;
use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData, time::Duration};

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
    ) -> Result<(), Error> {
        unimplemented!()
    }

    /// Requests that the network connection for the specified peer
    /// is disconnected.
    // TODO: support disconnect reasons.
    async fn disconnect_from_peer(&self, _peer: PeerNetworkId) -> Result<(), Error> {
        unimplemented!()
    }

    /// Returns a handle to the global `PeerMetadataStorage`
    fn get_peer_metadata_storage(&self) -> Arc<PeerMetadataStorage> {
        unimplemented!()
    }

    /// Sends the given message to the specified peer. Note: this
    /// method does not guarantee message delivery or handle responses.
    fn send_to_peer(&self, _message: Message, _peer: PeerNetworkId) -> Result<(), Error> {
        unimplemented!()
    }

    /// Sends the given message to each peer in the specified peer list.
    /// Note: this method does not guarantee message delivery or handle responses.
    fn send_to_peers(&self, _message: Message, _peers: &[PeerNetworkId]) -> Result<(), Error> {
        unimplemented!()
    }

    /// Sends the given message to the specified peer with the corresponding
    /// timeout. Awaits a response from the peer, or hits the timeout
    /// (whichever occurs first).
    async fn send_to_peer_rpc(
        &self,
        _message: Message,
        _rpc_timeout: Duration,
        _peer: PeerNetworkId,
    ) -> Result<Message, Error> {
        unimplemented!()
    }
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

/// A generic `NetworkInterface` for applications to connect to networking
///
/// Each application would implement their own `NetworkInterface`.  This would hold `AppData` specific
/// to the application as well as a specific `Sender` for cloning across threads and sending requests.
#[async_trait]
pub trait NetworkInterface<TMessage: Message + Send, NetworkSender> {
    /// The application specific key for `AppData`
    type AppDataKey: Clone + Debug + Eq + Hash;
    /// The application specific data to be stored
    type AppData: Clone + Debug;

    /// Provides the `PeerMetadataStorage` for other functions.  Not expected to be used externally.
    fn peer_metadata_storage(&self) -> &PeerMetadataStorage;

    /// Give a copy of the sender for the network
    fn sender(&self) -> NetworkSender;

    /// Retrieve only connected peers
    fn connected_peers(&self, network_id: NetworkId) -> HashMap<PeerNetworkId, PeerInfo> {
        self.filtered_peers(network_id, |(_, peer_info)| {
            peer_info.status == PeerState::Connected
        })
    }

    /// Filter peers with according `filter`
    fn filtered_peers<F: FnMut(&(&PeerId, &PeerInfo)) -> bool>(
        &self,
        network_id: NetworkId,
        filter: F,
    ) -> HashMap<PeerNetworkId, PeerInfo> {
        self.peer_metadata_storage()
            .read_filtered(network_id, filter)
    }

    /// Retrieve PeerInfo for the node
    fn peers(&self, network_id: NetworkId) -> HashMap<PeerNetworkId, PeerInfo> {
        self.peer_metadata_storage().read_all(network_id)
    }

    /// Application specific data interface
    fn app_data(&self) -> &LockingHashMap<Self::AppDataKey, Self::AppData>;
}

#[derive(Clone, Debug)]
pub struct MultiNetworkSender<
    TMessage: Message + Send,
    Sender: ApplicationNetworkSender<TMessage> + Send,
> {
    senders: HashMap<NetworkId, Sender>,
    _phantom: PhantomData<TMessage>,
}

impl<TMessage: Clone + Message + Send, Sender: ApplicationNetworkSender<TMessage> + Send>
    MultiNetworkSender<TMessage, Sender>
{
    pub fn new(senders: HashMap<NetworkId, Sender>) -> Self {
        MultiNetworkSender {
            senders,
            _phantom: Default::default(),
        }
    }

    fn sender(&self, network_id: &NetworkId) -> &Sender {
        self.senders.get(network_id).expect("Unknown NetworkId")
    }

    pub fn send_to(&self, recipient: PeerNetworkId, message: TMessage) -> Result<(), NetworkError> {
        self.sender(&recipient.network_id())
            .send_to(recipient.peer_id(), message)
    }

    pub fn send_to_many(
        &self,
        recipients: impl Iterator<Item = PeerNetworkId>,
        message: TMessage,
    ) -> Result<(), NetworkError> {
        for (network_id, recipients) in
            &recipients.group_by(|peer_network_id| peer_network_id.network_id())
        {
            let sender = self.sender(&network_id);
            let peer_ids = recipients.map(|peer_network_id| peer_network_id.peer_id());
            sender.send_to_many(peer_ids, message.clone())?;
        }
        Ok(())
    }

    pub async fn send_rpc(
        &self,
        recipient: PeerNetworkId,
        req_msg: TMessage,
        timeout: Duration,
    ) -> Result<TMessage, RpcError> {
        self.sender(&recipient.network_id())
            .send_rpc(recipient.peer_id(), req_msg, timeout)
            .await
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
