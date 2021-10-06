// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{
        storage::{LockingHashMap, PeerMetadataStorage},
        types::{PeerInfo, PeerState},
    },
    error::NetworkError,
    protocols::network::{ApplicationNetworkSender, Message, RpcError},
};
use async_trait::async_trait;
use diem_config::network_id::{NetworkId, PeerNetworkId};
use diem_types::PeerId;
use itertools::Itertools;
use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData, time::Duration};

/// A generic `NetworkInterface` for applications to connect to networking
///
/// Each application would implement their own `NetworkInterface`.  This would hold `AppData` specific
/// to the application as well as a specific `Sender` for cloning across threads and sending requests.
#[async_trait]
pub trait NetworkInterface<
    TMessage: Message + Send,
    PeerNetworkIdSender: ApplicationPeerNetworkIdSender<TMessage>,
>
{
    /// The application specific key for `AppData`
    type AppDataKey: Clone + Debug + Eq + Hash;
    /// The application specific data to be stored
    type AppData: Clone + Debug;

    /// Provides the `PeerMetadataStorage` for other functions.  Not expected to be used externally.
    fn peer_metadata_storage(&self) -> &PeerMetadataStorage;

    /// Give a copy of the sender for the network
    fn sender(&self) -> PeerNetworkIdSender;

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

impl<TMessage: Message + Send, Sender: ApplicationNetworkSender<TMessage> + Send>
    MultiNetworkSender<TMessage, Sender>
{
    pub fn new(senders: HashMap<NetworkId, Sender>) -> Self {
        MultiNetworkSender {
            senders,
            _phantom: Default::default(),
        }
    }
}

impl<TMessage: Clone + Message + Send, Sender: ApplicationNetworkSender<TMessage> + Send>
    MultiNetworkSender<TMessage, Sender>
{
    fn sender(&mut self, network_id: &NetworkId) -> &mut Sender {
        self.senders.get_mut(network_id).expect("Unknown NetworkId")
    }
}

#[async_trait]
impl<TMessage: Clone + Message + Send, Sender: ApplicationNetworkSender<TMessage> + Send>
    ApplicationPeerNetworkIdSender<TMessage> for MultiNetworkSender<TMessage, Sender>
{
    fn send_to(&mut self, recipient: PeerNetworkId, message: TMessage) -> Result<(), NetworkError> {
        self.sender(&recipient.network_id())
            .send_to(recipient.peer_id(), message)
    }

    fn send_to_many(
        &mut self,
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

    async fn send_rpc(
        &mut self,
        recipient: PeerNetworkId,
        req_msg: TMessage,
        timeout: Duration,
    ) -> Result<TMessage, RpcError> {
        self.sender(&recipient.network_id())
            .send_rpc(recipient.peer_id(), req_msg, timeout)
            .await
    }
}

#[async_trait]
pub trait ApplicationPeerNetworkIdSender<TMessage: Message + Send>: Clone {
    fn send_to(
        &mut self,
        _recipient: PeerNetworkId,
        _message: TMessage,
    ) -> Result<(), NetworkError> {
        unimplemented!()
    }

    fn send_to_many(
        &mut self,
        _recipients: impl Iterator<Item = PeerNetworkId>,
        _message: TMessage,
    ) -> Result<(), NetworkError> {
        unimplemented!()
    }

    async fn send_rpc(
        &mut self,
        recipient: PeerNetworkId,
        req_msg: TMessage,
        timeout: Duration,
    ) -> Result<TMessage, RpcError>;
}
