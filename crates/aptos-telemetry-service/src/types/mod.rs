// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod auth;
pub mod telemetry;
pub mod validator_set;

pub mod common {

    use crate::types::auth::Claims;
    use aptos_config::config::PeerRole;
    use aptos_types::chain_id::ChainId;
    use aptos_types::PeerId;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct EventIdentity {
        pub peer_id: PeerId,
        pub chain_id: ChainId,
        pub role_type: PeerRole,
        pub epoch: u64,
    }

    impl From<Claims> for EventIdentity {
        fn from(claims: Claims) -> Self {
            Self {
                peer_id: claims.peer_id,
                chain_id: claims.chain_id,
                role_type: claims.peer_role,
                epoch: claims.epoch,
            }
        }
    }
}

pub mod humio {
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Clone, Debug)]
    pub struct UnstructuredLog {
        pub fields: HashMap<String, String>,
        pub tags: HashMap<String, String>,
        pub messages: Vec<String>,
    }
}
