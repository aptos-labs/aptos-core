// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{json_rpc::JsonRpcClientWrapper, validator_config::DecryptedValidatorConfig};
use diem_global_constants::{CONSENSUS_KEY, OWNER_ACCOUNT};
use diem_management::error::Error;
use serde::Serialize;
use structopt::StructOpt;

#[derive(Debug, Default, Serialize)]
pub struct VerifyValidatorStateResult {
    /// Check if a validator is in the latest validator set on-chain.
    pub in_validator_set: Option<bool>,
    /// Check if the consensus key held in secure storage matches
    /// that registered on-chain for the validator.
    pub consensus_key_match: Option<bool>,
}

impl VerifyValidatorStateResult {
    pub fn is_state_consistent(&self) -> bool {
        self.in_validator_set == Some(true) && self.consensus_key_match == Some(true)
    }
}

#[derive(Debug, StructOpt)]
pub struct VerifyValidatorState {
    #[structopt(long, required_unless = "config")]
    json_server: Option<String>,
    #[structopt(flatten)]
    validator_config: diem_management::validator_config::ValidatorConfig,
}

impl VerifyValidatorState {
    pub fn execute(self) -> Result<VerifyValidatorStateResult, Error> {
        // Load the config, storage backend and create a json rpc client.
        let config = self
            .validator_config
            .config()?
            .override_json_server(&self.json_server);
        let storage = config.validator_backend();
        let client = JsonRpcClientWrapper::new(config.json_server);
        let owner_account = storage.account_address(OWNER_ACCOUNT)?;

        // Verify if the validator is in the set
        let in_validator_set = client
            .validator_set(None)?
            .iter()
            .any(|vi| vi.account_address() == &owner_account);

        // TODO(khiemngo): consider return early if the validator is not in the set

        // Fetch the current on-chain config for this operator's owner
        let validator_config = client.validator_config(owner_account).and_then(|vc| {
            DecryptedValidatorConfig::from_validator_config_resource(&vc, owner_account)
        })?;

        let storage_key = storage.ed25519_public_from_private(CONSENSUS_KEY)?;
        let consensus_key_match = storage_key == validator_config.consensus_public_key;

        // TODO(khiemngo): add checks for validator/fullnode network addresses
        // TODO(khiemngo): add check for key uniqueness

        Ok(VerifyValidatorStateResult {
            in_validator_set: Some(in_validator_set),
            consensus_key_match: Some(consensus_key_match),
        })
    }
}
