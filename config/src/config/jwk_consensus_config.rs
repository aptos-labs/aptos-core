// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct JWKConsensusConfig {
    pub max_network_channel_size: usize,
    /// Additional PEM-encoded certificate roots for trust observation HTTPS fetches.
    /// These do not affect JWK provider requests or the default system roots.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trust_observation_extra_root_certificates_pem: Vec<String>,
}

impl Default for JWKConsensusConfig {
    fn default() -> Self {
        Self {
            max_network_channel_size: 256,
            trust_observation_extra_root_certificates_pem: Vec::new(),
        }
    }
}
