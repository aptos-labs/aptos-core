// Copyright © Aptos Foundation

use aptos_network::ProtocolId;

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
