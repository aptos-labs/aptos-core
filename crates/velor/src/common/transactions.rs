// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    local_simulation,
    types::{
        AccountType, CliConfig, CliError, CliTypedResult, ConfigSearchMode, EncodingOptions,
        ExtractEd25519PublicKey, GasOptions, PrivateKeyInputOptions, ProfileOptions, PromptOptions,
        RestOptions, TransactionSummary, ACCEPTED_CLOCK_SKEW_US, US_IN_SECS,
    },
    utils::{get_account_with_state, get_sequence_number},
};
use velor_api_types::ViewFunction;
use velor_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    hash::CryptoHash,
};
use velor_move_debugger::velor_debugger::VelorDebugger;
use velor_rest_client::Client;
use velor_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use velor_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{
        authenticator::{AccountAuthenticator, TransactionAuthenticator},
        ReplayProtector, SignedTransaction, TransactionPayload, TransactionStatus,
    },
};
use velor_vm_types::output::VMOutput;
use clap::Parser;
use move_core_types::vm_status::VMStatus;
pub use move_package::*;
use std::{
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Default, Parser, PartialEq, clap::ValueEnum)]
pub enum ReplayProtectionType {
    Nonce,
    #[default]
    Seqnum,
}

impl Display for ReplayProtectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ReplayProtectionType::Nonce => "nonce",
            ReplayProtectionType::Seqnum => "seqnum",
        })
    }
}

/// Common options for simulating and running transactions
///
/// NOTE: this should be separated out from the existing TransactionOptions, which is now too big
///
/// TODO: I know this is a copy of the existing one, but it should streamline a lot of the code.  Currently experimental without any worries of backwards compatibility
#[derive(Debug, Default, Parser)]
pub struct TxnOptions {
    /// Sender account address
    ///
    /// This allows you to override the account address from the derived account address
    /// in the event that the authentication key was rotated or for a resource account
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) sender_account: Option<AccountAddress>,

    #[clap(flatten)]
    pub(crate) private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    pub(crate) encoding_options: EncodingOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) gas_options: GasOptions,
    #[clap(flatten)]
    pub prompt_options: PromptOptions,
    /// Replay protection mechanism to use when generating the transaction.
    ///
    /// When "nonce" is chosen, the transaction will be an orderless transaction and contains a replay protection nonce.
    ///
    /// When "seqnum" is chosen, the transaction will contain a sequence number that matches with the sender's onchain sequence number.
    #[clap(long, default_value_t = ReplayProtectionType::Seqnum)]
    pub(crate) replay_protection_type: ReplayProtectionType,
}

impl TxnOptions {
    /// Builds a rest client
    fn rest_client(&self) -> CliTypedResult<Client> {
        self.rest_options.client(&self.profile_options)
    }

    pub fn get_transaction_account_type(&self) -> CliTypedResult<AccountType> {
        if self.private_key_options.has_key_or_file() {
            Ok(AccountType::Local)
        } else if let Some(profile) = CliConfig::load_profile(
            self.profile_options.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )? {
            if profile.private_key.is_some() {
                Ok(AccountType::Local)
            } else {
                Ok(AccountType::HardwareWallet)
            }
        } else {
            Err(CliError::CommandArgumentError(
                "One of ['--private-key', '--private-key-file'] or a profile must be used"
                    .to_string(),
            ))
        }
    }

    /// Retrieves the private key and the associated address
    /// TODO: Cache this information
    pub fn get_key_and_address(&self) -> CliTypedResult<(Ed25519PrivateKey, AccountAddress)> {
        self.private_key_options.extract_private_key_and_address(
            self.encoding_options.encoding,
            &self.profile_options,
            self.sender_account,
        )
    }

    pub fn get_address(&self) -> CliTypedResult<AccountAddress> {
        self.private_key_options.extract_address(
            self.encoding_options.encoding,
            &self.profile_options,
            self.sender_account,
        )
    }

    pub fn get_public_key_and_address(&self) -> CliTypedResult<(Ed25519PublicKey, AccountAddress)> {
        self.private_key_options
            .extract_ed25519_public_key_and_address(
                self.encoding_options.encoding,
                &self.profile_options,
                self.sender_account,
            )
    }

    pub fn sender_address(&self) -> CliTypedResult<AccountAddress> {
        Ok(self.get_key_and_address()?.1)
    }

    pub fn get_public_key(&self) -> CliTypedResult<Ed25519PublicKey> {
        self.private_key_options
            .extract_public_key(self.encoding_options.encoding, &self.profile_options)
    }

    pub async fn sequence_number(&self, sender_address: AccountAddress) -> CliTypedResult<u64> {
        let client = self.rest_client()?;
        get_sequence_number(&client, sender_address).await
    }

    pub async fn view(&self, payload: ViewFunction) -> CliTypedResult<Vec<serde_json::Value>> {
        let client = self.rest_client()?;
        Ok(client
            .view_bcs_with_json_response(&payload, None)
            .await?
            .into_inner())
    }

    pub async fn simulate_remotely(
        &self,
        rng: &mut rand::rngs::StdRng,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary> {
        let client = self.rest_client()?;
        let sender_address = self.get_address()?;

        let gas_unit_price = if let Some(gas_unit_price) = self.gas_options.gas_unit_price {
            gas_unit_price
        } else {
            client.estimate_gas_price().await?.into_inner().gas_estimate
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

        let transaction_factory =
            TransactionFactory::new(chain_id).with_gas_unit_price(gas_unit_price);

        let mut txn_builder = transaction_factory
            .payload(payload.clone())
            .sender(sender_address)
            .sequence_number(sequence_number)
            .expiration_timestamp_secs(expiration_time_secs);
        if self.replay_protection_type == ReplayProtectionType::Nonce {
            txn_builder = txn_builder.upgrade_payload_with_rng(rng, true, true);
        }
        let unsigned_transaction = txn_builder.build();

        // TODO: Support other transaction authenticator types, like multi-agent and fee-payer
        let signed_transaction = SignedTransaction::new_signed_transaction(
            unsigned_transaction,
            TransactionAuthenticator::SingleSender {
                sender: AccountAuthenticator::NoAccountAuthenticator,
            },
        );

        let simulated_txn = client
            .simulate_bcs_with_gas_estimation(&signed_transaction, true, false)
            .await?
            .into_inner();

        let user_txn = simulated_txn.transaction.try_as_signed_user_txn().unwrap();

        // TODO: add events and outputs
        Ok(TransactionSummary {
            transaction_hash: simulated_txn.info.hash().into(),
            gas_used: Some(simulated_txn.info.gas_used()),
            gas_unit_price: Some(user_txn.gas_unit_price()),
            pending: None,
            sender: Some(user_txn.sender()),
            replay_protector: Some(user_txn.replay_protector()),
            sequence_number: match user_txn.replay_protector() {
                ReplayProtector::SequenceNumber(sequence_number) => Some(sequence_number),
                _ => None,
            },
            success: Some(simulated_txn.info.status().is_success()),
            timestamp_us: None,
            version: Some(simulated_txn.version),
            vm_status: Some(format!("{:?}", simulated_txn.info.status())), // TODO: add proper status
        })
    }

    /// Simulates a transaction locally, using the debugger to fetch required data from remote.
    async fn simulate_using_debugger<F>(
        &self,
        payload: TransactionPayload,
        execute: F,
    ) -> CliTypedResult<TransactionSummary>
    where
        F: FnOnce(
            &VelorDebugger,
            u64,
            SignedTransaction,
            velor_crypto::HashValue,
        ) -> CliTypedResult<(VMStatus, VMOutput)>,
    {
        let client = self.rest_client()?;

        // Fetch the chain states required for the simulation
        // TODO(Gas): get the following from the chain
        const DEFAULT_GAS_UNIT_PRICE: u64 = 100;
        const DEFAULT_MAX_GAS: u64 = 2_000_000;

        let (sender_key, sender_address) = self.get_key_and_address()?;
        let gas_unit_price = self
            .gas_options
            .gas_unit_price
            .unwrap_or(DEFAULT_GAS_UNIT_PRICE);
        let (account, state) = get_account_with_state(&client, sender_address).await?;
        let version = state.version;
        let chain_id = ChainId::new(state.chain_id);
        let sequence_number = account.sequence_number;

        let balance = client
            .view_apt_account_balance_at_version(sender_address, version)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?
            .into_inner();

        let max_gas = self.gas_options.max_gas.unwrap_or_else(|| {
            if gas_unit_price == 0 {
                DEFAULT_MAX_GAS
            } else {
                std::cmp::min(balance / gas_unit_price, DEFAULT_MAX_GAS)
            }
        });

        let transaction_factory = TransactionFactory::new(chain_id)
            .with_gas_unit_price(gas_unit_price)
            .with_max_gas_amount(max_gas)
            .with_transaction_expiration_time(self.gas_options.expiration_secs);
        let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
        let transaction =
            sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
        let hash = transaction.committed_hash();

        let debugger = VelorDebugger::rest_client(client)?;
        let (vm_status, vm_output) = execute(&debugger, version, transaction, hash)?;

        let success = match vm_output.status() {
            TransactionStatus::Keep(exec_status) => Some(exec_status.is_success()),
            TransactionStatus::Discard(_) | TransactionStatus::Retry => None,
        };

        Ok(TransactionSummary {
            transaction_hash: hash.into(),
            gas_used: Some(vm_output.gas_used()),
            gas_unit_price: Some(gas_unit_price),
            pending: None,
            sender: Some(sender_address),
            sequence_number: None,
            replay_protector: None, // The transaction is not committed so there is no new sequence number.
            success,
            timestamp_us: None,
            version: Some(version), // The transaction is not committed so there is no new version.
            vm_status: Some(vm_status.to_string()),
        })
    }

    /// Simulates a transaction locally.
    pub async fn simulate_locally(
        &self,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Simulating transaction locally...");

        self.simulate_using_debugger(payload, local_simulation::run_transaction_using_debugger)
            .await
    }

    /// Benchmarks the transaction payload locally.
    /// The transaction is executed multiple times, and the median value is calculated to improve
    /// the accuracy of the measurement results.
    pub async fn benchmark_locally(
        &self,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Benchmarking transaction locally...");

        self.simulate_using_debugger(
            payload,
            local_simulation::benchmark_transaction_using_debugger,
        )
        .await
    }

    /// Simulates the transaction locally with the gas profiler enabled.
    pub async fn profile_gas(
        &self,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Simulating transaction locally using the gas profiler...");

        self.simulate_using_debugger(
            payload,
            local_simulation::profile_transaction_using_debugger,
        )
        .await
    }

    pub async fn estimate_gas_price(&self) -> CliTypedResult<u64> {
        let client = self.rest_client()?;
        client
            .estimate_gas_price()
            .await
            .map(|inner| inner.into_inner().gas_estimate)
            .map_err(|err| {
                CliError::UnexpectedError(format!(
                    "Failed to retrieve gas price estimate {:?}",
                    err
                ))
            })
    }
}
