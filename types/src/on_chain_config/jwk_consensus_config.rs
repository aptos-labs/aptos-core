// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwks::SupportedOIDCProviders,
    move_any::{Any as MoveAny, Any, AsMoveAny},
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{FeatureFlag, Features, OnChainConfig},
};
use anyhow::anyhow;
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConfigOff {}

impl AsMoveAny for ConfigOff {
    const MOVE_TYPE_NAME: &'static str = "0x1::jwk_consensus_config::ConfigOff";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct OIDCProvider {
    pub name: String,
    pub config_url: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConfigV1 {
    pub oidc_providers: Vec<OIDCProvider>,
}

impl AsMoveAny for ConfigV1 {
    const MOVE_TYPE_NAME: &'static str = "0x1::jwk_consensus_config::ConfigV1";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainJWKConsensusConfig {
    Off,
    V1(ConfigV1),
}

impl OnChainJWKConsensusConfig {
    pub fn default_enabled() -> Self {
        Self::V1(ConfigV1 {
            oidc_providers: vec![OIDCProvider {
                name: "https://accounts.google.com".to_string(),
                config_url: "https://accounts.google.com/.well-known/openid-configuration"
                    .to_string(),
            }],
        })
    }

    pub fn default_disabled() -> Self {
        Self::Off
    }

    pub fn default_if_missing() -> Self {
        Self::Off
    }

    pub fn default_for_genesis() -> Self {
        // Here it is supposed to use `default_enabled()`.
        // Using an empty list instead to avoid DDoSing the CI infra or the actual providers.
        Self::V1(ConfigV1 {
            oidc_providers: vec![],
        })
    }

    pub fn jwk_consensus_enabled(&self) -> bool {
        match self {
            OnChainJWKConsensusConfig::Off => false,
            OnChainJWKConsensusConfig::V1 { .. } => true,
        }
    }

    pub fn oidc_providers_cloned(&self) -> Vec<OIDCProvider> {
        match self {
            OnChainJWKConsensusConfig::Off => vec![],
            OnChainJWKConsensusConfig::V1(v1) => v1.oidc_providers.clone(),
        }
    }
}

impl OnChainConfig for OnChainJWKConsensusConfig {
    const MODULE_IDENTIFIER: &'static str = "jwk_consensus_config";
    const TYPE_IDENTIFIER: &'static str = "JWKConsensusConfig";

    fn deserialize_into_config(bytes: &[u8]) -> anyhow::Result<Self> {
        let variant = bcs::from_bytes::<MoveAny>(bytes)?;
        match variant.type_name.as_str() {
            ConfigOff::MOVE_TYPE_NAME => Ok(OnChainJWKConsensusConfig::Off),
            ConfigV1::MOVE_TYPE_NAME => {
                let config_v1 = Any::unpack::<ConfigV1>(ConfigV1::MOVE_TYPE_NAME, variant).map_err(|e|anyhow!("OnChainJWKConsensusConfig deserialization failed with ConfigV1 unpack error: {e}"))?;
                Ok(OnChainJWKConsensusConfig::V1(config_v1))
            },
            _ => Err(anyhow!("unknown variant type")),
        }
    }
}

impl AsMoveValue for OnChainJWKConsensusConfig {
    fn as_move_value(&self) -> MoveValue {
        let packed_variant = match self {
            OnChainJWKConsensusConfig::Off => ConfigOff {}.as_move_any(),
            OnChainJWKConsensusConfig::V1(v1) => v1.as_move_any(),
        };
        MoveValue::Struct(MoveStruct::Runtime(vec![packed_variant.as_move_value()]))
    }
}

/// Before `JWKConsensusConfig` is initialized, convert from `Features` and `SupportedOIDCProviders` instead.
impl From<(Option<Features>, Option<SupportedOIDCProviders>)> for OnChainJWKConsensusConfig {
    fn from(
        (features, supported_oidc_providers): (Option<Features>, Option<SupportedOIDCProviders>),
    ) -> Self {
        if let Some(features) = features {
            if features.is_enabled(FeatureFlag::JWK_CONSENSUS) {
                let oidc_providers = supported_oidc_providers
                    .unwrap_or_default()
                    .providers
                    .into_iter()
                    .filter_map(|deprecated| OIDCProvider::try_from(deprecated).ok())
                    .collect();
                OnChainJWKConsensusConfig::V1(ConfigV1 { oidc_providers })
            } else {
                OnChainJWKConsensusConfig::Off
            }
        } else {
            OnChainJWKConsensusConfig::Off
        }
    }
}
