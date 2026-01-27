// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod auth;
pub mod telemetry;

pub mod common {

    use crate::types::auth::Claims;
    use aptos_config::config::PeerSet;
    use aptos_types::{chain_id::ChainId, PeerId};
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, fmt};
    use uuid::Uuid;

    pub type EpochNum = u64;
    pub type EpochedPeerStore = HashMap<ChainId, (EpochNum, PeerSet)>;
    pub type PeerStore = HashMap<ChainId, PeerSet>;
    pub type ChainCommonName = String;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct EventIdentity {
        pub peer_id: PeerId,
        pub chain_id: ChainId,
        pub role_type: NodeType,
        pub epoch: u64,
        pub uuid: Uuid,
    }

    impl From<Claims> for EventIdentity {
        fn from(claims: Claims) -> Self {
            Self {
                peer_id: claims.peer_id,
                chain_id: claims.chain_id,
                role_type: claims.node_type,
                epoch: claims.epoch,
                uuid: claims.run_uuid,
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
    pub enum NodeType {
        Validator,
        ValidatorFullNode,
        PublicFullNode,
        Unknown,
        UnknownValidator,
        UnknownFullNode,
        /// Custom node type with a user-defined name (e.g., "ShelbyStorageProvider")
        /// These are nodes that are registered in the on-chain allowlist.
        Custom(String),
        /// Unknown/untrusted custom node type - nodes that authenticated via custom contract
        /// endpoint but are NOT in the on-chain allowlist. Requires `allow_unknown_nodes: true`
        /// in the custom contract config. Routed to untrusted sinks for separate attribution.
        CustomUnknown(String),
    }

    impl NodeType {
        /// Get the string representation of the node type
        /// For Custom types, returns "custom({name})" to prevent ambiguity with built-in types
        pub fn as_str(&self) -> String {
            match self {
                NodeType::Validator => "validator".to_string(),
                NodeType::ValidatorFullNode => "validator_fullnode".to_string(),
                NodeType::PublicFullNode => "public_fullnode".to_string(),
                NodeType::Unknown => "unknown_peer".to_string(),
                NodeType::UnknownValidator => "unknown_validator".to_string(),
                NodeType::UnknownFullNode => "unknown_fullnode".to_string(),
                NodeType::Custom(name) => format!("custom({})", name),
                NodeType::CustomUnknown(name) => format!("custom_unknown({})", name),
            }
        }

        /// Check if this is an unknown/untrusted node type
        pub fn is_unknown(&self) -> bool {
            matches!(
                self,
                NodeType::Unknown
                    | NodeType::UnknownValidator
                    | NodeType::UnknownFullNode
                    | NodeType::CustomUnknown(_)
            )
        }

        /// Check if this is a custom contract node type (trusted or unknown)
        pub fn is_custom(&self) -> bool {
            matches!(self, NodeType::Custom(_) | NodeType::CustomUnknown(_))
        }

        /// Get the contract name if this is a custom node type
        pub fn custom_contract_name(&self) -> Option<&str> {
            match self {
                NodeType::Custom(name) | NodeType::CustomUnknown(name) => Some(name),
                _ => None,
            }
        }
    }

    impl fmt::Debug for NodeType {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self)
        }
    }

    impl fmt::Display for NodeType {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.as_str())
        }
    }
}

pub mod response {
    use crate::errors::ServiceError;
    use aptos_crypto::x25519;
    use reqwest::StatusCode;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct IndexResponse {
        pub public_key: x25519::PublicKey,
    }

    /// Health check response for liveness/readiness probes
    #[derive(Serialize, Deserialize)]
    pub struct HealthResponse {
        pub status: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct ErrorResponse {
        code: u16,
        message: String,
    }

    impl ErrorResponse {
        pub fn new(code: StatusCode, message: String) -> Self {
            Self {
                code: code.as_u16(),
                message,
            }
        }
    }

    impl From<&ServiceError> for ErrorResponse {
        fn from(err: &ServiceError) -> Self {
            Self::new(err.http_status_code(), err.to_string())
        }
    }
}

pub mod humio {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Deserialize, Serialize, Clone, Debug)]
    pub struct UnstructuredLog {
        pub fields: HashMap<String, String>,
        pub tags: HashMap<String, String>,
        pub messages: Vec<String>,
    }
}
