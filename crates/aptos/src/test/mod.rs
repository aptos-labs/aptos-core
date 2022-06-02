// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::{
        create::{CreateAccount, DEFAULT_FUNDED_COINS},
        fund::FundAccount,
        list::{ListAccount, ListQuery},
        transfer::{TransferCoins, TransferSummary},
    },
    common::{
        init::{InitTool, DEFAULT_FAUCET_URL, DEFAULT_REST_URL},
        types::{
            CliConfig, CliTypedResult, EncodingOptions, PrivateKeyInputOptions, ProfileOptions,
            PromptOptions,
        },
    },
    op::key::GenerateKey,
    CliCommand,
};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::move_types::account_address::AccountAddress;
use reqwest::Url;
use serde_json::Value;
use std::str::FromStr;

/// A framework for testing the CLI
pub struct CliTestFramework {
    endpoint: Url,
    faucet_endpoint: Url,
}

impl CliTestFramework {
    pub async fn new(endpoint: Url, faucet_endpoint: Url, num_accounts: usize) -> CliTestFramework {
        let framework = CliTestFramework {
            endpoint,
            faucet_endpoint,
        };

        // TODO: Make this allow a passed in random seed
        for i in 0..num_accounts {
            let private_key = GenerateKey::generate_ed25519_in_memory();

            // For now use, the config files to handle accounts
            framework
                .init(i, &private_key)
                .await
                .expect("Expected init command to succeed");
        }

        framework
    }

    pub async fn create_account(&self, index: usize) -> CliTypedResult<String> {
        CreateAccount {
            encoding_options: Default::default(),
            write_options: Default::default(),
            profile_options: profile(index),
            account: account_id(index),
            use_faucet: true,
            faucet_options: Default::default(),
            initial_coins: 0,
        }
        .execute()
        .await
    }

    pub async fn fund_account(&self, index: usize) -> CliTypedResult<String> {
        FundAccount {
            profile_options: profile(index),
            account: account_id(index),
            faucet_options: Default::default(),
            num_coins: DEFAULT_FUNDED_COINS,
        }
        .execute()
        .await
    }

    pub async fn list_account(&self, index: usize, query: ListQuery) -> CliTypedResult<Vec<Value>> {
        ListAccount {
            rest_options: Default::default(),
            profile_options: profile(index),
            account: Some(account_id(index)),
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
        let receiver_account = account_id(receiver_index);

        TransferCoins {
            write_options: Default::default(),
            encoding_options: Default::default(),
            profile_options: profile(sender_index),
            account: receiver_account,
            amount,
        }
        .execute()
        .await
    }

    pub async fn init(&self, index: usize, private_key: &Ed25519PrivateKey) -> CliTypedResult<()> {
        InitTool {
            rest_url: Some(self.endpoint.clone()),
            faucet_url: Some(self.faucet_endpoint.clone()),
            private_key_options: private_key_options(private_key),
            profile_options: profile(index),
            prompt_options: PromptOptions::yes(),
            encoding_options: EncodingOptions::default(),
        }
        .execute()
        .await
    }

    pub async fn account_balance(&self, index: usize) -> u64 {
        u64::from_str(
            self.list_account(index, ListQuery::Balance)
                .await
                .unwrap()
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
        .unwrap()
    }
}

fn account_id(index: usize) -> AccountAddress {
    let profile = CliConfig::load_profile(&index.to_string())
        .expect("Must select account in bounds")
        .expect("Expected to already be initialized");
    profile.account.expect("Expected to have account address")
}

fn profile(index: usize) -> ProfileOptions {
    ProfileOptions {
        profile: index.to_string(),
    }
}

fn private_key_options(private_key: &Ed25519PrivateKey) -> PrivateKeyInputOptions {
    PrivateKeyInputOptions::from_private_key(private_key)
        .expect("Must serialize private key to hex")
}

// Ignore test because we don't want to be dependent on testnet
#[tokio::test]
#[ignore]
async fn test_flow() {
    let framework = CliTestFramework::new(
        DEFAULT_REST_URL.parse().unwrap(),
        DEFAULT_FAUCET_URL.parse().unwrap(),
        2,
    )
    .await;

    assert_eq!(DEFAULT_FUNDED_COINS, framework.account_balance(0).await);
    assert_eq!(DEFAULT_FUNDED_COINS, framework.account_balance(1).await);

    let transfer_amount = 100;

    let response = framework
        .transfer_coins(0, 1, transfer_amount)
        .await
        .unwrap();
    let expected_sender_amount =
        DEFAULT_FUNDED_COINS - response.gas_used.unwrap() - transfer_amount;
    let expected_receiver_amount = DEFAULT_FUNDED_COINS + transfer_amount;

    assert_eq!(expected_sender_amount, framework.account_balance(0).await);
    assert_eq!(expected_receiver_amount, framework.account_balance(1).await);
}
