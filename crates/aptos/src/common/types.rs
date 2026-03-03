// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// ── Re-exports from aptos-cli-common ──
//
// These types were previously duplicated locally. They now live in `aptos-cli-common`
// and are re-exported here so that existing `use crate::common::types::...` paths
// continue to work.
pub use aptos_cli_common::{
    // Functions
    account_address_from_auth_key,
    account_address_from_public_key,
    get_mint_site_url,
    load_account_arg,
    load_manifest_account_arg,
    // Option structs
    AccountAddressWrapper,
    // Enums
    AccountType,
    AuthenticationKeyInputOptions,
    // Data types
    ChangeSummary,
    // Traits
    CliCommand,
    // Config types
    CliConfig,
    // Error type
    CliError,
    // Type aliases
    CliResult,
    CliTypedResult,
    ConfigSearchMode,
    EncodingOptions,
    ExtractEd25519PublicKey,
    FaucetOptions,
    GasOptions,
    HardwareWalletOptions,
    KeyType,
    MoveManifestAccountWrapper,
    MultisigAccount,
    MultisigAccountWithSequenceNumber,
    OptionalPoolAddressArgs,
    ParseEd25519PrivateKey,
    PoolAddressArgs,
    PrivateKeyInputOptions,
    ProfileConfig,
    ProfileOptions,
    ProfileSummary,
    PromptOptions,
    PublicKeyInputOptions,
    ReplayProtectionType,
    RestOptions,
    RngArgs,
    SaveFile,
    TransactionOptions,
    TransactionSummary,
    // Constants
    ACCEPTED_CLOCK_SKEW_US,
    APTOS_FOLDER_GIT_IGNORE,
    CONFIG_FOLDER,
    DEFAULT_EXPIRATION_SECS,
    DEFAULT_PROFILE,
    GIT_IGNORE,
    MOVE_FOLDER_GIT_IGNORE,
    US_IN_SECS,
};
use aptos_cli_common::{
    explorer_transaction_link, get_account_with_state, get_auth_key, get_sequence_number,
    prompt_yes_with_override, Network,
};
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_global_constants::adjust_gas_headroom;
use aptos_rest_client::{aptos_api_types::ViewFunction, Transaction};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{HardwareWalletAccount, HardwareWalletType, LocalAccount, TransactionSigner},
};
use aptos_transaction_simulation::SimulationStateStore;
use aptos_transaction_simulation_session::Session;
use aptos_types::{
    account_config::AccountResource,
    chain_id::ChainId,
    transaction::{authenticator::AuthenticationKey, SignedTransaction, TransactionPayload},
};
use async_trait::async_trait;
use move_core_types::account_address::AccountAddress;
use std::{
    cmp::max,
    convert::TryFrom,
    time::{SystemTime, UNIX_EPOCH},
};

/// User-agent string for the full Aptos CLI.
pub const USER_AGENT: &str = concat!("aptos-cli/", env!("CARGO_PKG_VERSION"));

// ── Extension trait for TransactionOptions ──
//
// These methods live in the full `aptos` CLI crate because they depend on heavy
// crates (debugger, session, SDK signers) that `aptos-cli-common` intentionally
// does not pull in. The methods provide chain submission, local simulation, and
// view-function support.

#[async_trait]
pub trait TransactionOptionsExt {
    /// Submit a transaction to the chain, returning the confirmed `Transaction`.
    async fn submit_transaction(&self, payload: TransactionPayload) -> CliTypedResult<Transaction>;

    /// Fetch the authentication key for an account from the REST API.
    async fn auth_key(&self, sender_address: AccountAddress) -> CliTypedResult<AuthenticationKey>;

    /// Fetch the sequence number (or derive it from a local session).
    async fn sequence_number(&self, sender_address: AccountAddress) -> CliTypedResult<u64>;

    /// Execute a view function and return the JSON results.
    async fn view(&self, payload: ViewFunction) -> CliTypedResult<Vec<serde_json::Value>>;
}

#[async_trait]
impl TransactionOptionsExt for TransactionOptions {
    async fn submit_transaction(&self, payload: TransactionPayload) -> CliTypedResult<Transaction> {
        let client = self.rest_client()?;
        let (sender_public_key, sender_address) = self.get_public_key_and_address()?;

        // Ask to confirm price if the gas unit price is estimated above the lowest value when
        // it is automatically estimated
        let ask_to_confirm_price;
        let gas_unit_price = if let Some(gas_unit_price) = self.gas_options.gas_unit_price {
            ask_to_confirm_price = false;
            gas_unit_price
        } else {
            let gas_unit_price = client.estimate_gas_price().await?.into_inner().gas_estimate;

            ask_to_confirm_price = true;
            gas_unit_price
        };

        // Get sequence number for account
        let (account, state) = get_account_with_state(&client, sender_address).await?;
        let sequence_number = account.sequence_number;

        // Retrieve local time, and ensure it's within an expected skew of the blockchain
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| CliError::UnexpectedError(err.to_string()))?
            .as_secs();
        let now_usecs = now * US_IN_SECS;

        // Warn local user that clock is skewed behind the blockchain.
        // There will always be a little lag from real time to blockchain time
        if now_usecs < state.timestamp_usecs - ACCEPTED_CLOCK_SKEW_US {
            eprintln!("Local clock is is skewed from blockchain clock.  Clock is more than {} seconds behind the blockchain {}", ACCEPTED_CLOCK_SKEW_US, state.timestamp_usecs / US_IN_SECS );
        }
        let expiration_time_secs = now + self.gas_options.expiration_secs;

        let chain_id = ChainId::new(state.chain_id);
        // TODO: Check auth key against current private key and provide a better message

        let max_gas = if let Some(max_gas) = self.gas_options.max_gas {
            // If the gas unit price was estimated ask, but otherwise you've chosen hwo much you want to spend
            if ask_to_confirm_price {
                let message = format!("Do you want to submit transaction for a maximum of {} Octas at a gas unit price of {} Octas?",  max_gas * gas_unit_price, gas_unit_price);
                prompt_yes_with_override(&message, self.prompt_options)?;
            }
            max_gas
        } else {
            let transaction_factory =
                TransactionFactory::new(chain_id).with_gas_unit_price(gas_unit_price);

            let txn_builder = transaction_factory
                .payload(payload.clone())
                .sender(sender_address)
                .sequence_number(sequence_number)
                .expiration_timestamp_secs(expiration_time_secs);

            let unsigned_transaction = if self.replay_protection_type == ReplayProtectionType::Nonce
            {
                let mut rng = rand::thread_rng();
                txn_builder
                    .upgrade_payload_with_rng(&mut rng, true, true)
                    .build()
            } else {
                txn_builder.build()
            };

            let signed_transaction = SignedTransaction::new(
                unsigned_transaction,
                sender_public_key.clone(),
                Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
            );

            let txns = client
                .simulate_with_gas_estimation(&signed_transaction, true, false)
                .await?
                .into_inner();
            let simulated_txn = txns.first().unwrap();

            // Check if the transaction will pass, if it doesn't then fail
            if !simulated_txn.info.success {
                return Err(CliError::SimulationError(
                    simulated_txn.info.vm_status.clone(),
                ));
            }

            // Take the gas used and use a headroom factor on it
            let gas_used = simulated_txn.info.gas_used.0;
            // TODO: remove the hardcoded 530 as it's the minumum gas units required for the transaction that will
            // automatically create an account for stateless account.
            let adjusted_max_gas =
                adjust_gas_headroom(gas_used, max(simulated_txn.request.max_gas_amount.0, 530));

            // Ask if you want to accept the estimate amount
            let upper_cost_bound = adjusted_max_gas * gas_unit_price;
            let lower_cost_bound = gas_used * gas_unit_price;
            let message = format!(
                    "Do you want to submit a transaction for a range of [{} - {}] Octas at a gas unit price of {} Octas?",
                    lower_cost_bound,
                    upper_cost_bound,
                    gas_unit_price);
            prompt_yes_with_override(&message, self.prompt_options)?;
            adjusted_max_gas
        };

        // Build a transaction
        let transaction_factory = TransactionFactory::new(chain_id)
            .with_gas_unit_price(gas_unit_price)
            .with_max_gas_amount(max_gas)
            .with_transaction_expiration_time(self.gas_options.expiration_secs);

        // Sign it with the appropriate signer
        let transaction = match self.get_transaction_account_type() {
            Ok(AccountType::Local) => {
                let (private_key, _) = self.get_key_and_address()?;
                let sender_account =
                    &mut LocalAccount::new(sender_address, private_key, sequence_number);
                let mut txn_builder = transaction_factory.payload(payload);
                if self.replay_protection_type == ReplayProtectionType::Nonce {
                    let mut rng = rand::thread_rng();
                    txn_builder = txn_builder.upgrade_payload_with_rng(&mut rng, true, true);
                };
                sender_account.sign_with_transaction_builder(txn_builder)
            },
            Ok(AccountType::HardwareWallet) => {
                let sender_account = &mut HardwareWalletAccount::new(
                    sender_address,
                    sender_public_key,
                    self.profile_options
                        .derivation_path()
                        .expect("derivative path is missing from profile")
                        .unwrap(),
                    HardwareWalletType::Ledger,
                    sequence_number,
                );
                let mut txn_builder = transaction_factory.payload(payload);
                if self.replay_protection_type == ReplayProtectionType::Nonce {
                    let mut rng = rand::thread_rng();
                    txn_builder = txn_builder.upgrade_payload_with_rng(&mut rng, true, true);
                };
                sender_account.sign_with_transaction_builder(txn_builder)?
            },
            Err(err) => return Err(err),
        };

        // Submit the transaction, printing out a useful transaction link
        client
            .submit_bcs(&transaction)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?;
        let transaction_hash = transaction.clone().committed_hash();
        let network = self.profile_options.profile().ok().and_then(|profile| {
            if let Some(network) = profile.network {
                Some(network)
            } else {
                // Approximate network from URL
                match profile.rest_url {
                    None => None,
                    Some(url) => {
                        if url.contains("mainnet") {
                            Some(Network::Mainnet)
                        } else if url.contains("testnet") {
                            Some(Network::Testnet)
                        } else if url.contains("devnet") {
                            Some(Network::Devnet)
                        } else if url.contains("localhost") || url.contains("127.0.0.1") {
                            Some(Network::Local)
                        } else {
                            None
                        }
                    },
                }
            }
        });
        eprintln!(
            "Transaction submitted: {}",
            explorer_transaction_link(transaction_hash, network)
        );
        let response = client
            .wait_for_signed_transaction(&transaction)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?;

        Ok(response.into_inner())
    }

    /// Gets the auth key by account address. We need to fetch the auth key from Rest API rather than creating an
    /// auth key out of the public key.
    async fn auth_key(&self, sender_address: AccountAddress) -> CliTypedResult<AuthenticationKey> {
        let client = self.rest_client()?;
        get_auth_key(&client, sender_address).await
    }

    async fn sequence_number(&self, sender_address: AccountAddress) -> CliTypedResult<u64> {
        match &self.session {
            None => {
                let client = self.rest_client()?;
                get_sequence_number(&client, sender_address).await
            },
            Some(session_path) => {
                let sess = Session::load(session_path)?;

                let account = sess
                    .state_store()
                    .get_resource::<AccountResource>(sender_address)?;
                let seq_num = account.map(|account| account.sequence_number).unwrap_or(0);

                Ok(seq_num)
            },
        }
    }

    async fn view(&self, payload: ViewFunction) -> CliTypedResult<Vec<serde_json::Value>> {
        match &self.session {
            None => {
                let client = self.rest_client()?;
                Ok(client
                    .view_bcs_with_json_response(&payload, None)
                    .await?
                    .into_inner())
            },

            Some(session_path) => {
                let mut sess = Session::load(session_path)?;
                let output = sess.execute_view_function(
                    payload.module,
                    payload.function,
                    payload.ty_args,
                    payload.args,
                    false,
                )?;
                Ok(output)
            },
        }
    }
}
