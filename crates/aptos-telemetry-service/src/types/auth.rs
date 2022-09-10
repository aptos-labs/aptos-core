// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::RoleType;
use aptos_crypto::x25519;
use aptos_types::{chain_id::ChainId, PeerId};
use serde::{Deserialize, Serialize};

use super::common::NodeType;

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthRequest {
    pub chain_id: ChainId,
    pub peer_id: PeerId,
    #[serde(default = "default_role_type")]
    pub role_type: RoleType,
    pub server_public_key: x25519::PublicKey,
    pub handshake_msg: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub handshake_msg: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    pub chain_id: ChainId,
    pub peer_id: PeerId,
    pub node_type: NodeType,
    pub epoch: u64,
    pub exp: usize,
    pub iat: usize,
}

fn default_role_type() -> RoleType {
    RoleType::Validator
}
