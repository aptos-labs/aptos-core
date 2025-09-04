// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::JWKConsensusMsg;
use velor_config::network_id::{NetworkId, PeerNetworkId};
use velor_network::{
    application::{error::Error, interface::NetworkClientInterface},
    ProtocolId,
};
use bytes::Bytes;
use move_core_types::account_address::AccountAddress as PeerId;
use std::{collections::HashMap, time::Duration};

/// Supported protocols in preferred order (from highest priority to lowest).
pub const DIRECT_SEND: &[ProtocolId] = &[
    ProtocolId::JWKConsensusDirectSendCompressed,
    ProtocolId::JWKConsensusDirectSendBcs,
    ProtocolId::JWKConsensusDirectSendJson,
];

/// Supported protocols in preferred order (from highest priority to lowest).
pub const RPC: &[ProtocolId] = &[
    ProtocolId::JWKConsensusRpcCompressed,
    ProtocolId::JWKConsensusRpcBcs,
    ProtocolId::JWKConsensusRpcJson,
];

#[derive(Clone)]
pub struct JWKConsensusNetworkClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<JWKConsensusMsg>>
    JWKConsensusNetworkClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    pub async fn send_rpc(
        &self,
        peer: PeerId,
        message: JWKConsensusMsg,
        rpc_timeout: Duration,
    ) -> Result<JWKConsensusMsg, Error> {
        let peer_network_id = self.get_peer_network_id_for_peer(peer);
        self.network_client
            .send_to_peer_rpc(message, rpc_timeout, peer_network_id)
            .await
    }

    pub async fn send_rpc_raw(
        &self,
        peer: PeerId,
        message: Bytes,
        rpc_timeout: Duration,
    ) -> Result<JWKConsensusMsg, Error> {
        let peer_network_id = self.get_peer_network_id_for_peer(peer);
        self.network_client
            .send_to_peer_rpc_raw(message, rpc_timeout, peer_network_id)
            .await
    }

    pub fn to_bytes_by_protocol(
        &self,
        peers: Vec<PeerId>,
        message: JWKConsensusMsg,
    ) -> anyhow::Result<HashMap<PeerId, Bytes>> {
        let peer_network_ids: Vec<PeerNetworkId> = peers
            .into_iter()
            .map(|peer| self.get_peer_network_id_for_peer(peer))
            .collect();
        Ok(self
            .network_client
            .to_bytes_by_protocol(peer_network_ids, message)?
            .into_iter()
            .map(|(peer_network_id, bytes)| (peer_network_id.peer_id(), bytes))
            .collect())
    }

    // TODO: we shouldn't need to expose this. Migrate the code to handle peer and network ids.
    fn get_peer_network_id_for_peer(&self, peer: PeerId) -> PeerNetworkId {
        PeerNetworkId::new(NetworkId::Validator, peer)
    }

    pub fn sort_peers_by_latency(&self, peers: &mut [PeerId]) {
        self.network_client
            .sort_peers_by_latency(NetworkId::Validator, peers)
    }
}
