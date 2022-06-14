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
        init::InitTool,
        types::{
            CliConfig, CliTypedResult, EncodingOptions, PrivateKeyInputOptions, ProfileOptions,
            PromptOptions, RestOptions, WriteTransactionOptions,
        },
    },
    CliCommand,
};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_keygen::KeyGen;
use aptos_sdk::move_types::account_address::AccountAddress;
use reqwest::Url;
use serde_json::Value;
use std::{str::FromStr, time::Duration};
use tokio::time::{sleep, Instant};

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
        let mut keygen = KeyGen::from_seed([0; 32]);

        // TODO: Make this allow a passed in random seed
        for i in 0..num_accounts {
            let private_key = keygen.generate_ed25519_private_key();

            // For now use, the config files to handle accounts
            framework
                .init(i, &private_key)
                .await
                .expect("Expected init command to succeed");
        }

        framework
    }

    pub async fn create_account(
        &self,
        index: usize,
        mint_key: &Ed25519PrivateKey,
    ) -> CliTypedResult<String> {
        CreateAccount {
            encoding_options: Default::default(),
            write_options: WriteTransactionOptions {
                private_key_options: PrivateKeyInputOptions::from_private_key(mint_key)?,
                rest_options: RestOptions::new(Some(self.endpoint.clone())),
                max_gas: 1000,
            },
            profile_options: profile(index),
            account: Self::account_id(index),
            use_faucet: false,
            faucet_options: Default::default(),
            initial_coins: DEFAULT_FUNDED_COINS,
        }
        .execute()
        .await
    }

    pub async fn create_account_with_faucet(&self, index: usize) -> CliTypedResult<String> {
        CreateAccount {
            encoding_options: Default::default(),
            write_options: Default::default(),
            profile_options: profile(index),
            account: Self::account_id(index),
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
            account: Self::account_id(index),
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
            account: Some(Self::account_id(index)),
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
        let receiver_account = Self::account_id(receiver_index);

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

    pub fn account_id(index: usize) -> AccountAddress {
        let profile = CliConfig::load_profile(&index.to_string())
            .expect("Must select account in bounds")
            .expect("Expected to already be initialized");
        profile.account.expect("Expected to have account address")
    }
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
