// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{external_resources::resource_fetcher::CachedResource, utils};
use anyhow::{anyhow, Result};
use aptos_types::keyless::Configuration;
use serde::{Deserialize, Serialize};

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
            .map(|v| utils::unhexlify_api_bytes(v.as_str()))
            .transpose()
            .map_err(|e| anyhow!("to_rust_repr() failed with unhexlify err: {e}"))?;
        let ret = Configuration {
            override_aud_vals: self.data.override_aud_vals.clone(),
            max_signatures_per_txn: self.data.max_signatures_per_txn,
            max_exp_horizon_secs: self.data.max_exp_horizon_secs.parse().map_err(|e| {
                anyhow!("to_rust_repr() failed at max_exp_horizon_secs convert: {e}")
            })?,
            training_wheels_pubkey,
            max_commited_epk_bytes: self.data.max_commited_epk_bytes,
            max_iss_val_bytes: self.data.max_iss_val_bytes,
            max_extra_field_bytes: self.data.max_extra_field_bytes,
            max_jwt_header_b64_bytes: self.data.max_jwt_header_b64_bytes,
        };
        Ok(ret)
    }
}

impl CachedResource for OnChainKeylessConfiguration {
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
