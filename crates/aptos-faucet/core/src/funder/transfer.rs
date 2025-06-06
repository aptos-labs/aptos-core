// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    common::{
        submit_transaction, ApiConnectionConfig, GasUnitPriceManager, TransactionSubmissionConfig,
    },
    FunderHealthMessage, FunderTrait,
};
use crate::{
    endpoints::{AptosTapError, AptosTapErrorCode, RejectionReason, RejectionReasonCode},
    funder::common::update_sequence_numbers,
    middleware::TRANSFER_FUNDER_ACCOUNT_BALANCE,
};
use anyhow::{Context, Result};
use aptos_logger::info;
use aptos_sdk::{
    crypto::{ed25519::Ed25519PrivateKey, PrivateKey},
    rest_client::{AptosBaseUrl, Client},
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{authenticator::AuthenticationKey, SignedTransaction, TransactionPayload},
        LocalAccount,
    },
};
use async_trait::async_trait;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, time::Duration};
use tokio::sync::RwLock;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransferFunderConfig {
    #[serde(flatten)]
    pub api_connection_config: ApiConnectionConfig,

    #[serde(flatten)]
    pub transaction_submission_config: TransactionSubmissionConfig,

    /// The minimum amount of coins the funder account should have. If it
    /// doesn't have this many, or if it gets to this point, the funder will
    /// intentionally fail to build, resulting in a failure on startup.
    pub minimum_funds: MinimumFunds,

    /// The amount of coins to fund the receiver account.
    pub amount_to_fund: AmountToFund,
}

impl TransferFunderConfig {
    pub async fn build_funder(&self) -> Result<TransferFunder> {
        // Read in private key.
        let key = self.api_connection_config.get_key()?;

        // Build account address from private key.
        let account_address = account_address_from_private_key(&key);

        // Build local representation of account.
        let faucet_account = LocalAccount::new(account_address, key, 0);

        let funder = TransferFunder::new(
            faucet_account,
            self.api_connection_config.chain_id,
            self.api_connection_config.node_url.clone(),
            self.api_connection_config.api_key.clone(),
            self.api_connection_config.additional_headers.clone(),
            self.minimum_funds,
            self.amount_to_fund,
            self.transaction_submission_config
                .get_gas_unit_price_ttl_secs(),
            self.transaction_submission_config.gas_unit_price_override,
            self.transaction_submission_config.max_gas_amount,
            self.transaction_submission_config
                .transaction_expiration_secs,
            self.transaction_submission_config
                .wait_for_outstanding_txns_secs,
            self.transaction_submission_config.wait_for_transactions,
        );

        Ok(funder)
    }
}

pub struct TransferFunder {
    faucet_account: RwLock<LocalAccount>,

    transaction_factory: TransactionFactory,

    /// URL of an Aptos node API.
    node_url: Url,
    node_api_key: Option<String>,
    node_additional_headers: Option<HashMap<String, String>>,

    /// The minimum amount of funds the Funder should have to operate.
    minimum_funds: MinimumFunds,

    /// Maximum amount we'll fund an account.
    amount_to_fund: AmountToFund,

    /// See comment of gas_unit_price.
    gas_unit_price_manager: GasUnitPriceManager,

    /// If this is Some, we'll use this. If not, we'll get the gas_unit_price
    /// from the gas_unit_price_manager.
    gas_unit_price_override: Option<u64>,

    /// When recovering from being overloaded, this struct ensures we handle
    /// requests in the order they came in.
    outstanding_requests: RwLock<Vec<(AccountAddress, u64)>>,

    /// Amount of time we'll wait for the seqnum to catch up before resetting it.
    wait_for_outstanding_txns_secs: u64,

    /// If set, we won't return responses until the transaction is processed.
    wait_for_transactions: bool,
}

impl TransferFunder {
    pub fn new(
        faucet_account: LocalAccount,
        chain_id: ChainId,
        node_url: Url,
        node_api_key: Option<String>,
        node_additional_headers: Option<HashMap<String, String>>,
        minimum_funds: MinimumFunds,
        amount_to_fund: AmountToFund,
        gas_unit_price_ttl_secs: Duration,
        gas_unit_price_override: Option<u64>,
        max_gas_amount: u64,
        transaction_expiration_secs: u64,
        wait_for_outstanding_txns_secs: u64,
        wait_for_transactions: bool,
    ) -> Self {
        let gas_unit_price_manager =
            GasUnitPriceManager::new(node_url.clone(), gas_unit_price_ttl_secs);

        Self {
            faucet_account: RwLock::new(faucet_account),
            transaction_factory: TransactionFactory::new(chain_id)
                .with_max_gas_amount(max_gas_amount)
                .with_transaction_expiration_time(transaction_expiration_secs),
            node_url,
            node_api_key,
            node_additional_headers,
            minimum_funds,
            amount_to_fund,
            gas_unit_price_manager,
            gas_unit_price_override,
            outstanding_requests: RwLock::new(vec![]),
            wait_for_outstanding_txns_secs,
            wait_for_transactions,
        }
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

    async fn get_gas_unit_price(&self) -> Result<u64, AptosTapError> {
        match self.gas_unit_price_override {
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

    /// This function builds, signs, submits, waits for, and checks the result
    /// of a transaction.
    async fn execute_transaction(
        &self,
        client: &Client,
        payload: TransactionPayload,
        // Only used for logging.
        receiver_address: &AccountAddress,
    ) -> Result<SignedTransaction, AptosTapError> {
        // Build a transaction factory using the gas unit price from the
        // GasUnitPriceManager. This mostly ensures that we will build a
        // transaction with a gas unit price that will be accepted.
        let transaction_factory = self
            .transaction_factory
            .clone()
            .with_gas_unit_price(self.get_gas_unit_price().await?);

        let transaction_builder = transaction_factory.payload(payload);

        let signed_transaction = self
            .faucet_account
            .write()
            .await
            .sign_with_transaction_builder(transaction_builder);

        submit_transaction(
            client,
            &self.faucet_account,
            signed_transaction,
            receiver_address,
            self.wait_for_transactions,
        )
        .await
    }

    async fn is_healthy_as_result(&self) -> Result<(), AptosTapError> {
        let funder_health = self.is_healthy().await;
        if !funder_health.can_process_requests {
            return Err(AptosTapError::new(
                format!(
                    "Tap TransferFunder is not able to handle requests right now: {}",
                    funder_health
                        .message
                        .unwrap_or_else(|| "no message".to_string()),
                ),
                AptosTapErrorCode::FunderAccountProblem,
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl FunderTrait for TransferFunder {
    /// Before actually initiating the fund transaction, we do a set of checks,
    /// such as ensuring that the funder has sufficient funds and that the
    /// receiver account does not yet exist. These are not meant to completely
    /// verify these preconditions, as there could be races between the checks
    /// and the transaction submission between requests, but it reduces the
    /// prevalence of transaction failure. The transaction we submit ensures
    /// that the account doesn't exist already, so that's our real guarantee,
    /// the prior checks are just to avoid paying gas if we don't need to.
    /// If check_only is set, we only do the initial checks without actually
    /// submitting any transactions.
    async fn fund(
        &self,
        amount: Option<u64>,
        receiver_address: AccountAddress,
        check_only: bool,
        did_bypass_checkers: bool,
    ) -> Result<Vec<SignedTransaction>, AptosTapError> {
        // Confirm the funder has sufficient balance, return a 500 if not. This
        // will only happen briefly, soon after we get into this state the LB
        // will deregister this instance based on the health check responses
        // being returned from `/`.
        self.is_healthy_as_result().await?;

        let client = self.get_api_client();

        // Determine amount to fund.
        let amount = self.get_amount(amount, did_bypass_checkers);

        // Update the sequence numbers of the accounts.
        let (_funder_seq_num, receiver_seq_num) = update_sequence_numbers(
            &client,
            &self.faucet_account,
            &self.outstanding_requests,
            receiver_address,
            amount,
            self.wait_for_outstanding_txns_secs,
        )
        .await?;

        // When updating the sequence numbers, we expect that the receiver sequence
        // number should be None, because the account should not exist yet.
        if receiver_seq_num.is_some() {
            return Err(AptosTapError::new(
                "Account ineligible".to_string(),
                AptosTapErrorCode::Rejected,
            )
            .rejection_reasons(vec![RejectionReason::new(
                format!("Account {} already exists", receiver_address),
                RejectionReasonCode::AccountAlreadyExists,
            )]));
        }

        // This Move function checks if the account exists, and if it does,
        // returns an error. If not, it creates the account and transfers the
        // requested amount of coins to it.
        let transactions = if check_only {
            vec![]
        } else {
            let txn = self
                .execute_transaction(
                    &client,
                    aptos_stdlib::aptos_account_transfer(receiver_address, amount),
                    &receiver_address,
                )
                .await?;
            info!(
                hash = txn.committed_hash().to_hex_literal(),
                address = receiver_address,
                amount = amount,
                event = "transaction_submitted"
            );
            vec![txn]
        };

        Ok(transactions)
    }

    fn get_amount(
        &self,
        amount: Option<u64>,
        // Ignored for now with TransferFunder, since generally we don't use Bypassers
        // when using the TransferFunder.
        _did_bypass_checkers: bool,
    ) -> u64 {
        match amount {
            Some(amount) => std::cmp::min(amount, self.amount_to_fund.0),
            None => self.amount_to_fund.0,
        }
    }

    /// Assert funder account actually exists and has the minimum funds.
    async fn is_healthy(&self) -> FunderHealthMessage {
        let account_address = self.faucet_account.read().await.address();
        let funder_balance = match self
            .get_api_client()
            .view_apt_account_balance(account_address)
            .await
        {
            Ok(response) => response.into_inner(),
            Err(e) => return FunderHealthMessage {
                can_process_requests: false,
                message: Some(format!(
                    "Failed to get account balance to determine whether tap account has sufficient funds: {:#}",
                    e
                )),
            },
        };

        TRANSFER_FUNDER_ACCOUNT_BALANCE.set(funder_balance as i64);

        if funder_balance < self.minimum_funds.0 {
            FunderHealthMessage {
                can_process_requests: false,
                message: Some(format!(
                    "Funder account {} has insufficient funds. It has {}, but the minimum is {}",
                    account_address, funder_balance, self.minimum_funds.0
                )),
            }
        } else {
            FunderHealthMessage {
                can_process_requests: true,
                message: None,
            }
        }
    }
}

fn account_address_from_private_key(private_key: &Ed25519PrivateKey) -> AccountAddress {
    let public_key = private_key.public_key();
    let auth_key = AuthenticationKey::ed25519(&public_key);
    AccountAddress::new(*auth_key.account_address())
}

// Use newtypes so we don't accidentally mix these up.

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MinimumFunds(pub u64);

impl std::fmt::Display for MinimumFunds {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl FromStr for MinimumFunds {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = s
            .parse::<u64>()
            .with_context(|| format!("Parsing u64 string {} failed", s))?;
        Ok(Self(data))
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AmountToFund(pub u64);

impl std::fmt::Display for AmountToFund {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl FromStr for AmountToFund {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = s
            .parse::<u64>()
            .with_context(|| format!("Parsing u64 string {} failed", s))?;
        Ok(Self(data))
    }
}
