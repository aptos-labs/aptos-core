// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliError, CliTypedResult},
        utils::{read_from_file, write_to_file},
    },
    genesis::{
        config::{HostAndPort, ValidatorConfiguration},
        git::{from_yaml, to_yaml, GitOptions},
    },
    op::key,
    CliCommand,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, x25519, PrivateKey};
use aptos_types::transaction::authenticator::AuthenticationKey;
use async_trait::async_trait;
use clap::Parser;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const PRIVATE_KEYS_FILE: &str = "private-keys.yml";

/// Generate account key, consensus key, and network key for a validator
#[derive(Parser)]
pub struct GenerateKeys {
    /// Output path for the three keys
    #[clap(long, parse(from_os_str), default_value = ".")]
    output_dir: PathBuf,
}

#[async_trait]
impl CliCommand<PathBuf> for GenerateKeys {
    fn command_name(&self) -> &'static str {
        "GenerateKeys"
    }

    async fn execute(self) -> CliTypedResult<PathBuf> {
        let account_key = key::GenerateKey::generate_ed25519_in_memory();
        let consensus_key = key::GenerateKey::generate_ed25519_in_memory();
        // Start network key based off of the account key, we can update it later
        let network_key =
            x25519::PrivateKey::from_ed25519_private_bytes(&account_key.to_bytes()).unwrap();
        let keys_file = self.output_dir.join(PRIVATE_KEYS_FILE);

        let account_address =
            AuthenticationKey::ed25519(&account_key.public_key()).derived_address();

        let config = KeysAndAccount {
            account_address,
            account_key,
            consensus_key,
            network_key,
        };
        write_to_file(
            keys_file.as_path(),
            "private_keys.yaml",
            to_yaml(&config)?.as_bytes(),
        )?;
        Ok(keys_file)
    }
}

#[derive(Deserialize, Serialize)]
pub struct KeysAndAccount {
    account_address: AccountAddress,
    account_key: Ed25519PrivateKey,
    consensus_key: Ed25519PrivateKey,
    network_key: x25519::PrivateKey,
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
        let key_files: KeysAndAccount =
            from_yaml(&String::from_utf8(bytes).map_err(CliError::from)?)?;
        let account_address = key_files.account_address;
        let account_key = key_files.account_key.public_key();
        let consensus_key = key_files.consensus_key.public_key();
        let network_key = key_files.network_key.public_key();

        let credentials = ValidatorConfiguration {
            account_address,
            consensus_key,
            account_key,
            network_key,
            validator_host: self.validator_host,
            full_node_host: self.full_node_host,
            stake_amount: self.stake_amount,
        };

        self.git_options
            .get_client()?
            .put(&self.username, &credentials)
    }
}
