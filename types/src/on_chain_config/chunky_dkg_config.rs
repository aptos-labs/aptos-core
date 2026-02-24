// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    move_any::{Any as MoveAny, AsMoveAny},
    move_fixed_point::FixedPoint64MoveStruct,
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::OnChainConfig,
};
use anyhow::bail;
use fixed::types::U64F64;
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ConfigOff {}

impl AsMoveAny for ConfigOff {
    const MOVE_TYPE_NAME: &'static str = "0x1::chunky_dkg_config::ConfigOff";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConfigV1 {
    pub secrecy_threshold: FixedPoint64MoveStruct,
    pub reconstruction_threshold: FixedPoint64MoveStruct,
}

impl Default for ConfigV1 {
    fn default() -> Self {
        Self {
            secrecy_threshold: FixedPoint64MoveStruct::from_u64f64(
                U64F64::from_num(1) / U64F64::from_num(2),
            ),
            reconstruction_threshold: FixedPoint64MoveStruct::from_u64f64(
                U64F64::from_num(2) / U64F64::from_num(3),
            ),
        }
    }
}

impl AsMoveAny for ConfigV1 {
    const MOVE_TYPE_NAME: &'static str = "0x1::chunky_dkg_config::ConfigV1";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ChunkyDKGConfigMoveStruct {
    variant: MoveAny,
}

#[derive(Clone, Debug)]
pub enum OnChainChunkyDKGConfig {
    Off,
    V1(ConfigV1),
}

impl OnChainChunkyDKGConfig {
    pub fn new_v1(
        secrecy_threshold_in_percentage: u64,
        reconstruct_threshold_in_percentage: u64,
    ) -> Self {
        let secrecy_threshold = FixedPoint64MoveStruct::from_u64f64(
            U64F64::from_num(secrecy_threshold_in_percentage) / U64F64::from_num(100),
        );
        let reconstruction_threshold = FixedPoint64MoveStruct::from_u64f64(
            U64F64::from_num(reconstruct_threshold_in_percentage) / U64F64::from_num(100),
        );
        Self::V1(ConfigV1 {
            secrecy_threshold,
            reconstruction_threshold,
        })
    }

    /// Used by DKG and Consensus on a new epoch to determine the actual `OnChainChunkyDKGConfig` to be used.
    pub fn from_configs(onchain_raw_config: Option<ChunkyDKGConfigMoveStruct>) -> Self {
        // TODO(ibalajiarun): Implement manual disabling logic based on SeqNum
        onchain_raw_config
            .and_then(|onchain_raw| OnChainChunkyDKGConfig::try_from(onchain_raw).ok())
            .unwrap_or_else(OnChainChunkyDKGConfig::default_if_missing)
    }
}

impl TryFrom<ChunkyDKGConfigMoveStruct> for OnChainChunkyDKGConfig {
    type Error = anyhow::Error;

    fn try_from(value: ChunkyDKGConfigMoveStruct) -> Result<Self, Self::Error> {
        let ChunkyDKGConfigMoveStruct { variant } = value;
        let variant_type_name = variant.type_name.as_str();
        match variant_type_name {
            ConfigOff::MOVE_TYPE_NAME => Ok(OnChainChunkyDKGConfig::Off),
            ConfigV1::MOVE_TYPE_NAME => {
                let v1 = MoveAny::unpack(ConfigV1::MOVE_TYPE_NAME, variant)?;
                Ok(OnChainChunkyDKGConfig::V1(v1))
            },
            unknown => bail!("unknown variant type: {}", unknown),
        }
    }
}

impl From<OnChainChunkyDKGConfig> for ChunkyDKGConfigMoveStruct {
    fn from(value: OnChainChunkyDKGConfig) -> Self {
        let variant = match value {
            OnChainChunkyDKGConfig::Off => MoveAny::pack(ConfigOff::MOVE_TYPE_NAME, ConfigOff {}),
            OnChainChunkyDKGConfig::V1(v1) => MoveAny::pack(ConfigV1::MOVE_TYPE_NAME, v1),
        };
        ChunkyDKGConfigMoveStruct { variant }
    }
}

impl OnChainChunkyDKGConfig {
    pub fn default_enabled() -> Self {
        OnChainChunkyDKGConfig::V1(ConfigV1::default())
    }

    pub fn default_disabled() -> Self {
        OnChainChunkyDKGConfig::Off
    }

    pub fn default_if_missing() -> Self {
        OnChainChunkyDKGConfig::Off
    }

    pub fn default_for_genesis() -> Self {
        OnChainChunkyDKGConfig::Off
    }

    pub fn chunky_dkg_enabled(&self) -> bool {
        match self {
            OnChainChunkyDKGConfig::Off => false,
            OnChainChunkyDKGConfig::V1(_) => true,
        }
    }

    pub fn secrecy_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainChunkyDKGConfig::Off => None,
            OnChainChunkyDKGConfig::V1(v1) => Some(v1.secrecy_threshold.as_u64f64()),
        }
    }

    pub fn reconstruct_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainChunkyDKGConfig::Off => None,
            OnChainChunkyDKGConfig::V1(v1) => Some(v1.reconstruction_threshold.as_u64f64()),
        }
    }
}

impl OnChainConfig for ChunkyDKGConfigMoveStruct {
    const MODULE_IDENTIFIER: &'static str = "chunky_dkg_config";
    const TYPE_IDENTIFIER: &'static str = "ChunkyDKGConfig";
}

impl AsMoveValue for ChunkyDKGConfigMoveStruct {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.variant.as_move_value()]))
    }
}
