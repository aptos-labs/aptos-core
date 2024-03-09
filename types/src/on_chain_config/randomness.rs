// Copyright © Aptos Foundation

use crate::on_chain_config::OnChainConfig;
use anyhow::format_err;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainRandomnessConfig {
    Off,
    On,
}

impl OnChainRandomnessConfig {
    pub fn default_enabled() -> Self {
        Self::On
    }

    pub fn default_disabled() -> Self {
        Self::Off
    }

    pub fn default_if_missing() -> Self {
        Self::Off
    }

    pub fn default_for_genesis() -> Self {
        Self::Off //TODO: change to ON after randomness is ready.
    }

    pub fn randomness_enabled(&self) -> bool {
        match self {
            OnChainRandomnessConfig::Off => false,
            OnChainRandomnessConfig::On => true,
        }
    }
}

impl OnChainConfig for OnChainRandomnessConfig {
    const MODULE_IDENTIFIER: &'static str = "randomness_config";
    const TYPE_IDENTIFIER: &'static str = "RandomnessConfig";

    fn deserialize_into_config(bytes: &[u8]) -> anyhow::Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes)
            .map_err(|e| format_err!("[on-chain config] Failed to deserialize into config: {}", e))
    }
}
