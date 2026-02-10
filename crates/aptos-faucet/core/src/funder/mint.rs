// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{FunderHealthMessage, FunderTrait};
use crate::endpoints::{AptosTapError, AptosTapErrorCode};
use anyhow::{Context, Result};
use aptos_logger::info;
use aptos_sdk::{
    crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    move_types::{identifier::Identifier, language_storage::ModuleId},
    rest_client::{AptosBaseUrl, Client},
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{
            authenticator::AuthenticationKey, EntryFunction, Script, SignedTransaction,
            TransactionArgument, TransactionPayload,
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
    submit_transaction, ApiConnectionConfig, AssetConfig, GasUnitPriceManager,
    TransactionSubmissionConfig, DEFAULT_AMOUNT_TO_FUND, DEFAULT_ASSET_NAME,
};

/// Entry function identifier containing module and function information.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct EntryFunctionId {
    /// Module address for the entry function
    pub module_address: AccountAddress,
    /// Module name for the entry function
    pub module_name: String,
    /// Function name for the entry function
    pub function_name: String,
}

/// Transaction method for minting coins.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransactionMethod {
    /// Use a script-based transaction (default)
    #[default]
    Script,
    /// Use an entry function transaction
    EntryFunction(EntryFunctionId),
}

/// Asset configuration specific to minting, extends the base AssetConfig with mint-specific fields.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MintAssetConfig {
    #[serde(flatten)]
    pub default: AssetConfig,

    /// Address of the account to send transactions from. On localnet, for
    /// example, this is a550c18. If not given, we use the account address
    /// corresponding to the given private key.
    pub mint_account_address: Option<AccountAddress>,

    /// Just use the account given in funder args, don't make a new one and
    /// delegate the mint capability to it.
    pub do_not_delegate: bool,

    /// Transaction method: script (default) or entry_function
    #[serde(default)]
    pub transaction_method: TransactionMethod,
}

impl MintAssetConfig {
    pub fn new(
        default: AssetConfig,
        mint_account_address: Option<AccountAddress>,
        do_not_delegate: bool,
    ) -> Self {
        Self {
            default,
            mint_account_address,
            do_not_delegate,
            transaction_method: TransactionMethod::default(),
        }
    }

    /// Delegate to the default AssetConfig's get_key method.
    pub fn get_key(&self) -> Result<Ed25519PrivateKey> {
        self.default.get_key()
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
    #[serde(default = "MintFunderConfig::get_default_asset_name")]
    pub default_asset: String,

    /// Default amount of coins to fund.
    /// If not provided, defaults to 100_000_000_000.
    #[serde(default = "MintFunderConfig::get_default_amount_to_fund")]
    pub amount_to_fund: u64,
}

impl MintFunderConfig {
    pub async fn build_funder(self) -> Result<MintFunder> {
        // Validate we have at least one asset
        if self.assets.is_empty() {
            return Err(anyhow::anyhow!("No assets configured"));
        }

        // Validate that the default asset exists in the assets map
        self.assets.get(&self.default_asset).ok_or_else(|| {
            anyhow::anyhow!(
                "Default asset '{}' is not configured in assets map",
                self.default_asset
            )
        })?;

        let mut assets_with_accounts = HashMap::new();

        for (asset_name, asset_config) in self.assets {
            let key = asset_config.get_key()?;

            // Create the mint account
            let mint_account = LocalAccount::new(
                asset_config.mint_account_address.unwrap_or_else(|| {
                    AuthenticationKey::ed25519(&Ed25519PublicKey::from(&key)).account_address()
                }),
                key,
                0,
            );

            assets_with_accounts.insert(asset_name, (asset_config, RwLock::new(mint_account)));
        }

        let minter = MintFunder::new(
            self.api_connection_config.node_url.clone(),
            self.api_connection_config.api_key.clone(),
            self.api_connection_config.additional_headers.clone(),
            self.api_connection_config.chain_id,
            self.transaction_submission_config,
            assets_with_accounts,
            self.default_asset,
            self.amount_to_fund,
        );

        let asset_names: Vec<String> = minter.assets.keys().cloned().collect();
        for asset_name in asset_names {
            let (asset_config, _) = minter
                .get_asset(&asset_name)
                .with_context(|| format!("Asset '{}' not found", asset_name))?;

            if !asset_config.do_not_delegate {
                // Delegate permissions to a new account
                let delegated_account = minter
                    .use_delegated_account(&asset_name)
                    .await
                    .with_context(|| {
                        format!("Failed to delegate account for asset '{}'", asset_name)
                    })?;

                // Update the account in the assets map
                minter
                    .update_asset_account(&asset_name, delegated_account)
                    .await
                    .with_context(|| {
                        format!("Failed to update asset account for '{}'", asset_name)
                    })?;
            }
        }

        Ok(minter)
    }

    pub fn get_default_asset_name() -> String {
        DEFAULT_ASSET_NAME.to_string()
    }

    fn get_default_amount_to_fund() -> u64 {
        DEFAULT_AMOUNT_TO_FUND
    }
}

pub struct MintFunder {
    /// URL of an Aptos node API.
    node_url: Url,
    node_api_key: Option<String>,
    node_additional_headers: Option<HashMap<String, String>>,

    txn_config: TransactionSubmissionConfig,

    transaction_factory: TransactionFactory,

    gas_unit_price_manager: GasUnitPriceManager,

    // Multi-asset support: store asset configs
    assets: HashMap<String, (MintAssetConfig, RwLock<LocalAccount>)>,
    default_asset: String,
    amount_to_fund: u64,
}

impl MintFunder {
    pub fn new(
        node_url: Url,
        node_api_key: Option<String>,
        node_additional_headers: Option<HashMap<String, String>>,
        chain_id: ChainId,
        txn_config: TransactionSubmissionConfig,
        assets: HashMap<String, (MintAssetConfig, RwLock<LocalAccount>)>,
        default_asset: String,
        amount_to_fund: u64,
    ) -> Self {
        let gas_unit_price_manager =
            GasUnitPriceManager::new(node_url.clone(), txn_config.get_gas_unit_price_ttl_secs());
        let transaction_factory = TransactionFactory::new(chain_id)
            .with_max_gas_amount(txn_config.max_gas_amount)
            .with_transaction_expiration_time(txn_config.transaction_expiration_secs)
            .with_use_replay_protection_nonce(true);
        Self {
            node_url,
            node_api_key,
            node_additional_headers,
            txn_config,
            transaction_factory,
            gas_unit_price_manager,
            assets,
            default_asset,
            amount_to_fund,
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

    /// Performs the delegated account creation and delegation. The (Aptos) coin::mint function that
    /// used in the MintFunder expects the caller to have the MintCapability.
    /// So we need to create a new account and delegate the MintCapability to it.
    pub async fn use_delegated_account(&self, asset_name: &str) -> Result<LocalAccount> {
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
            asset_name,
        )
        .await
        .context("Failed to create new account")?;

        // Build a transaction factory using the gas unit price from the
        // GasUnitPriceManager. This mostly ensures that we will build a
        // transaction with a gas unit price that will be accepted.
        let transaction_factory = self.get_transaction_factory().await?;

        // Delegate minting to the account
        {
            let faucet_account = self.get_asset_account(asset_name)?.read().await;
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
            "Successfully configured MintFunder to use delegated account for asset '{}': {}",
            asset_name,
            delegated_account.address()
        );

        Ok(delegated_account)
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

    /// Get the asset config and account for a given asset name.
    /// Returns an error if the asset doesn't exist (should never happen in normal operation).
    fn get_asset(
        &self,
        asset_name: &str,
    ) -> Result<&(MintAssetConfig, RwLock<LocalAccount>), AptosTapError> {
        self.assets.get(asset_name).ok_or_else(|| {
            AptosTapError::new(
                format!("Asset '{}' not found", asset_name),
                AptosTapErrorCode::InvalidRequest,
            )
        })
    }

    /// Get the account RwLock for a given asset name.
    /// Returns an error if the asset doesn't exist.
    fn get_asset_account(&self, asset_name: &str) -> Result<&RwLock<LocalAccount>, AptosTapError> {
        self.get_asset(asset_name).map(|(_, account)| account)
    }

    /// Get the asset config for a given asset name.
    /// Returns an error if the asset doesn't exist.
    fn get_asset_config(&self, asset_name: &str) -> Result<&MintAssetConfig, AptosTapError> {
        self.get_asset(asset_name).map(|(config, _)| config)
    }

    /// Update the account for a given asset name.
    /// This is useful after delegating mint capabilities to a new account.
    pub async fn update_asset_account(
        &self,
        asset_name: &str,
        new_account: LocalAccount,
    ) -> Result<()> {
        let account_rwlock = self
            .get_asset_account(asset_name)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        *account_rwlock.write().await = new_account;
        Ok(())
    }

    /// Core processing logic that handles transaction submission.
    pub async fn process(
        &self,
        client: &Client,
        amount: u64,
        receiver_address: AccountAddress,
        check_only: bool,
        wait_for_transactions: bool,
        asset_name: &str,
    ) -> Result<Vec<SignedTransaction>, AptosTapError> {
        if amount == 0 {
            let receiver_exists = client.get_account(receiver_address).await.is_ok();
            if receiver_exists {
                return Err(AptosTapError::new(
                    format!(
                        "Account {} already exists and amount asked for is 0",
                        receiver_address
                    ),
                    AptosTapErrorCode::InvalidRequest,
                ));
            }
        }

        if check_only {
            return Ok(vec![]);
        }

        let asset_config = self.get_asset_config(asset_name)?;
        let transaction_factory = self.get_transaction_factory().await?;

        let txn = {
            // Orderless transactions don't increment the sequence number, so a
            // read lock is sufficient.
            let faucet_account = self.get_asset_account(asset_name)?.read().await;

            let payload = match &asset_config.transaction_method {
                TransactionMethod::EntryFunction(entry_function_id) => {
                    // Create ModuleId from module_address and module_name
                    let module_id = ModuleId::new(
                        entry_function_id.module_address,
                        Identifier::new(entry_function_id.module_name.as_str()).map_err(|e| {
                            AptosTapError::new(
                                format!(
                                    "Invalid module_name '{}': {}",
                                    entry_function_id.module_name, e
                                ),
                                AptosTapErrorCode::InvalidRequest,
                            )
                        })?,
                    );

                    // Create function identifier
                    let function_identifier =
                        Identifier::new(entry_function_id.function_name.as_str()).map_err(|e| {
                            AptosTapError::new(
                                format!(
                                    "Invalid function_name '{}': {}",
                                    entry_function_id.function_name, e
                                ),
                                AptosTapErrorCode::InvalidRequest,
                            )
                        })?;

                    // Serialize arguments (receiver_address and amount)
                    use aptos_sdk::bcs;
                    let args = vec![
                        bcs::to_bytes(&receiver_address).map_err(|e| {
                            AptosTapError::new(
                                format!("Failed to serialize receiver_address: {}", e),
                                AptosTapErrorCode::InvalidRequest,
                            )
                        })?,
                        bcs::to_bytes(&amount).map_err(|e| {
                            AptosTapError::new(
                                format!("Failed to serialize amount: {}", e),
                                AptosTapErrorCode::InvalidRequest,
                            )
                        })?,
                    ];

                    let entry_function =
                        EntryFunction::new(module_id, function_identifier, vec![], args);

                    TransactionPayload::EntryFunction(entry_function)
                },
                TransactionMethod::Script => {
                    // Default script-based approach
                    TransactionPayload::Script(Script::new(MINTER_SCRIPT.to_vec(), vec![], vec![
                        TransactionArgument::Address(receiver_address),
                        TransactionArgument::U64(amount),
                    ]))
                },
            };

            faucet_account.sign_with_transaction_builder(transaction_factory.payload(payload))
        };

        Ok(vec![
            submit_transaction(client, txn, &receiver_address, wait_for_transactions).await?,
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

        // Validate asset exists
        self.get_asset_config(asset_name)?;

        let client = self.get_api_client();
        let amount = self.get_amount(amount, did_bypass_checkers);
        self.process(
            &client,
            amount,
            receiver_address,
            check_only,
            self.txn_config.wait_for_transactions,
            asset_name,
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
        let account_address = match self.get_asset_account(&self.default_asset) {
            Ok(account) => account.read().await.address(),
            Err(e) => {
                return FunderHealthMessage {
                    can_process_requests: false,
                    message: Some(format!(
                        "Default asset '{}' not found: {}",
                        self.default_asset, e
                    )),
                };
            },
        };
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
