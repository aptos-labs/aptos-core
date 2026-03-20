// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transaction options and local simulation support for the `aptos move simulate` command.

use crate::{local_simulation, MoveDebugger, MoveEnv};
// Re-export from aptos-cli-common to eliminate the duplicate definition.
pub use aptos_cli_common::ReplayProtectionType;
use aptos_cli_common::{
    get_account_with_state, CliError, CliTypedResult, EncodingOptions, GasOptions,
    PrivateKeyInputOptions, ProfileOptions, PromptOptions, RestOptions, TransactionSummary,
    ACCEPTED_CLOCK_SKEW_US, US_IN_SECS,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, hash::CryptoHash};
use aptos_rest_client::Client;
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{
        authenticator::{AccountAuthenticator, TransactionAuthenticator},
        PersistedAuxiliaryInfo, ReplayProtector, SignedTransaction, TransactionPayload,
        TransactionStatus,
    },
};
use aptos_vm_types::output::VMOutput;
use clap::Parser;
use move_core_types::vm_status::VMStatus;
use std::time::{SystemTime, UNIX_EPOCH};

/// Transaction options for the `Simulate` and `Replay` commands.
///
/// A lighter-weight alternative to `TransactionOptions` that provides local
/// simulation, benchmarking, gas profiling, and remote simulation capabilities
/// directly, without routing through `AptosContext`.
#[derive(Debug, Default, Parser)]
pub(crate) struct TxnOptions {
    /// Sender account address
    ///
    /// This allows you to override the account address from the derived account address
    /// in the event that the authentication key was rotated or for a resource account
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
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
    pub fn rest_client(&self) -> CliTypedResult<Client> {
        self.rest_options.client(&self.profile_options)
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
            deployed_object_address: None,
        })
    }

    /// Simulates a transaction locally, using the debugger to fetch required data from remote.
    async fn simulate_using_debugger<F>(
        &self,
        payload: TransactionPayload,
        env: &MoveEnv,
        execute: F,
    ) -> CliTypedResult<TransactionSummary>
    where
        F: FnOnce(
            &dyn MoveDebugger,
            u64,
            SignedTransaction,
            aptos_crypto::HashValue,
            PersistedAuxiliaryInfo,
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

        let debugger = env.create_move_debugger(client)?;
        let (vm_status, vm_output) = execute(
            &*debugger,
            version,
            transaction,
            hash,
            PersistedAuxiliaryInfo::None,
        )?;

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
            deployed_object_address: None,
        })
    }

    /// Simulates a transaction locally.
    pub async fn simulate_locally(
        &self,
        payload: TransactionPayload,
        env: &MoveEnv,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Simulating transaction locally...");

        self.simulate_using_debugger(
            payload,
            env,
            local_simulation::run_transaction_using_debugger,
        )
        .await
    }
}
