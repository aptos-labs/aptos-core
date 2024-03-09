// Copyright © Aptos Foundation

use crate::on_chain_config::OnChainConfig;
use anyhow::format_err;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainJWKConsensusConfig {
    Off,
    On {
        oidc_providers: BTreeMap<String, String>,
    },
}

impl OnChainJWKConsensusConfig {
    pub fn default_enabled() -> Self {
        Self::On {
            oidc_providers: BTreeMap::new(),
        }
    }

    pub fn default_disabled() -> Self {
        Self::Off
    }

    pub fn default_if_missing() -> Self {
        Self::Off
    }

    pub fn default_for_genesis() -> Self {
        Self::On {
            oidc_providers: BTreeMap::new(),
        }
    }

    pub fn jwk_consensus_enabled(&self) -> bool {
        match self {
            OnChainJWKConsensusConfig::Off => false,
            OnChainJWKConsensusConfig::On { .. } => true,
        }
    }

    pub fn oidc_providers(&self) -> Option<&BTreeMap<String, String>> {
        match self {
            OnChainJWKConsensusConfig::Off => None,
            OnChainJWKConsensusConfig::On { oidc_providers } => Some(oidc_providers),
        }
    }
}

impl OnChainConfig for OnChainJWKConsensusConfig {
    const MODULE_IDENTIFIER: &'static str = "jwk_consensus_config";
    const TYPE_IDENTIFIER: &'static str = "JWKConsensusConfig";

    fn deserialize_into_config(bytes: &[u8]) -> anyhow::Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes)
            .map_err(|e| format_err!("[on-chain config] Failed to deserialize into config: {}", e))
    }
}
