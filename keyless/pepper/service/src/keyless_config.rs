// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::watcher::{unhexlify_api_bytes, ExternalResource};
use anyhow::{anyhow, Result};
use aptos_infallible::RwLock;
use aptos_types::keyless::Configuration;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct TrainingWheelsPubKey {
    vec: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct OnChainKeylessConfiguration {
    /// Some type info returned by node API.
    pub r#type: String,
    pub data: ConfigData,
}

impl OnChainKeylessConfiguration {
    pub fn to_rust_repr(&self) -> Result<aptos_types::keyless::Configuration> {
        let training_wheels_pubkey = self
            .data
            .training_wheels_pubkey
            .vec
            .first()
            .map(|v| unhexlify_api_bytes(v.as_str()))
            .transpose()
            .map_err(|e| anyhow!("to_rust_repr() failed with unhexlify err: {e}"))?;
        // We so far only consume `training_wheels_pubkey`.
        let ret = Configuration {
            override_aud_vals: vec![],
            max_signatures_per_txn: 0,
            max_exp_horizon_secs: 0,
            training_wheels_pubkey,
            max_commited_epk_bytes: 0,
            max_iss_val_bytes: 0,
            max_extra_field_bytes: 0,
            max_jwt_header_b64_bytes: 0,
        };
        Ok(ret)
    }
}

impl ExternalResource for OnChainKeylessConfiguration {
    fn resource_name() -> String {
        "OnChainKeylessConfiguration".to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ConfigData {
    pub max_commited_epk_bytes: u16,
    pub max_exp_horizon_secs: String,
    pub max_extra_field_bytes: u16,
    pub max_iss_val_bytes: u16,
    pub max_jwt_header_b64_bytes: u32,
    pub max_signatures_per_txn: u16,
    pub override_aud_vals: Vec<String>,
    pub training_wheels_pubkey: TrainingWheelsPubKey,
}

pub static ONCHAIN_KEYLESS_CONFIG: Lazy<Arc<RwLock<Option<OnChainKeylessConfiguration>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));
