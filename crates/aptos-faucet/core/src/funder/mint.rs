// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{FunderHealthMessage, FunderTrait};
use crate::endpoints::{AptosTapError, AptosTapErrorCode};
use anyhow::{Context, Result};
use aptos_logger::info;
use aptos_sdk::{
    crypto::ed25519::Ed25519PublicKey,
    rest_client::{AptosBaseUrl, Client},
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{
            authenticator::AuthenticationKey, Script, SignedTransaction, TransactionArgument,
        },
        LocalAccount,
    },
};
use async_trait::async_trait;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

static MINTER_SCRIPT: &[u8] = include_bytes!(
    "../../../../../aptos-move/move-examples/scripts/minter/build/Minter/bytecode_scripts/main.mv"
);

use super::common::{
    submit_transaction, update_sequence_numbers, ApiConnectionConfig, GasUnitPriceManager,
    TransactionSubmissionConfig,
};

/// explain these contain additional args for the mint funder.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MintFunderConfig {
    #[serde(flatten)]
    pub api_connection_config: ApiConnectionConfig,

    #[serde(flatten)]
    pub transaction_submission_config: TransactionSubmissionConfig,

    /// Address of the account to send transactions from. On testnet, for
    /// example, this is a550c18. If not given, we use the account address
    /// corresponding to the given private key.
    pub mint_account_address: Option<AccountAddress>,

    /// Just use the account given in funder args, don't make a new one and
    /// delegate the mint capability to it.
    pub do_not_delegate: bool,
}

impl MintFunderConfig {
    pub async fn build_funder(self) -> Result<MintFunder> {
        let key = self.api_connection_config.get_key()?;

        let faucet_account = LocalAccount::new(
            self.mint_account_address.unwrap_or_else(|| {
                AuthenticationKey::ed25519(&Ed25519PublicKey::from(&key)).account_address()
            }),
            key,
            0,
        );

        let mut minter = MintFunder::new(
            self.api_connection_config.node_url.clone(),
            self.api_connection_config.api_key.clone(),
            self.api_connection_config.additional_headers.clone(),
            self.api_connection_config.chain_id,
            self.transaction_submission_config,
            faucet_account,
        );

        if !self.do_not_delegate {
            minter
                .use_delegated_account()
                .await
                .context("Failed to make MintFunder use delegated account")?;
        }

        Ok(minter)
    }
}

pub struct MintFunder {
    /// URL of an Aptos node API.
    node_url: Url,
    node_api_key: Option<String>,
    node_additional_headers: Option<HashMap<String, String>>,

    txn_config: TransactionSubmissionConfig,

    faucet_account: RwLock<LocalAccount>,

    transaction_factory: TransactionFactory,

    gas_unit_price_manager: GasUnitPriceManager,

    /// When recovering from being overloaded, this struct ensures we handle
    /// requests in the order they came in.
    outstanding_requests: RwLock<Vec<(AccountAddress, u64)>>,
}

impl MintFunder {
    pub fn new(
        node_url: Url,
        node_api_key: Option<String>,
        node_additional_headers: Option<HashMap<String, String>>,
        chain_id: ChainId,
        txn_config: TransactionSubmissionConfig,
        faucet_account: LocalAccount,
    ) -> Self {
        let gas_unit_price_manager =
            GasUnitPriceManager::new(node_url.clone(), txn_config.get_gas_unit_price_ttl_secs());
        let transaction_factory = TransactionFactory::new(chain_id)
            .with_max_gas_amount(txn_config.max_gas_amount)
            .with_transaction_expiration_time(txn_config.transaction_expiration_secs);
        Self {
            node_url,
            node_api_key,
            node_additional_headers,
            txn_config,
            faucet_account: RwLock::new(faucet_account),
            transaction_factory,
            gas_unit_price_manager,
            outstanding_requests: RwLock::new(vec![]),
        }
    }

    async fn get_gas_unit_price(&self) -> Result<u64, AptosTapError> {
        match self.txn_config.gas_unit_price_override {
            Some(gas_unit_price) => Ok(gas_unit_price),
            None => self
                .gas_unit_price_manager
                .get_gas_unit_price()
                .await
                .map_err(|e| {
                    AptosTapError::new_with_error_code(e, AptosTapErrorCode::AptosApiError)
                }),
        }
    }

    async fn get_transaction_factory(&self) -> Result<TransactionFactory, AptosTapError> {
        Ok(self
            .transaction_factory
            .clone()
            .with_gas_unit_price(self.get_gas_unit_price().await?))
    }

    /// todo explain / rename
    pub async fn use_delegated_account(&mut self) -> Result<()> {
        // Build a client.
        let client = self.get_api_client();

        // Create a new random account, then delegate to it
        let delegated_account = LocalAccount::generate(&mut rand::rngs::OsRng);

        // Create the account, wait for the response.
        self.process(
            &client,
            100_000_000_000,
            delegated_account
                .authentication_key()
                .clone()
                .account_address(),
            false,
            true,
        )
        .await
        .context("Failed to create new account")?;

        // Build a transaction factory using the gas unit price from the
        // GasUnitPriceManager. This mostly ensures that we will build a
        // transaction with a gas unit price that will be accepted.
        let transaction_factory = self.get_transaction_factory().await?;

        // Delegate minting to the account
        {
            let faucet_account = self.faucet_account.write().await;
            client
                .submit_and_wait(&faucet_account.sign_with_transaction_builder(
                    transaction_factory.payload(aptos_stdlib::aptos_coin_delegate_mint_capability(
                        delegated_account.address(),
                    )),
                ))
                .await
                .context("Failed to delegate minting to the new account")?;
        }

        // Claim the capability!
        client
            .submit_and_wait(&delegated_account.sign_with_transaction_builder(
                transaction_factory.payload(aptos_stdlib::aptos_coin_claim_mint_capability()),
            ))
            .await
            .context("Failed to claim the minting capability")?;

        info!(
            "Successfully configured MintFunder to use delegated account: {}",
            delegated_account.address()
        );

        self.faucet_account = RwLock::new(delegated_account);

        Ok(())
    }

    /// Within a single request we should just call this once and use this client
    /// the entire time because it uses cookies, ensuring we're talking to the same
    /// node behind the LB every time.
    pub fn get_api_client(&self) -> Client {
        let mut builder = Client::builder(AptosBaseUrl::Custom(self.node_url.clone()));

        if let Some(api_key) = self.node_api_key.clone() {
            builder = builder.api_key(&api_key).expect("Failed to set API key");
        }

        if let Some(additional_headers) = &self.node_additional_headers {
            for (key, value) in additional_headers {
                builder = builder.header(key, value).expect("Failed to set header");
            }
        }

        builder.build()
    }

    pub async fn process(
        &self,
        client: &Client,
        amount: u64,
        receiver_address: AccountAddress,
        check_only: bool,
        wait_for_transactions: bool,
    ) -> Result<Vec<SignedTransaction>, AptosTapError> {
        let (_faucet_seq, receiver_seq) = update_sequence_numbers(
            client,
            &self.faucet_account,
            &self.outstanding_requests,
            receiver_address,
            amount,
            self.txn_config.wait_for_outstanding_txns_secs,
        )
        .await?;

        if receiver_seq.is_some() && amount == 0 {
            return Err(AptosTapError::new(
                format!(
                    "Account {} already exists and amount asked for is 0",
                    receiver_address
                ),
                AptosTapErrorCode::InvalidRequest,
            ));
        }

        if check_only {
            return Ok(vec![]);
        }

        let txn =
            {
                let faucet_account = self.faucet_account.write().await;
                let transaction_factory = self.get_transaction_factory().await?;
                faucet_account.sign_with_transaction_builder(transaction_factory.script(
                    Script::new(MINTER_SCRIPT.to_vec(), vec![], vec![
                        TransactionArgument::Address(receiver_address),
                        TransactionArgument::U64(amount),
                    ]),
                ))
            };

        Ok(vec![
            submit_transaction(
                client,
                &self.faucet_account,
                txn,
                &receiver_address,
                wait_for_transactions,
            )
            .await?,
        ])
    }
}

#[async_trait]
impl FunderTrait for MintFunder {
    async fn fund(
        &self,
        amount: Option<u64>,
        receiver_address: AccountAddress,
        check_only: bool,
        did_bypass_checkers: bool,
    ) -> Result<Vec<SignedTransaction>, AptosTapError> {
        let client = self.get_api_client();
        let amount = self.get_amount(amount, did_bypass_checkers);
        self.process(
            &client,
            amount,
            receiver_address,
            check_only,
            self.txn_config.wait_for_transactions,
        )
        .await
    }

    fn get_amount(&self, amount: Option<u64>, did_bypass_checkers: bool) -> u64 {
        match (
            amount,
            self.txn_config.get_maximum_amount(did_bypass_checkers),
        ) {
            (Some(amount), Some(maximum_amount)) => std::cmp::min(amount, maximum_amount),
            (Some(amount), None) => amount,
            (None, Some(maximum_amount)) => maximum_amount,
            (None, None) => 0,
        }
    }

    /// Assert the funder account actually exists.
    async fn is_healthy(&self) -> FunderHealthMessage {
        let account_address = self.faucet_account.read().await.address();
        let client = self.get_api_client();
        match client.get_account_bcs(account_address).await {
            Ok(_) => FunderHealthMessage {
                can_process_requests: true,
                message: None,
            },
            Err(e) => return FunderHealthMessage {
                can_process_requests: false,
                message: Some(format!(
                    "Failed to read account information for {}, it may not exist or the fullnode might not be fully synced: {:#}",
                    account_address, e
                )),
            },
        }
    }
}
