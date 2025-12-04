// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
