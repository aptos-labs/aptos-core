// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{FunderHealthMessage, FunderTrait};
use crate::endpoints::{AptosTapError, AptosTapErrorCode};
use anyhow::{Context, Result};
use aptos_logger::info;
use aptos_sdk::{
    crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
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
use std::sync::Arc;
use tokio::sync::RwLock;
use rand::rngs::OsRng;

static MINTER_SCRIPT: &[u8] = include_bytes!(
    "../../../../../aptos-move/move-examples/scripts/minter/build/Minter/bytecode_scripts/main.mv"
);

use super::common::{
    submit_transaction, update_sequence_numbers, ApiConnectionConfig, AssetConfig,
    GasUnitPriceManager, TransactionSubmissionConfig, DEFAULT_ASSET_NAME,
};

/// Helper function to clone an Ed25519PrivateKey by serializing and deserializing it.
/// This is necessary because Ed25519PrivateKey doesn't implement Clone.
fn clone_private_key(key: &Ed25519PrivateKey) -> Ed25519PrivateKey {
    let serialized: &[u8] = &(key.to_bytes());
    Ed25519PrivateKey::try_from(serialized)
        .expect("Failed to deserialize private key - this should never happen")
}

/// Asset configuration specific to minting, extends the base AssetConfig with mint-specific fields.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MintAssetConfig {
    #[serde(flatten)]
    pub base: AssetConfig,

    /// Address of the account to send transactions from. On localnet, for
    /// example, this is a550c18. If not given, we use the account address
    /// corresponding to the given private key.
    pub mint_account_address: Option<AccountAddress>,

    /// Just use the account given in funder args, don't make a new one and
    /// delegate the mint capability to it.
    pub do_not_delegate: bool,
}

impl MintAssetConfig {
    pub fn new(
        base: AssetConfig,
        mint_account_address: Option<AccountAddress>,
        do_not_delegate: bool,
    ) -> Self {
        Self {
            base,
            mint_account_address,
            do_not_delegate,
        }
    }

    /// Delegate to the base AssetConfig's get_key method.
    pub fn get_key(&self) -> Result<Ed25519PrivateKey> {
        self.base.get_key()
    }
}

/// explain these contain additional args for the mint funder.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MintFunderConfig {
    #[serde(flatten)]
    pub api_connection_config: ApiConnectionConfig,

    #[serde(flatten)]
    pub transaction_submission_config: TransactionSubmissionConfig,

    pub assets: HashMap<String, MintAssetConfig>,

    /// Default asset to use when no asset is specified in requests.
    /// If not provided, defaults to "apt".
    #[serde(default)]
    pub default_asset: Option<String>,

    pub amount_to_fund: u64,
}

impl MintFunderConfig {
    pub async fn build_funder(self) -> Result<MintFunder> {
        // Validate we have at least one asset
        if self.assets.is_empty() {
            return Err(anyhow::anyhow!("No assets configured"));
        }

        // Resolve default asset: use configured value or fall back to constant
        let default_asset = self
            .default_asset
            .unwrap_or_else(|| DEFAULT_ASSET_NAME.to_string());

        // Validate that the default asset exists in the assets map
        let default_asset_config = self.assets.get(&default_asset).ok_or_else(|| {
            anyhow::anyhow!(
                "Default asset '{}' is not configured in assets map",
                default_asset
            )
        })?;

        let key = default_asset_config.get_key()?;
        let initial_account = LocalAccount::new(
            default_asset_config
                .mint_account_address
                .unwrap_or_else(|| {
                    AuthenticationKey::ed25519(&Ed25519PublicKey::from(&key)).account_address()
                }),
            key,
            0,
        );

        let minter = MintFunder::new(
            self.api_connection_config.node_url.clone(),
            self.api_connection_config.api_key.clone(),
            self.api_connection_config.additional_headers.clone(),
            self.api_connection_config.chain_id,
            self.transaction_submission_config,
            initial_account,
            self.assets.clone(),
            default_asset.clone(),
            self.amount_to_fund,
        );

        // If default asset needs delegation, do it once at startup
        if !default_asset_config.do_not_delegate {
            let delegated_account = minter
                .use_delegated_account_for_asset(&default_asset)
                .await
                .context("Failed to make MintFunder use delegated account")?;
            // Set it as the current faucet account
            // Note: delegated_account is already a LocalAccount (not Arc) from use_delegated_account_for_asset
            {
                let mut faucet_account = minter.faucet_account.write().await;
                *faucet_account = delegated_account;
            }
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

    // Multi-asset support: store asset configs
    assets: HashMap<String, MintAssetConfig>,
    default_asset: String,
    amount_to_fund: u64,

    // Cache of delegated accounts per asset to avoid recreating them
    // Using Arc to allow sharing the non-cloneable LocalAccount
    delegated_accounts: RwLock<HashMap<String, Arc<LocalAccount>>>,
}

impl MintFunder {
    pub fn new(
        node_url: Url,
        node_api_key: Option<String>,
        node_additional_headers: Option<HashMap<String, String>>,
        chain_id: ChainId,
        txn_config: TransactionSubmissionConfig,
        faucet_account: LocalAccount,
        assets: HashMap<String, MintAssetConfig>,
        default_asset: String,
        amount_to_fund: u64,
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
            assets,
            default_asset,
            amount_to_fund,
            delegated_accounts: RwLock::new(HashMap::new()),
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

    /// Legacy method for backward compatibility - delegates for the current asset
    pub async fn use_delegated_account(&self) -> Result<()> {
        // This is only used at startup for the default asset
        // We'll handle it differently in build_funder
        let delegated_account = self.use_delegated_account_for_asset(&self.default_asset).await?;
        let mut faucet_account = self.faucet_account.write().await;
        *faucet_account = delegated_account;
        Ok(())
    }

    /// Performs the delegated account creation and delegation for a specific asset.
    /// Returns the delegated account and caches it.
    ///
    /// This method uses a double-check locking pattern to prevent race conditions:
    /// 1. First check the cache with a read lock (fast path for already-delegated assets)
    /// 2. If not found, acquire a write lock and check again (prevents duplicate creation)
    /// 3. If still not found, create and delegate a new account, then cache it
    ///
    /// The write lock is held during account creation to ensure only one thread creates
    /// a delegated account per asset, even if multiple requests arrive concurrently.
    pub async fn use_delegated_account_for_asset(&self, asset_name: &str) -> Result<LocalAccount> {
        // Fast path: Check cache with read lock first
        // This allows concurrent reads for already-delegated assets
        {
            let delegated_accounts = self.delegated_accounts.read().await;
            if let Some(cached_account) = delegated_accounts.get(asset_name) {
                // Reconstruct LocalAccount from Arc (since LocalAccount doesn't implement Clone)
                // Clone the private key by serializing/deserializing
                let private_key = clone_private_key(cached_account.private_key());
                return Ok(LocalAccount::new(
                    cached_account.address(),
                    private_key,
                    cached_account.sequence_number(),
                ));
            }
        }

        // Slow path: Acquire write lock before creating account
        // This ensures only one thread creates the delegated account for this asset
        let mut delegated_accounts = self.delegated_accounts.write().await;

        // Double-check: Another thread may have created it while we waited for the write lock
        if let Some(cached_account) = delegated_accounts.get(asset_name) {
            // Reconstruct LocalAccount from Arc
            // Clone the private key by serializing/deserializing
            let private_key = clone_private_key(cached_account.private_key());
            return Ok(LocalAccount::new(
                cached_account.address(),
                private_key,
                cached_account.sequence_number(),
            ));
        }

        // We're the first to create a delegated account for this asset
        // The write lock is held during all async operations to prevent duplicates
        // Build a client.
        let client = self.get_api_client();

        // Create a new random account, then delegate to it
        let delegated_account = LocalAccount::generate(&mut OsRng::default());

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

        // Get the current faucet account (which should be the mint account for this asset)
        // This was set by the caller in the fund() method before calling this function
        let mint_account = {
            let faucet_account = self.faucet_account.read().await;
            // Reconstruct from the account to avoid cloning issues
            // Clone the private key by serializing/deserializing
            let private_key = clone_private_key(faucet_account.private_key());
            LocalAccount::new(
                faucet_account.address(),
                private_key,
                faucet_account.sequence_number(),
            )
        };

        // Delegate minting capability from the mint account to the new delegated account
        client
            .submit_and_wait(&mint_account.sign_with_transaction_builder(
                transaction_factory.payload(aptos_stdlib::aptos_coin_delegate_mint_capability(
                    delegated_account.address(),
                )),
            ))
            .await
            .context("Failed to delegate minting to the new account")?;

        // Claim the mint capability on the delegated account
        client
            .submit_and_wait(&delegated_account.sign_with_transaction_builder(
                transaction_factory.payload(aptos_stdlib::aptos_coin_claim_mint_capability()),
            ))
            .await
            .context("Failed to claim the minting capability")?;

        info!(
            "Successfully configured MintFunder to use delegated account for asset '{}': {}",
            asset_name,
            delegated_account.address()
        );

        // Cache the delegated account so future requests for this asset can reuse it
        let account_arc = Arc::new(delegated_account);
        delegated_accounts.insert(asset_name.to_string(), Arc::clone(&account_arc));

        // Reconstruct and return LocalAccount (not Arc)
        // Clone the private key by serializing/deserializing
        let private_key = clone_private_key(account_arc.private_key());
        Ok(LocalAccount::new(
            account_arc.address(),
            private_key,
            account_arc.sequence_number(),
        ))
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

    /// Core processing logic that handles sequence numbers and transaction submission.
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
        asset: Option<String>,
        check_only: bool,
        did_bypass_checkers: bool,
    ) -> Result<Vec<SignedTransaction>, AptosTapError> {
        // Resolve asset (use configured default if not specified)
        let asset_name = asset.as_deref().unwrap_or(&self.default_asset);

        // Get asset config
        let asset_config = self.assets.get(asset_name).ok_or_else(|| {
            AptosTapError::new(
                format!("Asset '{}' is not configured", asset_name),
                AptosTapErrorCode::InvalidRequest,
            )
        })?;

        // Switch to the mint account for this asset
        // Each asset has its own mint account (with its own private key)
        // We need to switch faucet_account to the correct mint account before processing
        let key = asset_config.get_key().map_err(|e| {
            AptosTapError::new_with_error_code(e, AptosTapErrorCode::InvalidRequest)
        })?;
        let account_address = asset_config.mint_account_address.unwrap_or_else(|| {
            AuthenticationKey::ed25519(&Ed25519PublicKey::from(&key)).account_address()
        });
        let mint_account = LocalAccount::new(account_address, key, 0);

        // Set the mint account as the current faucet account
        // This is a per-request operation - faucet_account is shared state that gets
        // switched for each request based on the requested asset
        {
            let mut faucet_account = self.faucet_account.write().await;
            *faucet_account = mint_account;
        }

        // Get or create the delegated account for this asset (if delegation is needed)
        // The delegated account is the one that actually performs the minting.
        // It's created once per asset and cached for reuse.
        //
        // Note: use_delegated_account_for_asset reads from faucet_account, so we must
        // set the mint account first (which we did above).
        if !asset_config.do_not_delegate {
            let delegated_account = self.use_delegated_account_for_asset(asset_name).await
                .map_err(|e| {
                    AptosTapError::new_with_error_code(e, AptosTapErrorCode::InternalError)
                })?;
            // Switch to the delegated account - this is the account that will sign the mint transaction
            // Note: delegated_account is already a LocalAccount (not Arc) from use_delegated_account_for_asset
            {
                let mut faucet_account = self.faucet_account.write().await;
                *faucet_account = delegated_account;
            }
        }

        // Process the minting request using the correct account
        // At this point, faucet_account points to either:
        // - The delegated account (if do_not_delegate is false)
        // - The mint account directly (if do_not_delegate is true)
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
            (None, Some(maximum_amount)) => std::cmp::min(self.amount_to_fund, maximum_amount),
            (None, None) => self.amount_to_fund,
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
