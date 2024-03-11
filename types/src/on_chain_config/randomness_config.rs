// Copyright © Aptos Foundation

use crate::{move_any::{Any as MoveAny, AsMoveAny}, move_any, move_utils::as_move_value::AsMoveValue, on_chain_config::OnChainConfig};
use anyhow::anyhow;
use fixed::types::U64F64;
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};
use crate::move_fixed_point::FixedPoint64MoveStruct;

#[derive(Deserialize, Serialize)]
pub struct ConfigOff {}

impl AsMoveAny for ConfigOff {
    const MOVE_TYPE_NAME: &'static str = "0x1::randomness_config::ConfigOff";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConfigV1 {
    pub secrecy_threshold: FixedPoint64MoveStruct,
    pub reconstruction_threshold: FixedPoint64MoveStruct,
}

impl Default for ConfigV1 {
    fn default() -> Self {
        Self {
            secrecy_threshold: FixedPoint64MoveStruct::from_u64f64(U64F64::from_num(1) / U64F64::from_num(2)),
            reconstruction_threshold: FixedPoint64MoveStruct::from_u64f64(U64F64::from_num(2) / U64F64::from_num(3)),
        }
    }
}

impl AsMoveAny for ConfigV1 {
    const MOVE_TYPE_NAME: &'static str = "0x1::randomness_config::ConfigV1";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainRandomnessConfig {
    Off,
    V1(ConfigV1),
}

impl OnChainRandomnessConfig {
    pub fn default_enabled() -> Self {
        Self::V1(ConfigV1::default())
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
            OnChainRandomnessConfig::V1(_) => true,
        }
    }

    pub fn secrecy_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainRandomnessConfig::Off => None,
            OnChainRandomnessConfig::V1(v1) => Some(v1.secrecy_threshold.as_u64f64())
        }
    }

    pub fn reconstruct_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainRandomnessConfig::Off => None,
            OnChainRandomnessConfig::V1(v1) => Some(v1.reconstruction_threshold.as_u64f64())
        }
    }
}

impl OnChainConfig for OnChainRandomnessConfig {
    const MODULE_IDENTIFIER: &'static str = "randomness_config";
    const TYPE_IDENTIFIER: &'static str = "RandomnessConfig";

    fn deserialize_into_config(bytes: &[u8]) -> anyhow::Result<Self> {
        let variant = bcs::from_bytes::<MoveAny>(bytes)?;
        let variant_type_name = variant.type_name.clone();
        match variant_type_name.as_str() {
            ConfigOff::MOVE_TYPE_NAME => Ok(OnChainRandomnessConfig::Off),
            ConfigV1::MOVE_TYPE_NAME => {
                let v1 = move_any::Any::unpack::<ConfigV1>(ConfigV1::MOVE_TYPE_NAME, variant).map_err(|e|anyhow!("deserialization failed with move any unpack error: {e}"))?;
                Ok(OnChainRandomnessConfig::V1(v1))
            },
            _ => Err(anyhow!("unknown variant type name")),
        }
    }
}

impl AsMoveValue for OnChainRandomnessConfig {
    fn as_move_value(&self) -> MoveValue {
        let packed_variant = match self {
            OnChainRandomnessConfig::Off => ConfigOff {}.as_move_any(),
            OnChainRandomnessConfig::V1(v1) => v1.as_move_any(),
        };
        MoveValue::Struct(MoveStruct::Runtime(vec![packed_variant.as_move_value()]))
    }
}
