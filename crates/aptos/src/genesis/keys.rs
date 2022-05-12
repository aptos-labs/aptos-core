// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliError, CliTypedResult, PromptOptions},
        utils::{check_if_file_exists, read_from_file, write_to_file},
    },
    genesis::{
        config::{HostAndPort, ValidatorConfiguration},
        git::{from_yaml, to_yaml, GitOptions},
    },
    op::key,
    CliCommand,
};
use aptos_config::{config::IdentityBlob, keys::ConfigKey};
use aptos_crypto::{ed25519::Ed25519PrivateKey, x25519, PrivateKey};
use aptos_types::transaction::authenticator::AuthenticationKey;
use async_trait::async_trait;
use clap::Parser;
use move_deps::move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const PRIVATE_KEYS_FILE: &str = "private-keys.yaml";
const VALIDATOR_FILE: &str = "validator-identity.yaml";
const VFN_FILE: &str = "validator-full-node-identity.yaml";

/// Generate account key, consensus key, and network key for a validator
#[derive(Parser)]
pub struct GenerateKeys {
    #[clap(flatten)]
    prompt_options: PromptOptions,
    /// Output path for the three keys
    #[clap(long, parse(from_os_str), default_value = ".")]
    output_dir: PathBuf,
}

#[async_trait]
impl CliCommand<Vec<PathBuf>> for GenerateKeys {
    fn command_name(&self) -> &'static str {
        "GenerateKeys"
    }

    async fn execute(self) -> CliTypedResult<Vec<PathBuf>> {
        let keys_file = self.output_dir.join(PRIVATE_KEYS_FILE);
        let validator_file = self.output_dir.join(VALIDATOR_FILE);
        let vfn_file = self.output_dir.join(VFN_FILE);
        check_if_file_exists(keys_file.as_path(), self.prompt_options)?;
        check_if_file_exists(validator_file.as_path(), self.prompt_options)?;
        check_if_file_exists(vfn_file.as_path(), self.prompt_options)?;

        let account_key = ConfigKey::new(key::GenerateKey::generate_ed25519_in_memory());
        let consensus_key = ConfigKey::new(key::GenerateKey::generate_ed25519_in_memory());
        let validator_network_key = ConfigKey::new(key::GenerateKey::generate_x25519_in_memory()?);
        let full_node_network_key = ConfigKey::new(key::GenerateKey::generate_x25519_in_memory()?);

        let account_address =
            AuthenticationKey::ed25519(&account_key.public_key()).derived_address();

        // Build these for use later as node identity
        let validator_blob = IdentityBlob {
            account_address: Some(account_address),
            account_key: Some(account_key.private_key()),
            consensus_key: Some(consensus_key.private_key()),
            network_key: validator_network_key.private_key(),
        };
        let vfn_blob = IdentityBlob {
            account_address: Some(account_address),
            account_key: None,
            consensus_key: None,
            network_key: full_node_network_key.private_key(),
        };

        let config = PrivateIdentity {
            account_address,
            account_key: account_key.private_key(),
            consensus_key: consensus_key.private_key(),
            full_node_network_key: full_node_network_key.private_key(),
            validator_network_key: validator_network_key.private_key(),
        };

        // Create the directory if it doesn't exist
        if !self.output_dir.exists() || !self.output_dir.is_dir() {
            std::fs::create_dir(&self.output_dir)
                .map_err(|e| CliError::IO(self.output_dir.to_str().unwrap().to_string(), e))?
        };

        write_to_file(
            keys_file.as_path(),
            PRIVATE_KEYS_FILE,
            to_yaml(&config)?.as_bytes(),
        )?;
        write_to_file(
            validator_file.as_path(),
            VALIDATOR_FILE,
            to_yaml(&validator_blob)?.as_bytes(),
        )?;
        write_to_file(vfn_file.as_path(), VFN_FILE, to_yaml(&vfn_blob)?.as_bytes())?;
        Ok(vec![keys_file, validator_file, vfn_file])
    }
}

/// Type for serializing private keys file
#[derive(Deserialize, Serialize)]
pub struct PrivateIdentity {
    account_address: AccountAddress,
    account_key: Ed25519PrivateKey,
    consensus_key: Ed25519PrivateKey,
    full_node_network_key: x25519::PrivateKey,
    validator_network_key: x25519::PrivateKey,
}

/// Set ValidatorConfiguration for a single validator in the git repository
#[derive(Parser)]
pub struct SetValidatorConfiguration {
    /// Username
    #[clap(long)]
    username: String,
    #[clap(flatten)]
    git_options: GitOptions,
    /// Path to folder with account.key, consensus.key, and network.key
    #[clap(long, parse(from_os_str), default_value = ".")]
    keys_dir: PathBuf,
    /// Host and port pair for the validator e.g. 127.0.0.1:6180
    #[clap(long)]
    validator_host: HostAndPort,
    /// Host and port pair for the fullnode e.g. 127.0.0.1:6180
    #[clap(long)]
    full_node_host: Option<HostAndPort>,
    /// Stake amount for stake distribution
    #[clap(long, default_value = "1")]
    stake_amount: u64,
}

#[async_trait]
impl CliCommand<()> for SetValidatorConfiguration {
    fn command_name(&self) -> &'static str {
        "SetValidatorConfiguration"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let private_keys_path = self.keys_dir.join(PRIVATE_KEYS_FILE);
        let bytes = read_from_file(private_keys_path.as_path())?;
        let key_files: PrivateIdentity =
            from_yaml(&String::from_utf8(bytes).map_err(CliError::from)?)?;
        let account_address = key_files.account_address;
        let account_key = key_files.account_key.public_key();
        let consensus_key = key_files.consensus_key.public_key();
        let validator_network_key = key_files.validator_network_key.public_key();
        let full_node_network_key = key_files.full_node_network_key.public_key();

        let credentials = ValidatorConfiguration {
            account_address,
            consensus_key,
            account_key,
            validator_network_key,
            validator_host: self.validator_host,
            full_node_network_key,
            full_node_host: self.full_node_host,
            stake_amount: self.stake_amount,
        };

        self.git_options
            .get_client()?
            .put(&self.username, &credentials)
    }
}
