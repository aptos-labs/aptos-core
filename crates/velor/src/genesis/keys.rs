// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliError, CliTypedResult, OptionalPoolAddressArgs, PromptOptions, RngArgs},
        utils::{
            check_if_file_exists, create_dir_if_not_exist, current_dir, dir_default_to_current,
            read_from_file, write_to_user_only_file,
        },
    },
    genesis::git::{from_yaml, to_yaml, GitOptions, LAYOUT_FILE, OPERATOR_FILE, OWNER_FILE},
    governance::CompileScriptFunction,
    CliCommand,
};
use velor_genesis::{
    config::{HostAndPort, Layout, OperatorConfiguration, OwnerConfiguration},
    keys::{generate_key_objects, PublicIdentity},
};
use velor_types::{
    account_address::AccountAddress,
    transaction::{Script, Transaction, WriteSetPayload},
};
use async_trait::async_trait;
use clap::Parser;
use std::path::{Path, PathBuf};

const PRIVATE_KEYS_FILE: &str = "private-keys.yaml";
pub const PUBLIC_KEYS_FILE: &str = "public-keys.yaml";
const VALIDATOR_FILE: &str = "validator-identity.yaml";
const VFN_FILE: &str = "validator-full-node-identity.yaml";

/// Generate keys for a new validator
///
/// Generates account key, consensus key, and network key for a validator
/// These keys are used for running a validator or operator in a network
#[derive(Parser)]
pub struct GenerateKeys {
    /// Output directory for the key files
    #[clap(long, value_parser)]
    pub(crate) output_dir: Option<PathBuf>,

    #[clap(flatten)]
    pub(crate) pool_address_args: OptionalPoolAddressArgs,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
    #[clap(flatten)]
    pub rng_args: RngArgs,
}

#[async_trait]
impl CliCommand<Vec<PathBuf>> for GenerateKeys {
    fn command_name(&self) -> &'static str {
        "GenerateKeys"
    }

    async fn execute(self) -> CliTypedResult<Vec<PathBuf>> {
        let output_dir = dir_default_to_current(self.output_dir.clone())?;

        let private_keys_file = output_dir.join(PRIVATE_KEYS_FILE);
        let public_keys_file = output_dir.join(PUBLIC_KEYS_FILE);
        let validator_file = output_dir.join(VALIDATOR_FILE);
        let vfn_file = output_dir.join(VFN_FILE);
        check_if_file_exists(private_keys_file.as_path(), self.prompt_options)?;
        check_if_file_exists(public_keys_file.as_path(), self.prompt_options)?;
        check_if_file_exists(validator_file.as_path(), self.prompt_options)?;
        check_if_file_exists(vfn_file.as_path(), self.prompt_options)?;

        let mut key_generator = self.rng_args.key_generator()?;
        let (mut validator_blob, mut vfn_blob, private_identity, public_identity) =
            generate_key_objects(&mut key_generator)?;

        // Allow for the owner to be different than the operator
        if let Some(pool_address) = self.pool_address_args.pool_address {
            validator_blob.account_address = Some(pool_address);
            vfn_blob.account_address = Some(pool_address);
        }

        // Create the directory if it doesn't exist
        create_dir_if_not_exist(output_dir.as_path())?;

        write_to_user_only_file(
            private_keys_file.as_path(),
            PRIVATE_KEYS_FILE,
            to_yaml(&private_identity)?.as_bytes(),
        )?;
        write_to_user_only_file(
            public_keys_file.as_path(),
            PUBLIC_KEYS_FILE,
            to_yaml(&public_identity)?.as_bytes(),
        )?;
        write_to_user_only_file(
            validator_file.as_path(),
            VALIDATOR_FILE,
            to_yaml(&validator_blob)?.as_bytes(),
        )?;
        write_to_user_only_file(vfn_file.as_path(), VFN_FILE, to_yaml(&vfn_blob)?.as_bytes())?;
        Ok(vec![
            public_keys_file,
            private_keys_file,
            validator_file,
            vfn_file,
        ])
    }
}

/// Set validator configuration for a single validator
///
/// This will set the validator configuration for a single validator in the git repository.
/// It will have to be run for each validator expected at genesis.
#[derive(Parser)]
pub struct SetValidatorConfiguration {
    /// Name of the validator
    #[clap(long)]
    pub(crate) username: String,

    /// Host and port pair for the validator e.g. 127.0.0.1:6180 or velorlabs.com:6180
    #[clap(long)]
    pub(crate) validator_host: HostAndPort,

    /// Host and port pair for the fullnode e.g. 127.0.0.1:6180 or velorlabs.com:6180
    #[clap(long)]
    pub(crate) full_node_host: Option<HostAndPort>,

    /// Stake amount for stake distribution
    #[clap(long, default_value_t = 1)]
    pub(crate) stake_amount: u64,

    /// Commission rate to pay operator
    ///
    /// This is a percentage between 0% and 100%
    #[clap(long, default_value_t = 0)]
    pub(crate) commission_percentage: u64,

    /// Whether the validator will be joining the genesis validator set
    ///
    /// If set this validator will already be in the validator set at genesis
    #[clap(long)]
    pub(crate) join_during_genesis: bool,

    /// Path to private identity generated from GenerateKeys
    #[clap(long, value_parser)]
    pub(crate) owner_public_identity_file: Option<PathBuf>,

    /// Path to operator public identity, defaults to owner identity
    #[clap(long, value_parser)]
    pub(crate) operator_public_identity_file: Option<PathBuf>,

    /// Path to voter public identity, defaults to owner identity
    #[clap(long, value_parser)]
    pub(crate) voter_public_identity_file: Option<PathBuf>,

    #[clap(flatten)]
    pub(crate) git_options: GitOptions,
}

#[async_trait]
impl CliCommand<()> for SetValidatorConfiguration {
    fn command_name(&self) -> &'static str {
        "SetValidatorConfiguration"
    }

    async fn execute(self) -> CliTypedResult<()> {
        // Load owner
        let owner_keys_file = if let Some(owner_keys_file) = self.owner_public_identity_file {
            owner_keys_file
        } else {
            current_dir()?.join(PUBLIC_KEYS_FILE)
        };
        let owner_identity = read_public_identity_file(owner_keys_file.as_path())?;

        // Load voter
        let voter_identity = if let Some(voter_keys_file) = self.voter_public_identity_file {
            read_public_identity_file(voter_keys_file.as_path())?
        } else {
            owner_identity.clone()
        };

        // Load operator
        let (operator_identity, operator_keys_file) =
            if let Some(operator_keys_file) = self.operator_public_identity_file {
                (
                    read_public_identity_file(operator_keys_file.as_path())?,
                    operator_keys_file,
                )
            } else {
                (owner_identity.clone(), owner_keys_file)
            };

        // Extract the possible optional fields
        let consensus_public_key =
            if let Some(consensus_public_key) = operator_identity.consensus_public_key {
                consensus_public_key
            } else {
                return Err(CliError::CommandArgumentError(format!(
                    "Failed to read consensus public key from public identity file {}",
                    operator_keys_file.display()
                )));
            };

        let validator_network_public_key = if let Some(validator_network_public_key) =
            operator_identity.validator_network_public_key
        {
            validator_network_public_key
        } else {
            return Err(CliError::CommandArgumentError(format!(
                "Failed to read validator network public key from public identity file {}",
                operator_keys_file.display()
            )));
        };

        let consensus_proof_of_possession = if let Some(consensus_proof_of_possession) =
            operator_identity.consensus_proof_of_possession
        {
            consensus_proof_of_possession
        } else {
            return Err(CliError::CommandArgumentError(format!(
                "Failed to read consensus proof of possession from public identity file {}",
                operator_keys_file.display()
            )));
        };

        // Only add the public key if there is a full node
        let full_node_network_public_key = if self.full_node_host.is_some() {
            operator_identity.full_node_network_public_key
        } else {
            None
        };

        // Build operator configuration file
        let operator_config = OperatorConfiguration {
            operator_account_address: operator_identity.account_address.into(),
            operator_account_public_key: operator_identity.account_public_key.clone(),
            consensus_public_key,
            consensus_proof_of_possession,
            validator_network_public_key,
            validator_host: self.validator_host,
            full_node_network_public_key,
            full_node_host: self.full_node_host,
        };

        let owner_config = OwnerConfiguration {
            owner_account_address: owner_identity.account_address.into(),
            owner_account_public_key: owner_identity.account_public_key,
            voter_account_address: voter_identity.account_address.into(),
            voter_account_public_key: voter_identity.account_public_key,
            operator_account_address: operator_identity.account_address.into(),
            operator_account_public_key: operator_identity.account_public_key,
            stake_amount: self.stake_amount,
            commission_percentage: self.commission_percentage,
            join_during_genesis: self.join_during_genesis,
        };

        let directory = PathBuf::from(&self.username);
        let operator_file = directory.join(OPERATOR_FILE);
        let owner_file = directory.join(OWNER_FILE);

        let git_client = self.git_options.get_client()?;
        git_client.put(operator_file.as_path(), &operator_config)?;
        git_client.put(owner_file.as_path(), &owner_config)
    }
}

pub fn read_public_identity_file(public_identity_file: &Path) -> CliTypedResult<PublicIdentity> {
    let bytes = read_from_file(public_identity_file)?;
    from_yaml(&String::from_utf8(bytes).map_err(CliError::from)?)
}

/// Generate a Layout template file
///
/// This will generate a layout template file for genesis with some default values.  To start a
/// new chain, these defaults should be carefully thought through and chosen.
#[derive(Parser)]
pub struct GenerateLayoutTemplate {
    /// Path of the output layout template
    #[clap(long, value_parser, default_value = LAYOUT_FILE)]
    pub(crate) output_file: PathBuf,

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<()> for GenerateLayoutTemplate {
    fn command_name(&self) -> &'static str {
        "GenerateLayoutTemplate"
    }

    async fn execute(self) -> CliTypedResult<()> {
        check_if_file_exists(self.output_file.as_path(), self.prompt_options)?;
        let layout = Layout::default();

        write_to_user_only_file(
            self.output_file.as_path(),
            &self.output_file.display().to_string(),
            to_yaml(&layout)?.as_bytes(),
        )
    }
}

/// Generate a WriteSet genesis
///
/// This will compile a Move script and generate a writeset from that script.
#[derive(Parser)]
pub struct GenerateAdminWriteSet {
    /// Path of the output genesis file
    #[clap(long, value_parser)]
    pub(crate) output_file: PathBuf,

    /// Address of the account which execute this script.
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) execute_as: AccountAddress,

    #[clap(flatten)]
    pub(crate) compile_proposal_args: CompileScriptFunction,

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<()> for GenerateAdminWriteSet {
    fn command_name(&self) -> &'static str {
        "GenerateAdminWriteSet"
    }

    async fn execute(self) -> CliTypedResult<()> {
        check_if_file_exists(self.output_file.as_path(), self.prompt_options)?;
        let (bytecode, _script_hash) = self
            .compile_proposal_args
            .compile("GenerateAdminWriteSet", self.prompt_options)?;

        let txn = Transaction::GenesisTransaction(WriteSetPayload::Script {
            execute_as: self.execute_as,
            script: Script::new(bytecode, vec![], vec![]),
        });

        write_to_user_only_file(
            self.output_file.as_path(),
            &self.output_file.display().to_string(),
            &bcs::to_bytes(&txn).map_err(CliError::from)?,
        )
    }
}
