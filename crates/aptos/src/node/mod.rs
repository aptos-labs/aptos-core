// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{
            load_account_arg, CliCommand, CliError, CliResult, CliTypedResult, EncodingType,
            ProfileOptions, RestOptions, TransactionOptions,
        },
        utils::read_from_file,
    },
    genesis::git::from_yaml,
};
use aptos_config::keys::ConfigKey;
use aptos_crypto::{bls12381, ed25519::Ed25519PrivateKey, x25519, ValidCryptoMaterialStringExt};
use aptos_faucet::{mint, mint::MintParams, Service};
use aptos_genesis::config::{HostAndPort, ValidatorConfiguration};
use aptos_rest_client::Transaction;
use aptos_sdk::types::LocalAccount;
use aptos_types::{
    account_address::AccountAddress, account_config::aptos_root_address, chain_id::ChainId,
};
use async_trait::async_trait;
use clap::Parser;
use hex::FromHex;
use rand::{rngs::StdRng, SeedableRng};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

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
        let response = client
            .get_resource(address, "0x1::Stake::StakePool")
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
        let response = client
            .get_resource(address, "0x1::Stake::ValidatorConfig")
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
        let response = client
            .get_resource(aptos_root_address(), "0x1::Stake::ValidatorSet")
            .await?;
        Ok(response.into_inner())
    }
}

/// Tool for running a local testnet
///
#[derive(Parser)]
pub enum LocalTestnetTool {
    Node(RunTestNode),
    MintCoins(MintCoins),
}

impl LocalTestnetTool {
    pub async fn execute(self) -> CliResult {
        use LocalTestnetTool::*;
        match self {
            Node(tool) => tool.execute_serialized().await,
            MintCoins(tool) => tool.execute_serialized().await,
        }
    }
}

/// Mint coins to a set of accounts
#[derive(Parser)]
pub struct MintCoins {
    /// Rest API server URL
    #[clap(long, default_value = "http://localhost:8080")]
    pub server_url: String,
    /// Path to the private key for creating test account and minting coins.
    /// To keep Testnet simple, we used one private key for aptos root account
    /// To manually generate a keypair, use generate-key:
    /// `cargo run -p generate-keypair -- -o <output_file_path>`
    #[clap(long, default_value = "/opt/aptos/mint.key")]
    pub mint_key_file_path: String,
    /// Ed25519PrivateKey for minting coins
    #[clap(long, parse(try_from_str = ConfigKey::from_encoded_string))]
    pub mint_key: Option<ConfigKey<Ed25519PrivateKey>>,
    /// Address of the account to send transactions from.
    /// On Testnet, for example, this is a550c18.
    /// If not present, the mint key's address is used
    #[clap(long, parse(try_from_str = AccountAddress::from_hex_literal))]
    pub mint_account_address: Option<AccountAddress>,
    /// Chain ID of the network this client is connecting to.
    /// For mainnet: "MAINNET" or 1, testnet: "TESTNET" or 2, devnet: "DEVNET" or 3,
    /// local swarm: "TESTING" or 4
    /// Note: Chain ID of 0 is not allowed; Use number if chain id is not predefined.
    #[clap(long, default_value = "TESTING")]
    pub chain_id: ChainId,
    /// Amount of coins to mint
    #[clap(long)]
    pub amount: u64,
    /// Addresses of accounts to mint coins to, split by commas e.g. 0x1337,0x2,3
    #[clap(long, group = "account-group")]
    pub accounts: Option<String>,
    /// File of addresses of account to mint coins to.  Formatted in YAML
    #[clap(long, group = "account-group", parse(from_os_str))]
    pub account_file: Option<PathBuf>,
}

#[async_trait]
impl CliCommand<String> for MintCoins {
    fn command_name(&self) -> &'static str {
        "MintCoins"
    }

    async fn execute(mut self) -> CliTypedResult<String> {
        let mint_account_address = self.mint_account_address.unwrap_or_else(aptos_root_address);
        let mint_key = if let Some(ref key) = self.mint_key {
            key.private_key()
        } else {
            EncodingType::BCS
                .load_key::<Ed25519PrivateKey>("mint key", Path::new(&self.mint_key_file_path))
                .unwrap()
        };
        let faucet_account = LocalAccount::new(mint_account_address, mint_key, 0);
        let service = Service::new(self.server_url, self.chain_id, faucet_account, None);

        let accounts: HashSet<AccountAddress> = if let Some(accounts) = self.accounts {
            accounts
                .trim()
                .split(',')
                .map(|str| load_account_arg(str).unwrap())
                .collect()
        } else if let Some(path) = self.account_file {
            let strings: Vec<String> =
                serde_yaml::from_str(&std::fs::read_to_string(path.as_path()).unwrap()).unwrap();
            strings
                .into_iter()
                .map(|str| load_account_arg(&str).unwrap())
                .collect()
        } else {
            panic!("Either --accounts or --account-file must be specified");
        };

        let mut successes = vec![];
        let mut failures = vec![];

        // Iterate through accounts to mint the tokens
        for account in accounts {
            let response = mint::process(
                &service,
                MintParams {
                    amount: self.amount,
                    auth_key: None,
                    address: Some(account.to_hex_literal()),
                    pub_key: None,
                    return_txns: None,
                },
            )
            .await;
            match response {
                Ok(_) => successes.push(account),
                Err(response) => {
                    println!(
                        "FAILURE: Account: {} Response: {:?}",
                        account.to_hex_literal(),
                        response
                    );
                    failures.push(account);
                }
            }
        }

        Ok(format!(
            "Successes: {:?} Failures: {:?}",
            successes, failures
        ))
    }
}

/// Show validator details of the validator set
#[derive(Parser)]
pub struct RunTestNode {
    /// An overridable config for the test node
    #[clap(long, parse(from_os_str))]
    config_path: Option<PathBuf>,
    /// The directory to save all files for the node
    #[clap(long, parse(from_os_str), default_value = "/opt/aptos")]
    node_dir: PathBuf,
    /// Randomize ports rather than using defaults of 8080
    #[clap(long)]
    random_ports: bool,
    /// Random seed for key generation in test mode
    #[clap(
    long,
    parse(try_from_str = FromHex::from_hex)
    )]
    seed: Option<[u8; 32]>,
}

#[async_trait]
impl CliCommand<()> for RunTestNode {
    fn command_name(&self) -> &'static str {
        "RunTestNode"
    }

    async fn execute(mut self) -> CliTypedResult<()> {
        let rng = self
            .seed
            .map(StdRng::from_seed)
            .unwrap_or_else(StdRng::from_entropy);

        aptos_node::load_test_environment(
            self.config_path,
            self.node_dir,
            self.random_ports,
            false,
            cached_framework_packages::module_blobs().to_vec(),
            rng,
        );

        Ok(())
    }
}
