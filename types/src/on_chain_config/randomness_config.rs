// Copyright © Aptos Foundation

use crate::{move_any::Any as MoveAny, on_chain_config::OnChainConfig};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainRandomnessConfig {
    Off,
    V1,
}

impl OnChainRandomnessConfig {
    pub fn default_enabled() -> Self {
        Self::V1
    }

    pub fn default_disabled() -> Self {
        Self::Off
    }

    pub fn default_if_missing() -> Self {
        Self::Off
    }

    pub fn default_for_genesis() -> Self {
        Self::Off //TODO: change to `V1` after randomness is ready.
    }

    pub fn randomness_enabled(&self) -> bool {
        match self {
            OnChainRandomnessConfig::Off => false,
            OnChainRandomnessConfig::V1 => true,
        }
    }
}

impl OnChainConfig for OnChainRandomnessConfig {
    const MODULE_IDENTIFIER: &'static str = "randomness_config";
    const TYPE_IDENTIFIER: &'static str = "RandomnessConfig";

    fn deserialize_into_config(bytes: &[u8]) -> anyhow::Result<Self> {
        let variant = bcs::from_bytes::<MoveAny>(bytes)?;
        match variant.type_name.as_str() {
            "0x1::randomness_config::ConfigOff" => Ok(OnChainRandomnessConfig::Off),
            "0x1::randomness_config::ConfigV1" => Ok(OnChainRandomnessConfig::V1),
            _ => Err(anyhow!("unknown variant type name")),
        }
    }
}
