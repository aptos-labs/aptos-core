// Copyright © Aptos Foundation

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
pub struct OnChainRandomnessConfig {
    variant: MoveAny,
}

pub enum Foo {
    Off,
    V1(ConfigV1),
}

impl TryFrom<OnChainRandomnessConfig> for Foo {
    type Error = anyhow::Error;

    fn try_from(value: OnChainRandomnessConfig) -> Result<Self, Self::Error> {
        let OnChainRandomnessConfig { variant } = value;
        let variant_type_name = variant.type_name.as_str();
        match variant_type_name {
            ConfigOff::MOVE_TYPE_NAME => Ok(Foo::Off),
            ConfigV1::MOVE_TYPE_NAME => {
                let v1 = MoveAny::unpack(ConfigV1::MOVE_TYPE_NAME, variant)
                    .map_err(|e| anyhow!("unpack as v1 failed: {e}"))?;
                Ok(Foo::V1(v1))
            },
            _ => Err(anyhow!("unknown variant type")),
        }
    }
}

impl From<Foo> for OnChainRandomnessConfig {
    fn from(value: Foo) -> Self {
        let variant = match value {
            Foo::Off => MoveAny::pack(ConfigOff::MOVE_TYPE_NAME, ConfigOff {}),
            Foo::V1(v1) => MoveAny::pack(ConfigV1::MOVE_TYPE_NAME, v1),
        };
        OnChainRandomnessConfig { variant }
    }
}

impl OnChainRandomnessConfig {
    pub fn default_enabled() -> Self {
        Foo::V1(ConfigV1::default()).into()
    }

    pub fn default_disabled() -> Self {
        Foo::Off.into()
    }

    pub fn default_if_missing() -> Self {
        Foo::Off.into()
    }

    pub fn default_for_genesis() -> Self {
        Foo::Off.into() //TODO: change to `V1` after randomness is ready.
    }

    pub fn randomness_enabled(&self) -> bool {
        match Foo::try_from(self.clone()) {
            Ok(Foo::Off) => false,
            Ok(Foo::V1(_)) => true,
            Err(_) => false,
        }
    }

    pub fn secrecy_threshold(&self) -> Option<U64F64> {
        match Foo::try_from(self.clone()) {
            Ok(Foo::Off) => None,
            Ok(Foo::V1(v1)) => Some(v1.secrecy_threshold.as_u64f64()),
            Err(_) => None,
        }
    }

    pub fn reconstruct_threshold(&self) -> Option<U64F64> {
        match Foo::try_from(self.clone()) {
            Ok(Foo::Off) => None,
            Ok(Foo::V1(v1)) => Some(v1.reconstruction_threshold.as_u64f64()),
            Err(_) => None,
        }
    }
}

impl OnChainConfig for OnChainRandomnessConfig {
    const MODULE_IDENTIFIER: &'static str = "randomness_config";
    const TYPE_IDENTIFIER: &'static str = "RandomnessConfig";
}

impl AsMoveValue for OnChainRandomnessConfig {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.variant.as_move_value()]))
    }
}
