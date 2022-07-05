// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliError, CliTypedResult, PromptOptions, RngArgs},
        utils::{check_if_file_exists, read_from_file, write_to_user_only_file},
    },
    genesis::git::{from_yaml, to_yaml, GitOptions},
    CliCommand,
};
use aptos_crypto::{bls12381, PrivateKey};
use aptos_genesis::{
    config::{HostAndPort, ValidatorConfiguration},
    keys::{generate_key_objects, PrivateIdentity},
};
use async_trait::async_trait;
use clap::Parser;
use std::path::PathBuf;

const PRIVATE_KEYS_FILE: &str = "private-keys.yaml";
const VALIDATOR_FILE: &str = "validator-identity.yaml";
const VFN_FILE: &str = "validator-full-node-identity.yaml";

/// Generate account key, consensus key, and network key for a validator
#[derive(Parser)]
pub struct GenerateKeys {
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
    #[clap(flatten)]
    pub rng_args: RngArgs,
    /// Output path for the three keys
    #[clap(long, parse(from_os_str), default_value = ".")]
    pub(crate) output_dir: PathBuf,
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

        let mut key_generator = self.rng_args.key_generator()?;
        let (validator_blob, vfn_blob, private_identity) =
            generate_key_objects(&mut key_generator)?;

        // Create the directory if it doesn't exist
        if !self.output_dir.exists() || !self.output_dir.is_dir() {
            std::fs::create_dir(&self.output_dir)
                .map_err(|e| CliError::IO(self.output_dir.to_str().unwrap().to_string(), e))?
        };

        write_to_user_only_file(
            keys_file.as_path(),
            PRIVATE_KEYS_FILE,
            to_yaml(&private_identity)?.as_bytes(),
        )?;
        write_to_user_only_file(
            validator_file.as_path(),
            VALIDATOR_FILE,
            to_yaml(&validator_blob)?.as_bytes(),
        )?;
        write_to_user_only_file(vfn_file.as_path(), VFN_FILE, to_yaml(&vfn_blob)?.as_bytes())?;
        Ok(vec![keys_file, validator_file, vfn_file])
    }
}

/// Set ValidatorConfiguration for a single validator in the git repository
#[derive(Parser)]
pub struct SetValidatorConfiguration {
    /// Username
    #[clap(long)]
    pub(crate) username: String,
    #[clap(flatten)]
    pub(crate) git_options: GitOptions,
    /// Path to folder with account.key, consensus.key, and network.key
    #[clap(long, parse(from_os_str), default_value = ".")]
    pub(crate) keys_dir: PathBuf,
    /// Host and port pair for the validator e.g. 127.0.0.1:6180
    #[clap(long)]
    pub(crate) validator_host: HostAndPort,
    /// Host and port pair for the fullnode e.g. 127.0.0.1:6180
    #[clap(long)]
    pub(crate) full_node_host: Option<HostAndPort>,
    /// Stake amount for stake distribution
    #[clap(long, default_value_t = 1)]
    pub(crate) stake_amount: u64,
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
        let account_key = key_files.account_private_key.public_key();
        let consensus_key = key_files.consensus_private_key.public_key();
        let proof_of_possession =
            bls12381::ProofOfPossession::create(&key_files.consensus_private_key);
        let validator_network_key = key_files.validator_network_private_key.public_key();

        let full_node_network_key = if self.full_node_host.is_some() {
            Some(key_files.full_node_network_private_key.public_key())
        } else {
            None
        };

        let credentials = ValidatorConfiguration {
            account_address,
            consensus_public_key: consensus_key,
            proof_of_possession,
            account_public_key: account_key,
            validator_network_public_key: validator_network_key,
            validator_host: self.validator_host,
            full_node_network_public_key: full_node_network_key,
            full_node_host: self.full_node_host,
            stake_amount: self.stake_amount,
        };

        self.git_options
            .get_client()?
            .put(&self.username, &credentials)
    }
}
