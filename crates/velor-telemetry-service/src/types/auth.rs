// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::common::NodeType;
use velor_config::config::RoleType;
use velor_crypto::x25519;
use velor_types::{chain_id::ChainId, PeerId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthRequest {
    pub chain_id: ChainId,
    pub peer_id: PeerId,
    #[serde(default = "default_role_type")]
    pub role_type: RoleType,
    pub server_public_key: x25519::PublicKey,
    pub handshake_msg: Vec<u8>,
    #[serde(default = "default_uuid")]
    pub run_uuid: Uuid,
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
    pub run_uuid: Uuid,
}

fn default_role_type() -> RoleType {
    RoleType::Validator
}

fn default_uuid() -> Uuid {
    Uuid::default()
}

impl Claims {
    #[cfg(test)]
    pub(crate) fn test() -> Self {
        use chrono::{Duration, Utc};

        Self {
            chain_id: ChainId::test(),
            peer_id: PeerId::random(),
            node_type: NodeType::Validator,
            epoch: 10,
            exp: Utc::now().timestamp() as usize,
            iat: Utc::now()
                .checked_add_signed(Duration::seconds(3600))
                .unwrap()
                .timestamp() as usize,
            run_uuid: Uuid::default(),
        }
    }
}
