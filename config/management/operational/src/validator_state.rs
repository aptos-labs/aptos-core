// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{json_rpc::JsonRpcClientWrapper, validator_config::DecryptedValidatorConfig};
use diem_global_constants::{
    CONSENSUS_KEY, FULLNODE_NETWORK_KEY, OWNER_ACCOUNT, VALIDATOR_NETWORK_KEY,
};
use diem_management::{
    config::ConfigPath, error::Error, secure_backend::ValidatorBackend, storage::to_x25519,
};
use serde::Serialize;
use structopt::StructOpt;

#[derive(Debug, Serialize)]
pub struct VerifyValidatorStateResult {
    /// Check if the consensus key held in secure storage matches
    /// that registered on-chain for the validator.
    pub consensus_key_match: Option<bool>,

    /// Check if the consensus key is unique
    pub consensus_key_unique: Option<bool>,

    /// Check if the fullnode network key held in secure storage matches
    /// that registered on-chain.
    pub fullnode_network_key_match: Option<bool>,

    /// Check if a validator is in the latest validator set on-chain.
    pub in_validator_set: Option<bool>,

    /// Check if the validator network key held in secure storage matches
    /// that registered on-chain.
    pub validator_network_key_match: Option<bool>,
}

impl VerifyValidatorStateResult {
    pub fn is_valid_state(&self) -> bool {
        self.in_validator_set == Some(true)
            && self.consensus_key_match == Some(true)
            && self.consensus_key_unique == Some(true)
            && self.validator_network_key_match == Some(true)
            && self.fullnode_network_key_match == Some(true)
    }
}

#[derive(Debug, StructOpt)]
pub struct VerifyValidatorState {
    #[structopt(flatten)]
    config: ConfigPath,
    #[structopt(long, required_unless = "config")]
    json_server: Option<String>,
    #[structopt(flatten)]
    validator_backend: ValidatorBackend,
}

impl VerifyValidatorState {
    pub fn execute(self) -> Result<VerifyValidatorStateResult, Error> {
        // Load the config, storage backend and create a json rpc client.
        let config = self
            .config
            .load()?
            .override_json_server(&self.json_server)
            .override_validator_backend(&self.validator_backend.validator_backend)?;
        let storage = config.validator_backend();
        let client = JsonRpcClientWrapper::new(config.json_server);
        let owner_account = storage.account_address(OWNER_ACCOUNT)?;

        let validator_infos = client.validator_set(None)?;

        let mut result = VerifyValidatorStateResult {
            consensus_key_match: None,
            consensus_key_unique: None,
            fullnode_network_key_match: None,
            in_validator_set: None,
            validator_network_key_match: None,
        };

        // Verify if the validator is in the set.
        result.in_validator_set = Some(
            validator_infos
                .iter()
                .any(|vi| vi.account_address() == &owner_account),
        );

        // Fetch the current on-chain config for this operator's owner.
        // Check if the consensus key held in secure storage matches
        // that registered on-chain.
        let validator_config = client.validator_config(owner_account).and_then(|vc| {
            DecryptedValidatorConfig::from_validator_config_resource(&vc, owner_account)
        })?;
        let storage_key = storage.ed25519_public_from_private(CONSENSUS_KEY)?;
        result.consensus_key_match = Some(storage_key == validator_config.consensus_public_key);

        // Check if the consensus key is unique
        result.consensus_key_unique = Some(!validator_infos.iter().any(|vi| {
            vi.account_address() != &owner_account && vi.consensus_public_key() == &storage_key
        }));

        // Check if the validator network key held in secure storage
        // matches that registered on-chain.
        let storage_key = storage.ed25519_public_from_private(VALIDATOR_NETWORK_KEY)?;
        result.validator_network_key_match = Some(
            Some(to_x25519(storage_key)?)
                == validator_config
                    .validator_network_address
                    .find_noise_proto(),
        );

        // Check if the fullnode network key held in secure storage
        // matches that registered on-chain.
        let storage_key = storage.ed25519_public_from_private(FULLNODE_NETWORK_KEY)?;
        result.fullnode_network_key_match = Some(
            Some(to_x25519(storage_key)?)
                == validator_config.fullnode_network_address.find_noise_proto(),
        );

        Ok(result)

        // TODO(khiemngo): Check if all keys match locally when compared with the validator infos
    }
}
