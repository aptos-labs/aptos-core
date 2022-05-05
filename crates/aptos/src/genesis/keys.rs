// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{CliTypedResult, EncodingType},
    genesis::{
        config::{HostAndPort, ValidatorConfiguration},
        git::GitOptions,
    },
    op::key,
    CliCommand,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, x25519, PrivateKey};
use async_trait::async_trait;
use clap::Parser;
use std::path::PathBuf;

const ACCOUNT_KEY_FILE: &str = "account.key";
const CONSENSUS_KEY_FILE: &str = "consensus.key";
const NETWORK_KEY_FILE: &str = "network.key";

/// Generate account key, consensus key, and network key for a validator
#[derive(Parser)]
pub struct GenerateKeys {
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
        let account_key_path = self.output_dir.join(ACCOUNT_KEY_FILE);
        let consensus_key_path = self.output_dir.join(CONSENSUS_KEY_FILE);
        let network_key_path = self.output_dir.join(NETWORK_KEY_FILE);
        let _ = key::GenerateKey::generate_ed25519(EncodingType::Hex, &account_key_path).await?;
        let _ = key::GenerateKey::generate_ed25519(EncodingType::Hex, &consensus_key_path).await?;
        let _ = key::GenerateKey::generate_x25519(EncodingType::Hex, &network_key_path).await?;
        Ok(vec![account_key_path, consensus_key_path, network_key_path])
    }
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
}

#[async_trait]
impl CliCommand<()> for SetValidatorConfiguration {
    fn command_name(&self) -> &'static str {
        "SetValidatorConfiguration"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let account_key_path = self.keys_dir.join(ACCOUNT_KEY_FILE);
        let consensus_key_path = self.keys_dir.join(CONSENSUS_KEY_FILE);
        let network_key_path = self.keys_dir.join(NETWORK_KEY_FILE);
        let account_key: Ed25519PrivateKey =
            EncodingType::Hex.load_key(ACCOUNT_KEY_FILE, &account_key_path)?;
        let consensus_key: Ed25519PrivateKey =
            EncodingType::Hex.load_key(CONSENSUS_KEY_FILE, &consensus_key_path)?;
        let network_key: x25519::PrivateKey =
            EncodingType::Hex.load_key(NETWORK_KEY_FILE, &network_key_path)?;

        let credentials = ValidatorConfiguration {
            consensus_key: consensus_key.public_key(),
            account_key: account_key.public_key(),
            network_key: network_key.public_key(),
            validator_host: self.validator_host,
            full_node_host: self.full_node_host,
        };

        self.git_options
            .get_client()?
            .put(&self.username, &credentials)
    }
}
