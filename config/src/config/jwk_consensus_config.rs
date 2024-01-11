// Copyright © Aptos Foundation

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct JWKConsensusConfig {
    pub max_network_channel_size: usize,
}

impl Default for JWKConsensusConfig {
    fn default() -> Self {
        Self {
            max_network_channel_size: 256,
        }
    }
}
