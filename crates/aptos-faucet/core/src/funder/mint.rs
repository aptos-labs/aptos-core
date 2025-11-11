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
        AccountKey, LocalAccount,
    },
};
use async_trait::async_trait;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};

static MINTER_SCRIPT: &[u8] = include_bytes!(
    "../../../../../aptos-move/move-examples/scripts/minter/build/Minter/bytecode_scripts/main.mv"
);

use super::common::{
    submit_transaction, update_sequence_numbers, ApiConnectionConfig, GasUnitPriceManager,
    TransactionSubmissionConfig, AssetConfig
};

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

    pub amount_to_fund: u64,
}

impl MintFunderConfig {
    pub async fn build_funder(self) -> Result<MintFunder> {
        // Validate we have at least one asset
        if self.assets.is_empty() {
            return Err(anyhow::anyhow!("No assets configured"));
        }

        // Initialize with ANY asset - we don't know which one will be used until the request comes in
        let default_asset_config = self.assets.get("apt")
            .or_else(|| self.assets.values().next())
            .unwrap();

        let key = default_asset_config.get_key()?;
        let initial_account = LocalAccount::new(
            default_asset_config.mint_account_address.unwrap_or_else(|| {
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
            self.amount_to_fund,
        );

        // Pre-create delegated accounts for all assets that need delegation
        minter.initialize_delegated_accounts(&self.assets).await?;

        Ok(minter)
    }
}

pub struct MintFunder {
    // Keep existing fields
    node_url: Url,
    node_api_key: Option<String>,
    node_additional_headers: Option<HashMap<String, String>>,
    txn_config: TransactionSubmissionConfig,
    transaction_factory: TransactionFactory,
    gas_unit_price_manager: GasUnitPriceManager,
    outstanding_requests: RwLock<Vec<(AccountAddress, u64)>>,

    // Keep single faucet account - just switch the key/address per request
    faucet_account: RwLock<LocalAccount>,

    // Store asset configs for lookup
    assets: HashMap<String, MintAssetConfig>,

    // Amount to fund when no specific amount is requested
    amount_to_fund: u64,

    // Cache for delegated accounts per asset (to avoid recreating them on every request)
    delegated_accounts_cache: Mutex<HashMap<String, LocalAccount>>,
}

impl MintFunder {
    pub fn new(
        node_url: Url,
        node_api_key: Option<String>,
        node_additional_headers: Option<HashMap<String, String>>,
        chain_id: ChainId,
        txn_config: TransactionSubmissionConfig,
        initial_account: LocalAccount,
        assets: HashMap<String, MintAssetConfig>,
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
            transaction_factory,
            gas_unit_price_manager,
            outstanding_requests: RwLock::new(vec![]),
            faucet_account: RwLock::new(initial_account),
            assets,
            amount_to_fund,
            delegated_accounts_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Initialize delegated accounts for all assets during startup
    async fn initialize_delegated_accounts(&self, assets: &HashMap<String, MintAssetConfig>) -> Result<()> {
        for (asset_name, asset_config) in assets {
            if !asset_config.do_not_delegate {
                info!("Attempting to create delegated account for asset: {}", asset_name);

                let key = asset_config.get_key()?;
                let account_address = asset_config.mint_account_address.unwrap_or_else(|| {
                    AuthenticationKey::ed25519(&Ed25519PublicKey::from(&key)).account_address()
                });

                // Create base account and clone key for fallback
                let key_bytes = key.to_bytes();
                let cloned_key = Ed25519PrivateKey::try_from(key_bytes.as_ref())?;
                let mut base_account = LocalAccount::new(account_address, key, 0);

                // Try to create delegated account with timeout
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(30), // 30 second timeout
                    self.create_delegated_account_for_asset(&mut base_account)
                ).await {
                    Ok(Ok(delegated_account)) => {
                        // Success - cache the delegated account
                        let mut cache = self.delegated_accounts_cache.lock().await;
                        let private_key_bytes = delegated_account.private_key().to_bytes();
                        let cloned_private_key = Ed25519PrivateKey::try_from(private_key_bytes.as_ref())
                            .context("Failed to clone private key")?;
                        cache.insert(asset_name.clone(), LocalAccount::new(
                            delegated_account.address(),
                            AccountKey::from_private_key(cloned_private_key),
                            delegated_account.sequence_number(),
                        ));

                        info!("Successfully created delegated account for asset: {} at address: {}",
                              asset_name, delegated_account.address());
                    },
                    Ok(Err(e)) => {
                        // Delegation failed - fall back to base account
                        eprintln!("Warning: Failed to create delegated account for asset '{}': {}. Falling back to base account.", asset_name, e);

                        let fallback_key = Ed25519PrivateKey::try_from(key_bytes.as_ref())?;
                        let mut cache = self.delegated_accounts_cache.lock().await;
                        cache.insert(asset_name.clone(), LocalAccount::new(
                            account_address,
                            AccountKey::from_private_key(fallback_key),
                            0,
                        ));

                        info!("Using base account for asset: {} at address: {}", asset_name, account_address);
                    },
                    Err(_) => {
                        // Timeout - fall back to base account
                        eprintln!("Warning: Timeout creating delegated account for asset '{}'. Falling back to base account.", asset_name);

                        let mut cache = self.delegated_accounts_cache.lock().await;
                        cache.insert(asset_name.clone(), LocalAccount::new(
                            account_address,
                            AccountKey::from_private_key(cloned_key),
                            0,
                        ));

                        info!("Using base account for asset: {} at address: {}", asset_name, account_address);
                    }
                }
            }
        }
        Ok(())
    }

    /// Create a faucet account for the requested asset
    async fn create_faucet_account_for_asset(&self, asset_config: &MintAssetConfig, asset_name: &str) -> Result<LocalAccount, AptosTapError> {
        // Handle delegation if needed
        if !asset_config.do_not_delegate {
            // Get the cached delegated account (should already exist from initialization)
            let cache = self.delegated_accounts_cache.lock().await;
            if let Some(cached_account) = cache.get(asset_name) {
                // Return a fresh copy of the cached account with current sequence number from blockchain
                let private_key_bytes = cached_account.private_key().to_bytes();
                let cloned_private_key = Ed25519PrivateKey::try_from(private_key_bytes.as_ref())
                    .map_err(|e| AptosTapError::new_with_error_code(anyhow::anyhow!(e), AptosTapErrorCode::InternalError))?;

                // Get current sequence number from blockchain
                let client = self.get_api_client();
                let current_seq = match client.get_account_bcs(cached_account.address()).await {
                    Ok(account_info) => account_info.inner().sequence_number(),
                    Err(_) => 0, // Account doesn't exist yet, start from 0
                };

                Ok(LocalAccount::new(
                    cached_account.address(),
                    AccountKey::from_private_key(cloned_private_key),
                    current_seq,
                ))
            } else {
                Err(AptosTapError::new(
                    format!("Delegated account for asset '{}' not found in cache. This should have been created during initialization.", asset_name),
                    AptosTapErrorCode::InternalError,
                ))
            }
        } else {
            // For non-delegated accounts, create a fresh account each time
            let key = asset_config.get_key()
                .map_err(|e| AptosTapError::new_with_error_code(e, AptosTapErrorCode::InvalidRequest))?;

            let account_address = asset_config.mint_account_address.unwrap_or_else(|| {
                AuthenticationKey::ed25519(&Ed25519PublicKey::from(&key)).account_address()
            });

            // Get current sequence number from blockchain
            let client = self.get_api_client();
            let current_seq = match client.get_account_bcs(account_address).await {
                Ok(account_info) => account_info.inner().sequence_number(),
                Err(_) => 0, // Account doesn't exist yet, start from 0
            };

            Ok(LocalAccount::new(account_address, key, current_seq))
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

    /// Create a delegated account for a specific asset account with retry logic
    async fn create_delegated_account_for_asset(&self, asset_account: &mut LocalAccount) -> Result<LocalAccount> {
        // Build a client.
        let client = self.get_api_client();

        // Create a new random account, then delegate to it
        let delegated_account = LocalAccount::generate(&mut rand::rngs::OsRng);

        // Create the account, wait for the response.
        self.process_with_account(
            &client,
            asset_account,
            100_000_000_000,
            delegated_account
                .authentication_key()
                .clone()
                .account_address(),
            false,
            false, // Don't wait for transaction to avoid timeout
        )
        .await
        .context("Failed to create new account")?;

        // Build a transaction factory using the gas unit price from the
        // GasUnitPriceManager. This mostly ensures that we will build a
        // transaction with a gas unit price that will be accepted.
        let transaction_factory = self.get_transaction_factory().await?;

        // Delegate minting to the account (submit without waiting)
        let delegation_txn = asset_account.sign_with_transaction_builder(
            transaction_factory.payload(aptos_stdlib::aptos_coin_delegate_mint_capability(
                delegated_account.address(),
            )),
        );

        client
            .submit_bcs(&delegation_txn)
            .await
            .context("Failed to submit delegation transaction")?;

        // Wait a bit for the transaction to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Claim the capability (submit without waiting)
        let claim_txn = delegated_account.sign_with_transaction_builder(
            transaction_factory.payload(aptos_stdlib::aptos_coin_claim_mint_capability()),
        );

        client
            .submit_bcs(&claim_txn)
            .await
            .context("Failed to submit claim transaction")?;

        // Wait a bit for the claim transaction to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        info!(
            "Successfully configured MintFunder to use delegated account: {}",
            delegated_account.address()
        );

        Ok(delegated_account)
    }

    /// todo explain / rename - kept for backward compatibility but now unused
    pub async fn use_delegated_account(&self) -> Result<()> {
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

        // Update: Handle Option<LocalAccount>
        let mut faucet_account = self.faucet_account.write().await;
        *faucet_account = delegated_account;

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

    pub async fn process_with_account(
        &self,
        client: &Client,
        faucet_account: &mut LocalAccount,
        amount: u64,
        receiver_address: AccountAddress,
        check_only: bool,
        wait_for_transactions: bool,
    ) -> Result<Vec<SignedTransaction>, AptosTapError> {
        // For process_with_account, we don't use the shared outstanding_requests tracking
        // since we're working with a specific account instance

        if check_only {
            return Ok(vec![]);
        }

        let transaction_factory = self.get_transaction_factory().await?;
        let txn = faucet_account.sign_with_transaction_builder(transaction_factory.script(
            Script::new(MINTER_SCRIPT.to_vec(), vec![], vec![
                TransactionArgument::Address(receiver_address),
                TransactionArgument::U64(amount),
            ]),
        ));

        // Submit the transaction directly without using the shared account wrapper
        self.submit_transaction_direct(
            client,
            faucet_account,
            txn,
            &receiver_address,
            wait_for_transactions,
        )
        .await
        .map(|txn| vec![txn])
    }

    /// Submit a transaction using a direct LocalAccount reference (not wrapped in RwLock)
    async fn submit_transaction_direct(
        &self,
        client: &Client,
        faucet_account: &mut LocalAccount,
        signed_transaction: SignedTransaction,
        receiver_address: &AccountAddress,
        wait_for_transactions: bool,
    ) -> Result<SignedTransaction, AptosTapError> {
        let (result, event_on_success) = if wait_for_transactions {
            (
                client
                    .submit_and_wait_bcs(&signed_transaction)
                    .await
                    .map(|_| ())
                    .map_err(|e| {
                        AptosTapError::new_with_error_code(e, AptosTapErrorCode::TransactionFailed)
                    }),
                "transaction_submitted_and_waited",
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

        match result {
            Ok(_) => {
                // Increment sequence number
                faucet_account.increment_sequence_number();

                info!(
                    event = event_on_success,
                    sender_address = faucet_account.address(),
                    receiver_address = receiver_address,
                    txn_hash = signed_transaction.committed_hash(),
                );
                Ok(signed_transaction)
            }
            Err(e) => {
                // Decrement sequence number on failure to maintain correct sequence tracking
                faucet_account.decrement_sequence_number();
                info!(
                    event = "transaction_failed",
                    sender_address = faucet_account.address(),
                    receiver_address = receiver_address,
                    txn_hash = signed_transaction.committed_hash(),
                    error = %e,
                );
                Err(e)
            }
        }
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

        // 1. Resolve asset (default to "apt")
        let asset_name = asset.as_deref().unwrap_or("apt");

        // 2. Get asset config
        let asset_config = self.assets.get(asset_name)
            .ok_or_else(|| AptosTapError::new(
                format!("Asset '{}' is not configured", asset_name),
                AptosTapErrorCode::InvalidRequest,
            ))?;

        // 3. Create a dedicated account for this asset request (no shared state)
        let mut asset_account = self.create_faucet_account_for_asset(asset_config, asset_name).await?;

        let client = self.get_api_client();
        let amount = self.get_amount(amount, did_bypass_checkers);

        // Use the dedicated account for this request
        self.process_with_account(
            &client,
            &mut asset_account,
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
