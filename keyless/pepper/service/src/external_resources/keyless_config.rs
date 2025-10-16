// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use anyhow::{anyhow, Result};
use aptos_types::keyless::Configuration;
use serde::{Deserialize, Serialize};

/// This struct is a representation of an OnChainKeylessConfiguration resource as found on-chain.
/// See, for example:
/// https://fullnode.testnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Configuration
///
/// Example JSON:
/// {
///  "type": "0x1::keyless_account::Configuration",
///  "data": {
///    "max_commited_epk_bytes": 93,
///    "max_exp_horizon_secs": "10000000",
///    "max_extra_field_bytes": 350,
///    "max_iss_val_bytes": 120,
///    "max_jwt_header_b64_bytes": 300,
///    "max_signatures_per_txn": 3,
///     "override_aud_vals": [],
///     "training_wheels_pubkey": {
///       "vec": [
///         "0x1388de358cf4701696bd58ed4b96e9d670cbbb914b888be1ceda6374a3098ed4"
///       ]
///     }
///   }
/// }
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct OnChainKeylessConfiguration {
    pub r#type: String, // Note: "type" is a reserved keyword, so we use raw identifier syntax
    pub data: KeylessConfigurationData,
}

impl OnChainKeylessConfiguration {
    #[cfg(test)]
    /// Creates a new OnChainKeylessConfiguration for testing
    pub fn new_for_testing() -> Self {
        // Create a test configuration
        let configuration = Configuration::new_for_testing();

        // Get the training wheels public key
        let training_wheels_pubkey = match configuration.training_wheels_pubkey {
            Some(public_key) => {
                let public_key_hex = hex::encode(&public_key);
                let public_key_string = format!("0x{}", public_key_hex);
                TrainingWheelsPubKey {
                    vec: vec![public_key_string],
                }
            },
            None => TrainingWheelsPubKey { vec: vec![] },
        };

        // Get the corresponding on-chain configuration data
        let data = KeylessConfigurationData {
            max_commited_epk_bytes: configuration.max_commited_epk_bytes,
            max_exp_horizon_secs: configuration.max_exp_horizon_secs.to_string(),
            max_extra_field_bytes: configuration.max_extra_field_bytes,
            max_iss_val_bytes: configuration.max_iss_val_bytes,
            max_jwt_header_b64_bytes: configuration.max_jwt_header_b64_bytes,
            max_signatures_per_txn: configuration.max_signatures_per_txn,
            override_aud_vals: configuration.override_aud_vals,
            training_wheels_pubkey,
        };

        // Return the on-chain keyless configuration
        OnChainKeylessConfiguration {
            r#type: "0x1::keyless_account::Configuration".to_string(),
            data,
        }
    }

    /// Converts the on-chain keyless configuration to the internal representation
    pub fn get_keyless_configuration(&self) -> Result<Configuration> {
        // Extract the training wheels public key
        let configuration_data = &self.data;
        let training_wheels_pubkey = configuration_data
            .training_wheels_pubkey
            .vec
            .first()
            .map(|v| utils::unhexlify_api_bytes(v.as_str()))
            .transpose()
            .map_err(|error| anyhow!("Failed to unhexlify training_wheels_pubkey: {}", error))?;

        // Get the max_exp_horizon_secs as u64
        let max_exp_horizon_secs = configuration_data
            .max_exp_horizon_secs
            .parse()
            .map_err(|error| anyhow!("Failed to parse max_exp_horizon_secs as u64: {}", error))?;

        // Return the configuration
        let configuration = Configuration {
            override_aud_vals: configuration_data.override_aud_vals.clone(),
            max_signatures_per_txn: configuration_data.max_signatures_per_txn,
            max_exp_horizon_secs,
            training_wheels_pubkey,
            max_commited_epk_bytes: configuration_data.max_commited_epk_bytes,
            max_iss_val_bytes: configuration_data.max_iss_val_bytes,
            max_extra_field_bytes: configuration_data.max_extra_field_bytes,
            max_jwt_header_b64_bytes: configuration_data.max_jwt_header_b64_bytes,
        };
        Ok(configuration)
    }
}

/// The data fields of the OnChainKeylessConfiguration resource
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct KeylessConfigurationData {
    pub max_commited_epk_bytes: u16,
    pub max_exp_horizon_secs: String,
    pub max_extra_field_bytes: u16,
    pub max_iss_val_bytes: u16,
    pub max_jwt_header_b64_bytes: u32,
    pub max_signatures_per_txn: u16,
    pub override_aud_vals: Vec<String>,
    pub training_wheels_pubkey: TrainingWheelsPubKey,
}

/// The training wheels public key of the OnChainKeylessConfiguration resource
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct TrainingWheelsPubKey {
    vec: Vec<String>,
}
