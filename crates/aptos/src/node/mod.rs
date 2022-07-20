// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::PromptOptions;
use crate::common::utils::prompt_yes_with_override;
use crate::config::GlobalConfig;
use crate::{
    common::{
        types::{
            CliCommand, CliError, CliResult, CliTypedResult, ProfileOptions, RestOptions,
            TransactionOptions,
        },
        utils::read_from_file,
    },
    genesis::git::from_yaml,
};
use aptos_config::config::NodeConfig;
use aptos_crypto::{bls12381, x25519, ValidCryptoMaterialStringExt};
use aptos_faucet::FaucetArgs;
use aptos_genesis::config::{HostAndPort, ValidatorConfiguration};
use aptos_rest_client::{Response, Transaction};
use aptos_types::chain_id::ChainId;
use aptos_types::{account_address::AccountAddress, account_config::CORE_CODE_ADDRESS};
use async_trait::async_trait;
use clap::Parser;
use hex::FromHex;
use rand::rngs::StdRng;
use rand::SeedableRng;
use reqwest::Url;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{
    path::PathBuf,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::Instant;

/// Tool for manipulating nodes
///
#[derive(Parser)]
pub enum NodeTool {
    AddStake(AddStake),
    UnlockStake(UnlockStake),
    WithdrawStake(WithdrawStake),
    IncreaseLockup(IncreaseLockup),
    RegisterValidatorCandidate(RegisterValidatorCandidate),
    JoinValidatorSet(JoinValidatorSet),
    LeaveValidatorSet(LeaveValidatorSet),
    ShowValidatorConfig(ShowValidatorConfig),
    ShowValidatorSet(ShowValidatorSet),
    ShowValidatorStake(ShowValidatorStake),
    RunLocalTestnet(RunLocalTestnet),
}

impl NodeTool {
    pub async fn execute(self) -> CliResult {
        use NodeTool::*;
        match self {
            AddStake(tool) => tool.execute_serialized().await,
            UnlockStake(tool) => tool.execute_serialized().await,
            WithdrawStake(tool) => tool.execute_serialized().await,
            IncreaseLockup(tool) => tool.execute_serialized().await,
            RegisterValidatorCandidate(tool) => tool.execute_serialized().await,
            JoinValidatorSet(tool) => tool.execute_serialized().await,
            LeaveValidatorSet(tool) => tool.execute_serialized().await,
            ShowValidatorSet(tool) => tool.execute_serialized().await,
            ShowValidatorStake(tool) => tool.execute_serialized().await,
            ShowValidatorConfig(tool) => tool.execute_serialized().await,
            RunLocalTestnet(tool) => tool.execute_serialized_without_logger().await,
        }
    }
}

/// Stake coins for an account to the stake pool
#[derive(Parser)]
pub struct AddStake {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Amount of coins to add to stake
    #[clap(long)]
    pub amount: u64,
}

#[async_trait]
impl CliCommand<Transaction> for AddStake {
    fn command_name(&self) -> &'static str {
        "AddStake"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "add_stake",
                vec![],
                vec![bcs::to_bytes(&self.amount)?],
            )
            .await
    }
}

/// Unlock staked coins
///
/// Coins can only be unlocked if they no longer have an applied lockup period
#[derive(Parser)]
pub struct UnlockStake {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Amount of coins to unlock
    #[clap(long)]
    pub amount: u64,
}

#[async_trait]
impl CliCommand<Transaction> for UnlockStake {
    fn command_name(&self) -> &'static str {
        "UnlockStake"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "unlock",
                vec![],
                vec![bcs::to_bytes(&self.amount)?],
            )
            .await
    }
}

/// Withdraw all unlocked staked coins
///
/// Before calling `WithdrawStake`, `UnlockStake` must be called first.
#[derive(Parser)]
pub struct WithdrawStake {
    #[clap(flatten)]
    pub(crate) node_op_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Transaction> for WithdrawStake {
    fn command_name(&self) -> &'static str {
        "WithdrawStake"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.node_op_options
            .submit_script_function(AccountAddress::ONE, "Stake", "withdraw", vec![], vec![])
            .await
    }
}

/// Increase lockup of all staked coins in an account
#[derive(Parser)]
pub struct IncreaseLockup {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Number of seconds to increase the lockup period by
    ///
    /// Examples: '1d', '5 days', '1 month'
    #[clap(long, parse(try_from_str=parse_duration::parse))]
    pub(crate) lockup_duration: Duration,
}

#[async_trait]
impl CliCommand<Transaction> for IncreaseLockup {
    fn command_name(&self) -> &'static str {
        "IncreaseLockup"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        if self.lockup_duration.is_zero() {
            return Err(CliError::CommandArgumentError(
                "Must provide a non-zero lockup duration".to_string(),
            ));
        }

        let lockup_timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_add(self.lockup_duration.as_secs());

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "increase_lockup",
                vec![],
                vec![bcs::to_bytes(&lockup_timestamp_secs)?],
            )
            .await
    }
}

/// Register the current account as a Validator candidate
#[derive(Parser)]
pub struct RegisterValidatorCandidate {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Validator Configuration file, created from the `genesis set-validator-configuration` command
    #[clap(long)]
    pub(crate) validator_config_file: Option<PathBuf>,
    /// Hex encoded Consensus public key
    #[clap(long, parse(try_from_str = bls12381::PublicKey::from_encoded_string))]
    pub(crate) consensus_public_key: Option<bls12381::PublicKey>,
    /// Host and port pair for the validator e.g. 127.0.0.1:6180
    #[clap(long)]
    pub(crate) validator_host: Option<HostAndPort>,
    /// Validator x25519 public network key
    #[clap(long, parse(try_from_str = x25519::PublicKey::from_encoded_string))]
    pub(crate) validator_network_public_key: Option<x25519::PublicKey>,
    /// Host and port pair for the fullnode e.g. 127.0.0.1:6180.  Optional
    #[clap(long)]
    pub(crate) full_node_host: Option<HostAndPort>,
    /// Full node x25519 public network key
    #[clap(long, parse(try_from_str = x25519::PublicKey::from_encoded_string))]
    pub(crate) full_node_network_public_key: Option<x25519::PublicKey>,
}

impl RegisterValidatorCandidate {
    fn process_inputs(
        &self,
    ) -> CliTypedResult<(
        bls12381::PublicKey,
        x25519::PublicKey,
        Option<x25519::PublicKey>,
        HostAndPort,
        Option<HostAndPort>,
    )> {
        let validator_config = self.read_validator_config()?;

        let consensus_public_key = if let Some(ref consensus_public_key) = self.consensus_public_key
        {
            consensus_public_key.clone()
        } else if let Some(ref validator_config) = validator_config {
            validator_config.consensus_public_key.clone()
        } else {
            return Err(CliError::CommandArgumentError(
                "Must provide either --validator-config-file or --consensus-public-key".to_string(),
            ));
        };

        let validator_network_public_key =
            if let Some(public_key) = self.validator_network_public_key {
                public_key
            } else if let Some(ref validator_config) = validator_config {
                validator_config.validator_network_public_key
            } else {
                return Err(CliError::CommandArgumentError(
                    "Must provide either --validator-config-file or --validator-network-public-key"
                        .to_string(),
                ));
            };

        let full_node_network_public_key =
            if let Some(public_key) = self.full_node_network_public_key {
                Some(public_key)
            } else if let Some(ref validator_config) = validator_config {
                validator_config.full_node_network_public_key
            } else {
                None
            };

        let validator_host = if let Some(ref host) = self.validator_host {
            host.clone()
        } else if let Some(ref validator_config) = validator_config {
            validator_config.validator_host.clone()
        } else {
            return Err(CliError::CommandArgumentError(
                "Must provide either --validator-config-file or --validator-host".to_string(),
            ));
        };

        let full_node_host = if let Some(ref host) = self.full_node_host {
            Some(host.clone())
        } else if let Some(ref validator_config) = validator_config {
            validator_config.full_node_host.clone()
        } else {
            None
        };

        Ok((
            consensus_public_key,
            validator_network_public_key,
            full_node_network_public_key,
            validator_host,
            full_node_host,
        ))
    }

    fn read_validator_config(&self) -> CliTypedResult<Option<ValidatorConfiguration>> {
        if let Some(ref file) = self.validator_config_file {
            Ok(from_yaml(
                &String::from_utf8(read_from_file(file)?).map_err(CliError::from)?,
            )?)
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl CliCommand<Transaction> for RegisterValidatorCandidate {
    fn command_name(&self) -> &'static str {
        "RegisterValidatorCandidate"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let (
            consensus_public_key,
            validator_network_public_key,
            full_node_network_public_key,
            validator_host,
            full_node_host,
        ) = self.process_inputs()?;
        let validator_network_addresses =
            vec![validator_host.as_network_address(validator_network_public_key)?];
        let full_node_network_addresses =
            match (full_node_host.as_ref(), full_node_network_public_key) {
                (Some(host), Some(public_key)) => vec![host.as_network_address(public_key)?],
                _ => vec![],
            };

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "register_validator_candidate",
                vec![],
                vec![
                    bcs::to_bytes(&consensus_public_key)?,
                    // Double BCS encode, so that we can hide the original type
                    bcs::to_bytes(&bcs::to_bytes(&validator_network_addresses)?)?,
                    bcs::to_bytes(&bcs::to_bytes(&full_node_network_addresses)?)?,
                ],
            )
            .await
    }
}

/// Arguments used for operator of the staking pool
#[derive(Parser)]
pub struct OperatorArgs {
    /// Address of the Staking pool
    #[clap(long)]
    pub(crate) pool_address: Option<AccountAddress>,
}

impl OperatorArgs {
    fn address(&self, profile_options: &ProfileOptions) -> CliTypedResult<AccountAddress> {
        if let Some(address) = self.pool_address {
            Ok(address)
        } else {
            profile_options.account_address()
        }
    }
}

/// Join the validator set after meeting staking requirements
#[derive(Parser)]
pub struct JoinValidatorSet {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) operator_args: OperatorArgs,
}

#[async_trait]
impl CliCommand<Transaction> for JoinValidatorSet {
    fn command_name(&self) -> &'static str {
        "JoinValidatorSet"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let address = self
            .operator_args
            .address(&self.txn_options.profile_options)?;

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "join_validator_set",
                vec![],
                vec![bcs::to_bytes(&address)?],
            )
            .await
    }
}

/// Leave the validator set
#[derive(Parser)]
pub struct LeaveValidatorSet {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) operator_args: OperatorArgs,
}

#[async_trait]
impl CliCommand<Transaction> for LeaveValidatorSet {
    fn command_name(&self) -> &'static str {
        "LeaveValidatorSet"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let address = self
            .operator_args
            .address(&self.txn_options.profile_options)?;

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "leave_validator_set",
                vec![],
                vec![bcs::to_bytes(&address)?],
            )
            .await
    }
}

/// Show validator details of the current validator
#[derive(Parser)]
pub struct ShowValidatorStake {
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) operator_args: OperatorArgs,
}

#[async_trait]
impl CliCommand<serde_json::Value> for ShowValidatorStake {
    fn command_name(&self) -> &'static str {
        "ShowValidatorStake"
    }

    async fn execute(mut self) -> CliTypedResult<serde_json::Value> {
        let client = self.rest_options.client(&self.profile_options.profile)?;
        let address = self.operator_args.address(&self.profile_options)?;
        let response = get_resource_migration(
            &client,
            address,
            "0x1::Stake::StakePool",
            "0x1::stake::StakePool",
        )
        .await?;
        Ok(response.into_inner())
    }
}

/// Show validator details of the current validator
#[derive(Parser)]
pub struct ShowValidatorConfig {
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) operator_args: OperatorArgs,
}

#[async_trait]
impl CliCommand<serde_json::Value> for ShowValidatorConfig {
    fn command_name(&self) -> &'static str {
        "ShowValidatorConfig"
    }

    async fn execute(mut self) -> CliTypedResult<serde_json::Value> {
        let client = self.rest_options.client(&self.profile_options.profile)?;
        let address = self.operator_args.address(&self.profile_options)?;
        let response = get_resource_migration(
            &client,
            address,
            "0x1::Stake::ValidatorConfig",
            "0x1::stake::ValidatorConfig",
        )
        .await?;
        Ok(response.into_inner())
    }
}

/// Show validator details of the validator set
#[derive(Parser)]
pub struct ShowValidatorSet {
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
}

#[async_trait]
impl CliCommand<serde_json::Value> for ShowValidatorSet {
    fn command_name(&self) -> &'static str {
        "ShowValidatorSet"
    }

    async fn execute(mut self) -> CliTypedResult<serde_json::Value> {
        let client = self.rest_options.client(&self.profile_options.profile)?;
        let response = get_resource_migration(
            &client,
            CORE_CODE_ADDRESS,
            "0x1::Stake::ValidatorSet",
            "0x1::stake::ValidatorSet",
        )
        .await?;
        Ok(response.into_inner())
    }
}

async fn get_resource_migration(
    client: &aptos_rest_client::Client,
    address: AccountAddress,
    original_resource: &'static str,
    new_resource: &'static str,
) -> CliTypedResult<Response<serde_json::Value>> {
    if let Ok(response) = client.get_resource(address, original_resource).await {
        Ok(response)
    } else {
        Ok(client.get_resource(address, new_resource).await?)
    }
}

const MAX_WAIT_S: u64 = 30;
const WAIT_INTERVAL_MS: u64 = 100;
const TESTNET_FOLDER: &str = "testnet";

/// Run local testnet
///
/// This local testnet will run it's own Genesis and run as a single node
/// network locally.  Optionally, a faucet can be added for minting coins.
#[derive(Parser)]
pub struct RunLocalTestnet {
    /// An overridable config template for the test node
    #[clap(long, parse(from_os_str))]
    config_path: Option<PathBuf>,
    /// The directory to save all files for the node
    #[clap(long, parse(from_os_str))]
    test_dir: Option<PathBuf>,
    /// Random seed for key generation in test mode
    #[clap(long, parse(try_from_str = FromHex::from_hex))]
    seed: Option<[u8; 32]>,
    /// Clean the state and start with a new chain at genesis
    #[clap(long)]
    force_restart: bool,
    #[clap(flatten)]
    prompt_options: PromptOptions,
    /// Run a faucet alongside the node
    #[clap(long)]
    with_faucet: bool,
    /// Port to run the faucet on
    #[clap(long, default_value = "8081")]
    faucet_port: u16,
}

#[async_trait]
impl CliCommand<()> for RunLocalTestnet {
    fn command_name(&self) -> &'static str {
        "RunLocalTestnet"
    }

    async fn execute(mut self) -> CliTypedResult<()> {
        let rng = self
            .seed
            .map(StdRng::from_seed)
            .unwrap_or_else(StdRng::from_entropy);

        let global_config = GlobalConfig::load()?;
        let test_dir = global_config.get_config_location()?.join(TESTNET_FOLDER);

        // Remove the current test directory and start with a new node
        if self.force_restart && test_dir.exists() {
            prompt_yes_with_override(
                "Are you sure you want to delete the existing chain?",
                self.prompt_options,
            )?;
            std::fs::remove_dir_all(test_dir.as_path()).map_err(|err| {
                CliError::IO(format!("Failed to delete {}", test_dir.display()), err)
            })?;
        }

        // Spawn the node in a separate thread
        let config_path = self.config_path.clone();
        let test_dir_copy = test_dir.clone();
        let _node = thread::spawn(move || {
            aptos_node::load_test_environment(
                config_path,
                Some(test_dir_copy),
                false,
                false,
                cached_framework_packages::module_blobs().to_vec(),
                rng,
            )
            .map_err(|err| CliError::UnexpectedError(format!("Node failed to run {}", err)))
        });

        // Run faucet if selected
        let _maybe_faucet = if self.with_faucet {
            let max_wait = Duration::from_secs(MAX_WAIT_S);
            let wait_interval = Duration::from_millis(WAIT_INTERVAL_MS);

            // Load the config to get the rest port
            let config_path = test_dir.join("0").join("node.yaml");

            // We have to wait for the node to be configured above in the other thread
            let mut config = None;
            let start = Instant::now();
            while start.elapsed() < max_wait {
                if let Ok(loaded_config) = NodeConfig::load(&config_path) {
                    config = Some(loaded_config);
                    break;
                }
                tokio::time::sleep(wait_interval).await;
            }

            // Retrieve the port from the local node
            let port = if let Some(config) = config {
                config.api.address.port()
            } else {
                return Err(CliError::UnexpectedError(
                    "Failed to find node configuration to start faucet".to_string(),
                ));
            };

            // Check that the REST API is ready
            let rest_url = Url::parse(&format!("http://localhost:{}", port)).map_err(|err| {
                CliError::UnexpectedError(format!("Failed to parse localhost URL {}", err))
            })?;
            let rest_client = aptos_rest_client::Client::new(rest_url.clone());
            let start = Instant::now();
            let mut started_successfully = false;

            while start.elapsed() < max_wait {
                if rest_client.get_index().await.is_ok() {
                    started_successfully = true;
                    break;
                }
                tokio::time::sleep(wait_interval).await
            }

            if !started_successfully {
                return Err(CliError::UnexpectedError(
                    "Failed to startup local node before faucet".to_string(),
                ));
            }

            // Start the faucet
            Some(
                FaucetArgs {
                    address: "0.0.0.0".to_string(),
                    port: self.faucet_port,
                    server_url: rest_url,
                    mint_key_file_path: test_dir.join("mint.key"),
                    mint_key: None,
                    mint_account_address: None,
                    chain_id: ChainId::test(),
                    maximum_amount: None,
                    do_not_delegate: false,
                }
                .run()
                .await,
            )
        } else {
            None
        };

        // Wait for an interrupt
        let term = Arc::new(AtomicBool::new(false));
        while !term.load(Ordering::Acquire) {
            std::thread::park();
        }
        Ok(())
    }
}
