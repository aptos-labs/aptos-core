// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::init::InitTool;
use crate::common::types::{
    account_address_from_public_key, EncodingOptions, FaucetOptions, PromptOptions, RngArgs,
};
use crate::node::{
    AddStake, IncreaseLockup, JoinValidatorSet, LeaveValidatorSet, OperatorArgs,
    RegisterValidatorCandidate, ShowValidatorConfig, ShowValidatorSet, ShowValidatorStake,
    UnlockStake, UpdateValidatorNetworkAddresses, ValidatorConfigArgs, WithdrawStake,
};
use crate::{
    account::{
        create::{CreateAccount, DEFAULT_FUNDED_COINS},
        fund::FundAccount,
        list::{ListAccount, ListQuery},
        transfer::{TransferCoins, TransferSummary},
    },
    common::types::{CliTypedResult, PrivateKeyInputOptions, RestOptions, TransactionOptions},
    CliCommand,
};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{bls12381, x25519, PrivateKey};
use aptos_genesis::config::HostAndPort;
use aptos_keygen::KeyGen;
use aptos_rest_client::Transaction;
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_types::validator_info::ValidatorInfo;
use aptos_types::{on_chain_config::ConsensusScheme, validator_config::ValidatorConfig};
use reqwest::Url;
use serde_json::Value;
use std::{str::FromStr, time::Duration};
use tokio::time::{sleep, Instant};

/// A framework for testing the CLI
pub struct CliTestFramework {
    account_keys: Vec<Ed25519PrivateKey>,
    endpoint: Url,
    faucet_endpoint: Url,
}

impl CliTestFramework {
    pub async fn new(endpoint: Url, faucet_endpoint: Url, num_accounts: usize) -> CliTestFramework {
        let mut framework = CliTestFramework {
            account_keys: Vec::new(),
            endpoint,
            faucet_endpoint,
        };
        let mut keygen = KeyGen::from_seed([9; 32]);

        for i in 0..num_accounts {
            // Can't use the init command because the config causes flakiness multithreaded
            let private_key = keygen.generate_ed25519_private_key();
            framework.add_private_key(private_key);
            framework
                .create_account_with_faucet(i)
                .await
                .expect("Should create account");
        }

        framework
    }

    pub fn add_private_key(&mut self, private_key: Ed25519PrivateKey) {
        self.account_keys.push(private_key);
    }

    pub async fn create_account(
        &self,
        index: usize,
        mint_key: &Ed25519PrivateKey,
    ) -> CliTypedResult<String> {
        CreateAccount {
            txn_options: TransactionOptions {
                private_key_options: PrivateKeyInputOptions::from_private_key(mint_key)?,
                encoding_options: Default::default(),
                profile_options: Default::default(),
                rest_options: self.rest_options(),
                gas_options: Default::default(),
            },
            account: self.account_id(index),
            use_faucet: false,
            faucet_options: Default::default(),
            initial_coins: DEFAULT_FUNDED_COINS,
        }
        .execute()
        .await
    }

    pub async fn create_account_with_faucet(&self, index: usize) -> CliTypedResult<String> {
        CreateAccount {
            txn_options: Default::default(),
            account: self.account_id(index),
            use_faucet: true,
            faucet_options: self.faucet_options(),
            initial_coins: DEFAULT_FUNDED_COINS,
        }
        .execute()
        .await
    }

    pub async fn fund_account(&self, index: usize) -> CliTypedResult<String> {
        FundAccount {
            profile_options: Default::default(),
            account: self.account_id(index),
            faucet_options: self.faucet_options(),
            num_coins: DEFAULT_FUNDED_COINS,
        }
        .execute()
        .await
    }

    pub async fn list_account(&self, index: usize, query: ListQuery) -> CliTypedResult<Vec<Value>> {
        ListAccount {
            rest_options: self.rest_options(),
            profile_options: Default::default(),
            account: Some(self.account_id(index)),
            query,
        }
        .execute()
        .await
    }

    pub async fn transfer_coins(
        &self,
        sender_index: usize,
        receiver_index: usize,
        amount: u64,
    ) -> CliTypedResult<TransferSummary> {
        let receiver_account = self.account_id(receiver_index);

        TransferCoins {
            txn_options: self.transaction_options(sender_index),
            account: receiver_account,
            amount,
        }
        .execute()
        .await
    }

    pub async fn show_validator_config(&self) -> CliTypedResult<ValidatorConfig> {
        ShowValidatorConfig {
            rest_options: self.rest_options(),
            profile_options: Default::default(),
            operator_args: OperatorArgs { pool_address: None },
        }
        .execute()
        .await
        .map(|v| to_validator_config(&v))
    }

    pub async fn show_validator_set(&self) -> CliTypedResult<ValidatorSet> {
        ShowValidatorSet {
            rest_options: self.rest_options(),
            profile_options: Default::default(),
        }
        .execute()
        .await
        .map(|v| to_validator_set(&v))
    }

    pub async fn show_validator_stake(&self) -> CliTypedResult<Value> {
        ShowValidatorStake {
            rest_options: self.rest_options(),
            profile_options: Default::default(),
            operator_args: OperatorArgs { pool_address: None },
        }
        .execute()
        .await
    }

    pub async fn register_validator_candidate(
        &self,
        index: usize,
        consensus_public_key: bls12381::PublicKey,
        proof_of_possession: bls12381::ProofOfPossession,
        validator_host: HostAndPort,
        validator_network_public_key: x25519::PublicKey,
    ) -> CliTypedResult<Transaction> {
        RegisterValidatorCandidate {
            txn_options: self.transaction_options(index),
            validator_config_args: ValidatorConfigArgs {
                validator_config_file: None,
                consensus_public_key: Some(consensus_public_key),
                proof_of_possession: Some(proof_of_possession),
                validator_host: Some(validator_host),
                validator_network_public_key: Some(validator_network_public_key),
                full_node_host: None,
                full_node_network_public_key: None,
            },
        }
        .execute()
        .await
    }

    pub async fn add_stake(&self, index: usize, amount: u64) -> CliTypedResult<Transaction> {
        AddStake {
            txn_options: self.transaction_options(index),
            amount,
        }
        .execute()
        .await
    }

    pub async fn unlock_stake(&self, index: usize, amount: u64) -> CliTypedResult<Transaction> {
        UnlockStake {
            txn_options: self.transaction_options(index),
            amount,
        }
        .execute()
        .await
    }

    pub async fn withdraw_stake(&self, index: usize) -> CliTypedResult<Transaction> {
        WithdrawStake {
            node_op_options: self.transaction_options(index),
        }
        .execute()
        .await
    }

    pub async fn increase_lockup(
        &self,
        index: usize,
        lockup_duration: Duration,
    ) -> CliTypedResult<Transaction> {
        IncreaseLockup {
            txn_options: self.transaction_options(index),
            lockup_duration,
        }
        .execute()
        .await
    }

    pub async fn join_validator_set(&self, index: usize) -> CliTypedResult<Transaction> {
        JoinValidatorSet {
            txn_options: self.transaction_options(index),
            operator_args: OperatorArgs { pool_address: None },
        }
        .execute()
        .await
    }

    pub async fn leave_validator_set(&self, index: usize) -> CliTypedResult<Transaction> {
        LeaveValidatorSet {
            txn_options: self.transaction_options(index),
            operator_args: OperatorArgs { pool_address: None },
        }
        .execute()
        .await
    }

    pub async fn update_validator_network_addresses(
        &self,
        index: usize,
        validator_host: HostAndPort,
        validator_network_public_key: x25519::PublicKey,
    ) -> CliTypedResult<Transaction> {
        UpdateValidatorNetworkAddresses {
            txn_options: self.transaction_options(index),
            operator_args: OperatorArgs { pool_address: None },
            validator_config_args: ValidatorConfigArgs {
                validator_config_file: None,
                consensus_public_key: None,
                proof_of_possession: None,
                validator_host: Some(validator_host),
                validator_network_public_key: Some(validator_network_public_key),
                full_node_host: None,
                full_node_network_public_key: None,
            },
        }
        .execute()
        .await
    }

    pub async fn init(&self, private_key: &Ed25519PrivateKey) -> CliTypedResult<()> {
        InitTool {
            rest_url: Some(self.endpoint.clone()),
            faucet_url: Some(self.faucet_endpoint.clone()),
            rng_args: RngArgs::from_seed([0; 32]),
            private_key_options: PrivateKeyInputOptions::from_private_key(private_key)?,
            profile_options: Default::default(),
            prompt_options: PromptOptions::yes(),
            encoding_options: EncodingOptions::default(),
            skip_faucet: false,
        }
        .execute()
        .await
    }

    /// Wait for an account to exist
    pub async fn wait_for_account(&self, index: usize) -> CliTypedResult<Vec<Value>> {
        let mut result = self.list_account(index, ListQuery::Balance).await;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            match result {
                Ok(_) => return result,
                _ => {
                    sleep(Duration::from_millis(500)).await;
                    result = self.list_account(index, ListQuery::Balance).await;
                }
            };
        }

        result
    }

    pub async fn account_balance(&self, index: usize) -> CliTypedResult<u64> {
        Ok(u64::from_str(
            self.wait_for_account(index)
                .await?
                .get(0)
                .unwrap()
                .as_object()
                .unwrap()
                .get("coin")
                .unwrap()
                .as_object()
                .unwrap()
                .get("value")
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap())
    }

    pub async fn wait_for_balance(
        &self,
        index: usize,
        expected_balance: u64,
    ) -> CliTypedResult<u64> {
        let mut result = self.account_balance(index).await;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            if let Ok(balance) = result {
                if balance == expected_balance {
                    return result;
                }
            }

            sleep(Duration::from_millis(500)).await;
            result = self.account_balance(index).await;
        }

        result
    }

    pub fn rest_options(&self) -> RestOptions {
        RestOptions::new(Some(self.endpoint.clone()))
    }

    pub fn faucet_options(&self) -> FaucetOptions {
        FaucetOptions::new(Some(self.faucet_endpoint.clone()))
    }

    fn transaction_options(&self, index: usize) -> TransactionOptions {
        TransactionOptions {
            private_key_options: PrivateKeyInputOptions::from_private_key(self.private_key(index))
                .unwrap(),
            rest_options: self.rest_options(),
            ..Default::default()
        }
    }

    pub fn private_key(&self, index: usize) -> &Ed25519PrivateKey {
        self.account_keys.get(index).unwrap()
    }

    pub fn account_id(&self, index: usize) -> AccountAddress {
        let private_key = self.private_key(index);
        account_address_from_public_key(&private_key.public_key())
    }
}

// ValidatorConfig/ValidatorSet doesn't match Move ValidatorSet struct,
// and json is serialized with different types from both, so hardcoding deserialization.

fn str_to_vec(value: &serde_json::Value) -> Vec<u8> {
    let str = value.as_str().unwrap();
    (&*hex::decode(&str[2..str.len()]).unwrap()).to_vec()
}

fn to_validator_config(value: &serde_json::Value) -> ValidatorConfig {
    ValidatorConfig {
        consensus_public_key: serde_json::from_value(
            value.get("consensus_pubkey").unwrap().clone(),
        )
        .unwrap(),
        validator_network_addresses: str_to_vec(value.get("network_addresses").unwrap()),
        fullnode_network_addresses: str_to_vec(value.get("fullnode_addresses").unwrap()),
        validator_index: u64::from_str(value.get("validator_index").unwrap().as_str().unwrap())
            .unwrap(),
    }
}

fn to_validator_info_vec(value: &serde_json::Value) -> Vec<ValidatorInfo> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|value| {
            let account_addr =
                AccountAddress::from_hex_literal(value.get("addr").unwrap().as_str().unwrap())
                    .unwrap();
            ValidatorInfo::new(
                account_addr,
                u64::from_str(value.get("voting_power").unwrap().as_str().unwrap()).unwrap(),
                to_validator_config(value.get("config").unwrap()),
            )
        })
        .collect()
}

// Original ValidatorSet has private fields, to make sure invariants are kept,
// so creating a new one for testing
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorSet {
    pub consensus_scheme: ConsensusScheme,
    pub active_validators: Vec<ValidatorInfo>,
    pub pending_inactive: Vec<ValidatorInfo>,
    pub pending_active: Vec<ValidatorInfo>,
}

fn to_validator_set(value: &serde_json::Value) -> ValidatorSet {
    ValidatorSet {
        consensus_scheme: match value.get("consensus_scheme").unwrap().as_u64().unwrap() {
            0u64 => ConsensusScheme::Ed25519,
            _ => panic!(),
        },
        active_validators: to_validator_info_vec(value.get("active_validators").unwrap()),
        pending_inactive: to_validator_info_vec(value.get("pending_inactive").unwrap()),
        pending_active: to_validator_info_vec(value.get("pending_active").unwrap()),
    }
}
