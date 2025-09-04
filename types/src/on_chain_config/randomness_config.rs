// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_any::{Any as MoveAny, AsMoveAny},
    move_fixed_point::FixedPoint64MoveStruct,
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::OnChainConfig,
};
use anyhow::anyhow;
use fixed::types::U64F64;
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

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
    const MOVE_TYPE_NAME: &'static str = "0x1::randomness_config::ConfigV1";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConfigV2 {
    pub secrecy_threshold: FixedPoint64MoveStruct,
    pub reconstruction_threshold: FixedPoint64MoveStruct,
    pub fast_path_secrecy_threshold: FixedPoint64MoveStruct,
}

impl Default for ConfigV2 {
    fn default() -> Self {
        Self {
            secrecy_threshold: FixedPoint64MoveStruct::from_u64f64(
                U64F64::from_num(1) / U64F64::from_num(2),
            ),
            reconstruction_threshold: FixedPoint64MoveStruct::from_u64f64(
                U64F64::from_num(2) / U64F64::from_num(3),
            ),
            fast_path_secrecy_threshold: FixedPoint64MoveStruct::from_u64f64(
                U64F64::from_num(2) / U64F64::from_num(3),
            ),
        }
    }
}

impl AsMoveAny for ConfigV2 {
    const MOVE_TYPE_NAME: &'static str = "0x1::randomness_config::ConfigV2";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct RandomnessConfigSeqNum {
    pub seq_num: u64,
}

impl RandomnessConfigSeqNum {
    pub fn default_if_missing() -> Self {
        Self { seq_num: 0 }
    }
}

impl OnChainConfig for RandomnessConfigSeqNum {
    const MODULE_IDENTIFIER: &'static str = "randomness_config_seqnum";
    const TYPE_IDENTIFIER: &'static str = "RandomnessConfigSeqNum";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct RandomnessConfigMoveStruct {
    variant: MoveAny,
}

#[derive(Clone, Debug)]
pub enum OnChainRandomnessConfig {
    Off,
    V1(ConfigV1),
    V2(ConfigV2),
}

impl OnChainRandomnessConfig {
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

    pub fn new_v2(
        secrecy_threshold_in_percentage: u64,
        reconstruct_threshold_in_percentage: u64,
        fast_path_secrecy_threshold_in_percentage: u64,
    ) -> Self {
        let secrecy_threshold = FixedPoint64MoveStruct::from_u64f64(
            U64F64::from_num(secrecy_threshold_in_percentage) / U64F64::from_num(100),
        );
        let reconstruction_threshold = FixedPoint64MoveStruct::from_u64f64(
            U64F64::from_num(reconstruct_threshold_in_percentage) / U64F64::from_num(100),
        );
        let fast_path_secrecy_threshold = FixedPoint64MoveStruct::from_u64f64(
            U64F64::from_num(fast_path_secrecy_threshold_in_percentage) / U64F64::from_num(100),
        );
        Self::V2(ConfigV2 {
            secrecy_threshold,
            reconstruction_threshold,
            fast_path_secrecy_threshold,
        })
    }

    /// Used by DKG and Consensus on a new epoch to determine the actual `OnChainRandomnessConfig` to be used.
    pub fn from_configs(
        local_seqnum: u64,
        onchain_seqnum: u64,
        onchain_raw_config: Option<RandomnessConfigMoveStruct>,
    ) -> Self {
        if local_seqnum > onchain_seqnum {
            Self::default_disabled()
        } else {
            onchain_raw_config
                .and_then(|onchain_raw| OnChainRandomnessConfig::try_from(onchain_raw).ok())
                .unwrap_or_else(OnChainRandomnessConfig::default_if_missing)
        }
    }
}

impl TryFrom<RandomnessConfigMoveStruct> for OnChainRandomnessConfig {
    type Error = anyhow::Error;

    fn try_from(value: RandomnessConfigMoveStruct) -> Result<Self, Self::Error> {
        let RandomnessConfigMoveStruct { variant } = value;
        let variant_type_name = variant.type_name.as_str();
        match variant_type_name {
            ConfigOff::MOVE_TYPE_NAME => Ok(OnChainRandomnessConfig::Off),
            ConfigV1::MOVE_TYPE_NAME => {
                let v1 = MoveAny::unpack(ConfigV1::MOVE_TYPE_NAME, variant)
                    .map_err(|e| anyhow!("unpack as v1 failed: {e}"))?;
                Ok(OnChainRandomnessConfig::V1(v1))
            },
            ConfigV2::MOVE_TYPE_NAME => {
                let v2 = MoveAny::unpack(ConfigV2::MOVE_TYPE_NAME, variant)
                    .map_err(|e| anyhow!("unpack as v2 failed: {e}"))?;
                Ok(OnChainRandomnessConfig::V2(v2))
            },
            _ => Err(anyhow!("unknown variant type")),
        }
    }
}

impl From<OnChainRandomnessConfig> for RandomnessConfigMoveStruct {
    fn from(value: OnChainRandomnessConfig) -> Self {
        let variant = match value {
            OnChainRandomnessConfig::Off => MoveAny::pack(ConfigOff::MOVE_TYPE_NAME, ConfigOff {}),
            OnChainRandomnessConfig::V1(v1) => MoveAny::pack(ConfigV1::MOVE_TYPE_NAME, v1),
            OnChainRandomnessConfig::V2(v2) => MoveAny::pack(ConfigV2::MOVE_TYPE_NAME, v2),
        };
        RandomnessConfigMoveStruct { variant }
    }
}

impl OnChainRandomnessConfig {
    pub fn default_enabled() -> Self {
        OnChainRandomnessConfig::V2(ConfigV2::default())
    }

    pub fn default_disabled() -> Self {
        OnChainRandomnessConfig::Off
    }

    pub fn default_if_missing() -> Self {
        OnChainRandomnessConfig::Off
    }

    pub fn default_for_genesis() -> Self {
        OnChainRandomnessConfig::V2(ConfigV2::default())
    }

    pub fn randomness_enabled(&self) -> bool {
        match self {
            OnChainRandomnessConfig::Off => false,
            OnChainRandomnessConfig::V1(_) => true,
            OnChainRandomnessConfig::V2(_) => true,
        }
    }

    pub fn fast_randomness_enabled(&self) -> bool {
        match self {
            OnChainRandomnessConfig::Off => false,
            OnChainRandomnessConfig::V1(_) => false,
            OnChainRandomnessConfig::V2(_) => true,
        }
    }

    pub fn secrecy_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainRandomnessConfig::Off => None,
            OnChainRandomnessConfig::V1(v1) => Some(v1.secrecy_threshold.as_u64f64()),
            OnChainRandomnessConfig::V2(v2) => Some(v2.secrecy_threshold.as_u64f64()),
        }
    }

    pub fn reconstruct_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainRandomnessConfig::Off => None,
            OnChainRandomnessConfig::V1(v1) => Some(v1.reconstruction_threshold.as_u64f64()),
            OnChainRandomnessConfig::V2(v2) => Some(v2.reconstruction_threshold.as_u64f64()),
        }
    }

    pub fn fast_path_secrecy_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainRandomnessConfig::Off => None,
            OnChainRandomnessConfig::V1(_) => None,
            OnChainRandomnessConfig::V2(v2) => Some(v2.fast_path_secrecy_threshold.as_u64f64()),
        }
    }
}

impl OnChainConfig for RandomnessConfigMoveStruct {
    const MODULE_IDENTIFIER: &'static str = "randomness_config";
    const TYPE_IDENTIFIER: &'static str = "RandomnessConfig";
}

impl AsMoveValue for RandomnessConfigMoveStruct {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.variant.as_move_value()]))
    }
}
