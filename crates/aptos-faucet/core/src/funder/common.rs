// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    endpoints::{AptosTapError, AptosTapErrorCode},
    middleware::NUM_OUTSTANDING_TRANSACTIONS,
};
use anyhow::{anyhow, Context, Result};
use aptos_config::keys::ConfigKey;
use aptos_logger::{
    error, info,
    prelude::{sample, SampleRate},
    warn,
};
use aptos_sdk::{
    crypto::ed25519::Ed25519PrivateKey,
    rest_client::Client,
    types::{
        account_address::AccountAddress, chain_id::ChainId, transaction::SignedTransaction,
        LocalAccount,
    },
};
use clap::Parser;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

// Default max in mempool is 20.
const MAX_NUM_OUTSTANDING_TRANSACTIONS: u64 = 15;

const DEFAULT_KEY_FILE_PATH: &str = "/opt/aptos/etc/mint.key";

/// This defines configuration for any Funder that needs to interact with a real
/// blockchain API. This includes the MintFunder and the TransferFunder currently.
///
/// Note that the clap derives are only necessary for the use of this struct from the
/// faucet CLI, they are not necessary for the service.
#[derive(Clone, Debug, Deserialize, Parser, Serialize)]
pub struct ApiConnectionConfig {
    /// Aptos node (any node type with an open API) server URL.
    /// Include the port in this if not using the default for the scheme.
    #[clap(long, default_value = "https://fullnode.testnet.aptoslabs.com/")]
    pub node_url: Url,

    /// API key for talking to the node API.
    #[clap(long)]
    pub api_key: Option<String>,

    /// Any additional headers to send with the request. We don't accept this on the
    /// CLI.
    #[clap(skip)]
    pub additional_headers: Option<HashMap<String, String>>,

    /// Path to the private key for creating test account and minting coins in
    /// the MintFunder case, or for transferring coins in the TransferFunder case.
    /// To keep Testnet simple, we used one private key for aptos root account
    /// To manually generate a keypair, use generate-key:
    /// `cargo run -p generate-keypair -- -o <output_file_path>`
    #[serde(default = "ApiConnectionConfig::default_mint_key_file_path")]
    #[clap(long, default_value = DEFAULT_KEY_FILE_PATH, value_parser)]
    key_file_path: PathBuf,

    /// Hex string of an Ed25519PrivateKey for minting / transferring coins.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[clap(long, value_parser = ConfigKey::<Ed25519PrivateKey>::from_encoded_string)]
    key: Option<ConfigKey<Ed25519PrivateKey>>,

    /// Chain ID of the network this client is connecting to. For example, for mainnet:
    /// "MAINNET" or 1, testnet: "TESTNET" or 2. If there is no predefined string
    /// alias (e.g. "MAINNET"), just use the number. Note: Chain ID of 0 is not allowed.
    #[clap(long, default_value_t = ChainId::testnet())]
    pub chain_id: ChainId,
}

impl ApiConnectionConfig {
    pub fn new(
        node_url: Url,
        api_key: Option<String>,
        additional_headers: Option<HashMap<String, String>>,
        key_file_path: PathBuf,
        key: Option<ConfigKey<Ed25519PrivateKey>>,
        chain_id: ChainId,
    ) -> Self {
        Self {
            node_url,
            api_key,
            additional_headers,
            key_file_path,
            key,
            chain_id,
        }
    }

    fn default_mint_key_file_path() -> PathBuf {
        PathBuf::from_str(DEFAULT_KEY_FILE_PATH).unwrap()
    }

    pub fn get_key(&self) -> Result<Ed25519PrivateKey> {
        if let Some(ref key) = self.key {
            return Ok(key.private_key());
        }
        let key_bytes = std::fs::read(self.key_file_path.as_path()).with_context(|| {
            format!(
                "Failed to read key file: {}",
                self.key_file_path.to_string_lossy()
            )
        })?;
        // decode as bcs first, fall back to a file of hex
        let result = aptos_sdk::bcs::from_bytes(&key_bytes); //.with_context(|| "bad bcs");
        if let Ok(x) = result {
            return Ok(x);
        }
        let keystr = String::from_utf8(key_bytes).map_err(|e| anyhow!(e))?;
        Ok(ConfigKey::from_encoded_string(keystr.as_str())
            .with_context(|| {
                format!(
                    "{}: key file failed as both bcs and hex",
                    self.key_file_path.to_string_lossy()
                )
            })?
            .private_key())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransactionSubmissionConfig {
    /// Maximum amount of OCTA to give an account.
    maximum_amount: Option<u64>,

    /// With this it is possible to set a different maximum amount for requests that
    /// were allowed to skip the Checkers by a Bypasser. This can be helpful for CI,
    /// where we might need to mint a greater amount than is normally required in the
    /// standard case. If not given, maximum_amount is used whether the request
    /// bypassed the checks or not.
    maximum_amount_with_bypass: Option<u64>,

    /// How long to wait between fetching updated gas unit prices.
    #[serde(default = "TransactionSubmissionConfig::default_gas_unit_price_ttl_secs")]
    gas_unit_price_ttl_secs: u16,

    /// If given, we'll use this value for the gas unit price. If not, we'll use
    /// the gas unit price estimation API periodically.
    pub gas_unit_price_override: Option<u64>,

    /// The maximum amount of gas to spend on a single transfer.
    #[serde(default = "TransactionSubmissionConfig::default_max_gas_amount")]
    pub max_gas_amount: u64,

    /// Expiration time we'll allow for transactions.
    #[serde(default = "TransactionSubmissionConfig::default_transaction_expiration_secs")]
    pub transaction_expiration_secs: u64,

    /// Amount of time we'll wait for the seqnum to catch up before resetting it.
    #[serde(default = "TransactionSubmissionConfig::default_wait_for_outstanding_txns_secs")]
    pub wait_for_outstanding_txns_secs: u64,

    /// Whether to wait for the transaction before returning.
    #[serde(default)]
    pub wait_for_transactions: bool,
}

impl TransactionSubmissionConfig {
    pub fn new(
        maximum_amount: Option<u64>,
        maximum_amount_with_bypass: Option<u64>,
        gas_unit_price_ttl_secs: u16,
        gas_unit_price_override: Option<u64>,
        max_gas_amount: u64,
        transaction_expiration_secs: u64,
        wait_for_outstanding_txns_secs: u64,
        wait_for_transactions: bool,
    ) -> Self {
        Self {
            maximum_amount,
            maximum_amount_with_bypass,
            gas_unit_price_ttl_secs,
            gas_unit_price_override,
            max_gas_amount,
            transaction_expiration_secs,
            wait_for_outstanding_txns_secs,
            wait_for_transactions,
        }
    }

    fn default_gas_unit_price_ttl_secs() -> u16 {
        30
    }

    fn default_max_gas_amount() -> u64 {
        500_000
    }

    fn default_transaction_expiration_secs() -> u64 {
        25
    }

    fn default_wait_for_outstanding_txns_secs() -> u64 {
        30
    }

    pub fn get_gas_unit_price_ttl_secs(&self) -> Duration {
        Duration::from_secs(self.gas_unit_price_ttl_secs.into())
    }

    /// If a Bypasser let the request bypass the Checkers and
    /// maximum_amount_with_bypass is set, this function will return
    /// that. Otherwise it will return maximum_amount.
    pub fn get_maximum_amount(
        &self,
        // True if a Bypasser let the request bypass the Checkers.
        did_bypass_checkers: bool,
    ) -> Option<u64> {
        match (self.maximum_amount_with_bypass, did_bypass_checkers) {
            (Some(max), true) => Some(max),
            _ => self.maximum_amount,
        }
    }
}

struct NumOutstandingTransactionsResetter;

impl Drop for NumOutstandingTransactionsResetter {
    fn drop(&mut self) {
        NUM_OUTSTANDING_TRANSACTIONS.set(0);
    }
}

/// This function is responsible for updating our local record of the sequence
/// numbers of the funder and receiver accounts.
pub async fn update_sequence_numbers(
    client: &Client,
    funder_account: &RwLock<LocalAccount>,
    // The value here is the requester address and amount requested.
    outstanding_requests: &RwLock<Vec<(AccountAddress, u64)>>,
    receiver_address: AccountAddress,
    amount: u64,
    wait_for_outstanding_txns_secs: u64,
) -> Result<(u64, Option<u64>), AptosTapError> {
    let (mut funder_seq, mut receiver_seq) =
        get_sequence_numbers(client, funder_account, receiver_address).await?;
    let our_funder_seq = {
        let funder_account = funder_account.write().await;

        // If the onchain sequence_number is greater than what we have, update our
        // sequence_numbers
        if funder_seq > funder_account.sequence_number() {
            funder_account.set_sequence_number(funder_seq);
        }
        funder_account.sequence_number()
    };

    let _resetter = NumOutstandingTransactionsResetter;

    let mut set_outstanding = false;
    // We shouldn't have too many outstanding txns
    for _ in 0..(wait_for_outstanding_txns_secs * 2) {
        if our_funder_seq < funder_seq + MAX_NUM_OUTSTANDING_TRANSACTIONS {
            // Enforce a stronger ordering of priorities based upon the MintParams that arrived
            // first. Then put the other folks to sleep to try again until the queue fills up.
            if !set_outstanding {
                let mut requests = outstanding_requests.write().await;
                requests.push((receiver_address, amount));
                set_outstanding = true;
            }

            if outstanding_requests.read().await.first() == Some(&(receiver_address, amount)) {
                // There might have been two requests with the same parameters, so we ensure that
                // we only pop off one of them. We do a read lock first since that is cheap,
                // followed by a write lock.
                let mut requests = outstanding_requests.write().await;
                if requests.first() == Some(&(receiver_address, amount)) {
                    requests.remove(0);
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            continue;
        }
        let num_outstanding = our_funder_seq - funder_seq;

        sample!(
            SampleRate::Duration(Duration::from_secs(2)),
            warn!(
                "We have too many outstanding transactions: {}. Sleeping to let the system catchup.",
                num_outstanding
            );
        );

        // Report the number of outstanding transactions.
        NUM_OUTSTANDING_TRANSACTIONS.set(num_outstanding as i64);

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        (funder_seq, receiver_seq) =
            get_sequence_numbers(client, funder_account, receiver_address).await?;
    }

    // If after 30 seconds we still have not caught up, we are likely unhealthy.
    if our_funder_seq >= funder_seq + MAX_NUM_OUTSTANDING_TRANSACTIONS {
        error!("We are unhealthy, transactions have likely expired.");
        let funder_account = funder_account.write().await;
        if funder_account.sequence_number() >= funder_seq + MAX_NUM_OUTSTANDING_TRANSACTIONS {
            info!("Resetting the sequence number counter.");
            funder_account.set_sequence_number(funder_seq);
        } else {
            info!("Someone else reset the sequence number counter ahead of us.");
        }
    }

    // After this point we report 0 outstanding transactions. This happens by virtue
    // of the NumOutstandingTransactionsResetter dropping out of scope. We do it this
    // way instead of explicitly calling it here because if the caller hangs up part
    // way through the request, the future for the request handler stops getting polled,
    // meaning we'd never make it here. Leveraging Drop makes sure it always happens.

    Ok((funder_seq, receiver_seq))
}

/// This function gets the sequence number for the funder account (sender)
/// and the receiver account. It can return an error if the funder account
/// does not exist.
async fn get_sequence_numbers(
    client: &Client,
    funder_account: &RwLock<LocalAccount>,
    receiver_address: AccountAddress,
) -> Result<(u64, Option<u64>), AptosTapError> {
    let funder_address = funder_account.read().await.address();
    let f_request = client.get_account(funder_address);
    let r_request = client.get_account(receiver_address);
    let mut responses = futures::future::join_all([f_request, r_request]).await;

    let receiver_seq_num = responses
        .remove(1)
        .as_ref()
        .ok()
        .map(|account| account.inner().sequence_number);

    let funder_seq_num = responses
        .remove(0)
        .map_err(|e| {
            AptosTapError::new(
                format!("funder account {} not found: {:#}", funder_address, e),
                AptosTapErrorCode::AccountDoesNotExist,
            )
        })?
        .inner()
        .sequence_number;

    Ok((funder_seq_num, receiver_seq_num))
}

/// Submit a transaction, potentially wait for it depending on `wait_for_transactions`
pub async fn submit_transaction(
    client: &Client,
    faucet_account: &RwLock<LocalAccount>,
    signed_transaction: SignedTransaction,
    receiver_address: &AccountAddress,
    wait_for_transactions: bool,
) -> Result<SignedTransaction, AptosTapError> {
    let (result, event_on_success) = if wait_for_transactions {
        // If this fails, we assume it is the user's fault, e.g. because the
        // account already exists, but it is possible that the transaction
        // timed out. It's hard to tell because this function returns an opaque
        // anyhow error. https://github.com/aptos-labs/aptos-tap/issues/60.
        (
            client
                .submit_and_wait_bcs(&signed_transaction)
                .await
                .map(|_| ())
                .map_err(|e| {
                    AptosTapError::new_with_error_code(e, AptosTapErrorCode::TransactionFailed)
                }),
            "transaction_success",
        )
    } else {
        (
            client
                .submit_bcs(&signed_transaction)
                .await
                .map(|_| ())
                .map_err(|e| {
                    AptosTapError::new_with_error_code(e, AptosTapErrorCode::TransactionFailed)
                }),
            "transaction_submitted",
        )
    };

    // If there was an issue submitting a transaction we should just reset
    // our sequence numbers to what it was before.
    match result {
        Ok(_) => {
            info!(
                hash = signed_transaction.committed_hash(),
                address = receiver_address,
                event = event_on_success,
            );
            Ok(signed_transaction)
        },
        Err(e) => {
            faucet_account.write().await.decrement_sequence_number();
            warn!(
                hash = signed_transaction.committed_hash(),
                address = receiver_address,
                event = "transaction_failure",
                error_message = format!("{:#}", e)
            );
            Err(e)
        },
    }
}

/// This struct manages gas unit price. When callers get the value through this
/// struct, it will update the value if it is too old.
pub struct GasUnitPriceManager {
    api_client: aptos_sdk::rest_client::Client,
    gas_unit_price: AtomicU64,
    last_updated: Arc<RwLock<Option<Instant>>>,
    cache_ttl: Duration,
}

impl GasUnitPriceManager {
    pub fn new(node_url: Url, cache_ttl: Duration) -> Self {
        Self {
            api_client: aptos_sdk::rest_client::Client::new(node_url),
            gas_unit_price: AtomicU64::new(0),
            last_updated: Arc::new(RwLock::new(None)),
            cache_ttl,
        }
    }

    pub async fn get_gas_unit_price(&self) -> Result<u64> {
        let now = Instant::now();

        // If we're still within the TTL, just return the current value.
        if let Some(last_updated) = *self.last_updated.read().await {
            if now.duration_since(last_updated) < self.cache_ttl {
                return Ok(self.gas_unit_price.load(Ordering::Acquire));
            }
        }

        // We're beyond the TTL, update the value and last_updated.
        let mut last_updated = self.last_updated.write().await;
        let new_price = self.fetch_gas_unit_price().await?;
        self.gas_unit_price.store(new_price, Ordering::Release);
        *last_updated = Some(now);

        info!(gas_unit_price = new_price, event = "gas_unit_price_updated");
        Ok(new_price)
    }

    async fn fetch_gas_unit_price(&self) -> Result<u64> {
        Ok(self
            .api_client
            .estimate_gas_price()
            .await?
            .into_inner()
            .gas_estimate)
    }
}
