// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::JWKConsensusMsg;
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_network2::{
    application::{error::Error, interface::NetworkClientInterface},
    ProtocolId,
};
use move_core_types::account_address::AccountAddress as PeerId;
use std::time::Duration;

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
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, peer);
        self.network_client
            .send_to_peer_rpc(message, rpc_timeout, peer_network_id)
            .await
    }
}
