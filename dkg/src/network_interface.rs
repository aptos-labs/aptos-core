// Copyright © Aptos Foundation

use crate::DKGMessage;
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_network::{
    application::{error::Error, interface::NetworkClientInterface},
    ProtocolId,
};
use aptos_types::PeerId;
use std::time::Duration;

pub const RPC: &[ProtocolId] = &[
    ProtocolId::DKGRpcCompressed,
    ProtocolId::DKGRpcBcs,
    ProtocolId::DKGRpcJson,
];

pub const DIRECT_SEND: &[ProtocolId] = &[
    ProtocolId::DKGDirectSendCompressed,
    ProtocolId::DKGDirectSendBcs,
    ProtocolId::DKGDirectSendJson,
];

#[derive(Clone)]
pub struct DKGNetworkClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<DKGMessage>> DKGNetworkClient<NetworkClient> {
    /// Returns a new consensus network client
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    /// Send a single message to the destination peer
    pub fn send_to(&self, peer: PeerId, message: DKGMessage) -> Result<(), Error> {
        let peer_network_id = self.get_peer_network_id_for_peer(peer);
        self.network_client.send_to_peer(message, peer_network_id)
    }

    /// Send a single message to the destination peers
    pub fn send_to_many(
        &self,
        peers: impl Iterator<Item = PeerId>,
        message: DKGMessage,
    ) -> Result<(), Error> {
        let peer_network_ids: Vec<PeerNetworkId> = peers
            .map(|peer| self.get_peer_network_id_for_peer(peer))
            .collect();
        self.network_client
            .send_to_peers(message, &peer_network_ids)
    }

    /// Send a RPC to the destination peer
    pub async fn send_rpc(
        &self,
        peer: PeerId,
        message: DKGMessage,
        rpc_timeout: Duration,
    ) -> Result<DKGMessage, Error> {
        let peer_network_id = self.get_peer_network_id_for_peer(peer);
        self.network_client
            .send_to_peer_rpc(message, rpc_timeout, peer_network_id)
            .await
    }

    // TODO: we shouldn't need to expose this. Migrate the code to handle peer and network ids.
    fn get_peer_network_id_for_peer(&self, peer: PeerId) -> PeerNetworkId {
        PeerNetworkId::new(NetworkId::Validator, peer)
    }
}
