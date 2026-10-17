// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of [`aptos_move_cli::AptosContext`] for the full Aptos CLI.
//!
//! This bridges `aptos-move-cli` commands (which use `aptos_cli_common::TransactionOptions`)
//! to the full Aptos CLI environment for network operations, local simulation,
//! gas profiling, benchmarking, and session-based execution.

use aptos_cli_common::{
    explorer_transaction_link, format_txn_status, get_account_with_state, prompt_yes_with_override,
    AccountType, CliError, CliTypedResult, Network, PromptOptions, ReplayProtectionType,
    TransactionOptions, TransactionSummary, ACCEPTED_CLOCK_SKEW_US, US_IN_SECS,
};
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_global_constants::adjust_gas_headroom;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::{Client, State};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{HardwareWalletAccount, HardwareWalletType, LocalAccount, TransactionSigner},
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    chain_id::ChainId,
    transaction::{
        PersistedAuxiliaryInfo, ReplayProtector, SignedTransaction, TransactionPayload,
        TransactionStatus,
    },
};
use aptos_vm_types::output::VMOutput;
use async_trait::async_trait;
use futures::future::join_all;
use move_core_types::vm_status::VMStatus;
use std::{
    cmp::max,
    time::{SystemTime, UNIX_EPOCH},
};

/// Real implementation of [`aptos_move_cli::AptosContext`] for the full Aptos CLI.
///
/// Routes transactions to the appropriate backend: local simulation, benchmarking,
/// gas profiling, session-based execution, or remote chain submission.
pub struct RealAptosContext;

#[async_trait]
impl aptos_move_cli::AptosContext for RealAptosContext {
    async fn submit_transaction(
        &self,
        options: &TransactionOptions,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary> {
        // Validation
        if options.profile_gas && options.benchmark {
            return Err(CliError::UnexpectedError(
                "Cannot perform benchmarking and gas profiling at the same time.".to_string(),
            ));
        }
        if options.session.is_some() && options.profile_gas {
            return Err(CliError::UnexpectedError(
                "`--profile-gas` cannot be used with `--session` yet".to_string(),
            ));
        }
        if options.session.is_some() && options.benchmark {
            return Err(CliError::UnexpectedError(
                "`--benchmark` cannot be used with `--session` yet".to_string(),
            ));
        }

        // Route to the appropriate backend.
        if let Some(session_path) = &options.session {
            simulate_using_session(options, session_path, payload).await
        } else if options.profile_gas {
            println!();
            println!("Simulating transaction locally using the gas profiler...");
            let fold_unique_stack = options.fold_unique_stack;
            let skip_gas_profiler_consistency_check = options.skip_gas_profiler_consistency_check;
            simulate_using_debugger(
                options,
                payload,
                |debugger, version, txn, hash, aux_info| {
                    aptos_move_cli::local_simulation::profile_transaction_using_debugger(
                        debugger,
                        version,
                        txn,
                        hash,
                        aux_info,
                        fold_unique_stack,
                        skip_gas_profiler_consistency_check,
                    )
                },
            )
            .await
        } else if options.benchmark {
            println!();
            println!("Benchmarking transaction locally...");
            simulate_using_debugger(
                options,
                payload,
                |debugger, version, txn, hash, aux_info| {
                    aptos_move_cli::local_simulation::benchmark_transaction_using_debugger(
                        debugger, version, txn, hash, aux_info,
                    )
                },
            )
            .await
        } else if options.local {
            println!();
            println!("Simulating transaction locally...");
            simulate_using_debugger(
                options,
                payload,
                |debugger, version, txn, hash, aux_info| {
                    aptos_move_cli::local_simulation::run_transaction_using_debugger(
                        debugger, version, txn, hash, aux_info,
                    )
                },
            )
            .await
        } else {
            submit_to_chain(options, payload).await
        }
    }

    async fn submit_transactions(
        &self,
        options: &TransactionOptions,
        payloads: Vec<TransactionPayload>,
    ) -> Vec<CliTypedResult<TransactionSummary>> {
        // Only a chain has sequence numbers that can be assigned ahead of the transactions, and
        // gains from not waiting for each of them. Everything else executes one at a time.
        let sends_to_chain = options.session.is_none()
            && !options.profile_gas
            && !options.benchmark
            && !options.local;
        if sends_to_chain && payloads.len() > 1 {
            submit_many_to_chain(options, payloads)
                .await
                .unwrap_or_else(|error| vec![Err(error)])
        } else {
            aptos_move_cli::submit_transactions_in_order(self, options, payloads).await
        }
    }

    async fn view(
        &self,
        options: &TransactionOptions,
        request: aptos_rest_client::aptos_api_types::ViewFunction,
    ) -> CliTypedResult<Vec<serde_json::Value>> {
        match &options.session {
            Some(session_path) => {
                let mut sess = aptos_transaction_simulation_session::Session::load(session_path)?;
                let output = sess.execute_view_function(
                    request.module,
                    request.function,
                    request.ty_args,
                    request.args,
                    options.profile_gas,
                    options.skip_gas_profiler_consistency_check,
                )?;
                Ok(output)
            },
            None => {
                let client = options.rest_client()?;
                Ok(client
                    .view_bcs_with_json_response(&request, None)
                    .await
                    .map_err(|err| CliError::ApiError(err.to_string()))?
                    .into_inner())
            },
        }
    }
}

// ── Local simulation using AptosDebugger ──

async fn simulate_using_debugger<F>(
    options: &TransactionOptions,
    payload: TransactionPayload,
    execute: F,
) -> CliTypedResult<TransactionSummary>
where
    F: FnOnce(
        &AptosDebugger,
        u64,
        SignedTransaction,
        aptos_crypto::HashValue,
        PersistedAuxiliaryInfo,
    ) -> CliTypedResult<(VMStatus, VMOutput)>,
{
    let client = options.rest_client()?;

    const DEFAULT_GAS_UNIT_PRICE: u64 = 100;
    const DEFAULT_MAX_GAS: u64 = 2_000_000;

    let (sender_key, sender_address) = options.get_key_and_address()?;
    let gas_unit_price = options
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

    let max_gas = options.gas_options.max_gas.unwrap_or_else(|| {
        if gas_unit_price == 0 {
            DEFAULT_MAX_GAS
        } else {
            std::cmp::min(balance / gas_unit_price, DEFAULT_MAX_GAS)
        }
    });

    let transaction_factory = TransactionFactory::new(chain_id)
        .with_gas_unit_price(gas_unit_price)
        .with_max_gas_amount(max_gas)
        .with_transaction_expiration_time(options.gas_options.expiration_secs);
    let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
    let transaction =
        sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
    let hash = transaction.committed_hash();

    let debugger = AptosDebugger::rest_client(client)?;
    let (vm_status, vm_output) = execute(
        &debugger,
        version,
        transaction,
        hash,
        PersistedAuxiliaryInfo::V1 {
            transaction_index: 0,
        },
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
        replay_protector: None,
        success,
        timestamp_us: None,
        version: Some(version),
        vm_status: Some(format_txn_status(vm_output.status(), &vm_status)),
        deployed_object_address: None,
        events: None,
        changes: None,
    })
}

// ── Session-based local simulation ──

async fn simulate_using_session(
    options: &TransactionOptions,
    session_path: &std::path::Path,
    payload: TransactionPayload,
) -> CliTypedResult<TransactionSummary> {
    use aptos_transaction_simulation::SimulationStateStore;
    use aptos_transaction_simulation_session::Session;

    let mut sess = Session::load(session_path)?;
    let state_store = sess.state_store();

    const DEFAULT_GAS_UNIT_PRICE: u64 = 100;
    const DEFAULT_MAX_GAS: u64 = 2_000_000;

    let (sender_key, sender_address) = options.get_key_and_address()?;

    let account = state_store.get_resource::<AccountResource>(sender_address)?;
    let seq_num = account.map(|a| a.sequence_number).unwrap_or(0);

    let gas_unit_price = options
        .gas_options
        .gas_unit_price
        .unwrap_or(DEFAULT_GAS_UNIT_PRICE);
    let balance = state_store.get_apt_balance(sender_address)?;
    let max_gas = options.gas_options.max_gas.unwrap_or_else(|| {
        if gas_unit_price == 0 {
            DEFAULT_MAX_GAS
        } else {
            std::cmp::min(balance / gas_unit_price, DEFAULT_MAX_GAS)
        }
    });

    let transaction_factory = TransactionFactory::new(state_store.get_chain_id()?)
        .with_gas_unit_price(gas_unit_price)
        .with_max_gas_amount(max_gas)
        .with_transaction_expiration_time(options.gas_options.expiration_secs);
    let sender_account = &mut LocalAccount::new(sender_address, sender_key, seq_num);
    let transaction =
        sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
    let hash = transaction.committed_hash();

    let (vm_status, txn_output) = sess.execute_transaction(transaction, false, false)?;

    let success = match txn_output.status() {
        TransactionStatus::Keep(exec_status) => Some(exec_status.is_success()),
        TransactionStatus::Discard(_) | TransactionStatus::Retry => None,
    };

    Ok(TransactionSummary {
        transaction_hash: hash.into(),
        gas_used: Some(txn_output.gas_used()),
        gas_unit_price: Some(gas_unit_price),
        pending: None,
        sender: Some(sender_address),
        sequence_number: Some(seq_num),
        replay_protector: None,
        success,
        timestamp_us: None,
        version: None,
        vm_status: Some(format_txn_status(txn_output.status(), &vm_status)),
        deployed_object_address: None,
        events: None,
        changes: None,
    })
}

// ── Remote chain submission ──

/// What it takes to estimate, sign and submit the transactions of the sender of `options`.
struct ChainSender<'a> {
    options: &'a TransactionOptions,
    client: Client,
    public_key: Ed25519PublicKey,
    address: AccountAddress,
    chain_id: ChainId,
    gas_unit_price: u64,
    /// When the transactions that are built now expire.
    expiration_time_secs: u64,
}

/// The gas that a transaction is estimated to need.
struct GasEstimate {
    gas_used: u64,
    max_gas: u64,
}

/// The calls to a chain that submitting a group of transactions makes, and the signing. The group
/// logic only knows this, which is what lets the order and the concurrency of the calls be tested
/// without a node.
#[async_trait]
trait GroupChain {
    /// Simulates `payload` as the transaction of the sender with this sequence number, to estimate
    /// the gas that it needs. Fails if the simulation fails.
    async fn estimate_gas(
        &self,
        payload: TransactionPayload,
        sequence_number: u64,
    ) -> CliTypedResult<GasEstimate>;

    /// Signs `payload` as the transaction of the sender with this sequence number.
    fn sign(
        &self,
        payload: TransactionPayload,
        sequence_number: u64,
        max_gas: u64,
    ) -> CliTypedResult<SignedTransaction>;

    /// Submits a signed transaction, without waiting for it to be committed.
    async fn submit(&self, transaction: &SignedTransaction) -> CliTypedResult<()>;

    /// Waits for a transaction that was submitted to be committed.
    async fn wait(&self, transaction: &SignedTransaction) -> CliTypedResult<TransactionSummary>;
}

#[async_trait]
impl GroupChain for ChainSender<'_> {
    async fn estimate_gas(
        &self,
        payload: TransactionPayload,
        sequence_number: u64,
    ) -> CliTypedResult<GasEstimate> {
        let transaction_factory =
            TransactionFactory::new(self.chain_id).with_gas_unit_price(self.gas_unit_price);
        let txn_builder = transaction_factory
            .payload(payload)
            .sender(self.address)
            .sequence_number(sequence_number)
            .expiration_timestamp_secs(self.expiration_time_secs);

        let unsigned_transaction =
            if self.options.replay_protection_type == ReplayProtectionType::Nonce {
                let mut rng = rand::thread_rng();
                txn_builder
                    .upgrade_payload_with_rng(&mut rng, true, true)
                    .build()
            } else {
                txn_builder.build()
            };

        let signed_transaction = SignedTransaction::new(
            unsigned_transaction,
            self.public_key.clone(),
            Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
        );

        let txns = self
            .client
            .simulate_with_gas_estimation(&signed_transaction, true, false)
            .await?
            .into_inner();
        let simulated_txn = txns.first().unwrap();

        // Check if the transaction will pass
        if !simulated_txn.info.success {
            return Err(CliError::SimulationError(
                simulated_txn.info.vm_status.clone(),
            ));
        }

        let gas_used = simulated_txn.info.gas_used.0;
        let max_gas =
            adjust_gas_headroom(gas_used, max(simulated_txn.request.max_gas_amount.0, 530));
        Ok(GasEstimate { gas_used, max_gas })
    }

    fn sign(
        &self,
        payload: TransactionPayload,
        sequence_number: u64,
        max_gas: u64,
    ) -> CliTypedResult<SignedTransaction> {
        let transaction_factory = TransactionFactory::new(self.chain_id)
            .with_gas_unit_price(self.gas_unit_price)
            .with_max_gas_amount(max_gas)
            .with_transaction_expiration_time(self.options.gas_options.expiration_secs);

        // Sign with the appropriate signer
        match self.options.get_transaction_account_type() {
            Ok(AccountType::Local) => {
                let (private_key, _) = self.options.get_key_and_address()?;
                let sender_account =
                    &mut LocalAccount::new(self.address, private_key, sequence_number);
                let mut txn_builder = transaction_factory.payload(payload);
                if self.options.replay_protection_type == ReplayProtectionType::Nonce {
                    let mut rng = rand::thread_rng();
                    txn_builder = txn_builder.upgrade_payload_with_rng(&mut rng, true, true);
                }
                Ok(sender_account.sign_with_transaction_builder(txn_builder))
            },
            Ok(AccountType::HardwareWallet) => {
                let sender_account = &mut HardwareWalletAccount::new(
                    self.address,
                    self.public_key.clone(),
                    self.options
                        .profile_options
                        .derivation_path()
                        .expect("derivative path is missing from profile")
                        .unwrap(),
                    HardwareWalletType::Ledger,
                    sequence_number,
                );
                let mut txn_builder = transaction_factory.payload(payload);
                if self.options.replay_protection_type == ReplayProtectionType::Nonce {
                    let mut rng = rand::thread_rng();
                    txn_builder = txn_builder.upgrade_payload_with_rng(&mut rng, true, true);
                }
                Ok(sender_account.sign_with_transaction_builder(txn_builder)?)
            },
            Err(err) => Err(err),
        }
    }

    async fn submit(&self, transaction: &SignedTransaction) -> CliTypedResult<()> {
        self.client
            .submit_bcs(transaction)
            .await
            .map(|_| ())
            .map_err(|err| CliError::ApiError(err.to_string()))
    }

    async fn wait(&self, transaction: &SignedTransaction) -> CliTypedResult<TransactionSummary> {
        let response = self
            .client
            .wait_for_signed_transaction(transaction)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?;
        Ok(TransactionSummary::from(response.into_inner()))
    }
}

/// The time at which a transaction that is built now expires. Warns if the local clock is behind the
/// clock of the chain.
fn expiration_time_secs(options: &TransactionOptions, state: &State) -> CliTypedResult<u64> {
    // Retrieve local time and check clock skew
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| CliError::UnexpectedError(err.to_string()))?
        .as_secs();
    let now_usecs = now * US_IN_SECS;
    if now_usecs < state.timestamp_usecs - ACCEPTED_CLOCK_SKEW_US {
        eprintln!(
            "Local clock is skewed from blockchain clock. Clock is more than {} seconds behind the blockchain {}",
            ACCEPTED_CLOCK_SKEW_US,
            state.timestamp_usecs / US_IN_SECS
        );
    }
    Ok(now + options.gas_options.expiration_secs)
}

/// The network that links to an explorer point to, if it is known from the profile.
fn explorer_network(options: &TransactionOptions) -> Option<Network> {
    options.profile_options.profile().ok().and_then(|profile| {
        if let Some(network) = profile.network {
            Some(network)
        } else {
            profile.rest_url.and_then(|url| {
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
            })
        }
    })
}

/// The gas unit price to use: the one that is given, or the chain's estimate, which the user has to
/// confirm.
async fn gas_unit_price_to_use(
    options: &TransactionOptions,
    client: &Client,
) -> CliTypedResult<(u64, bool)> {
    if let Some(gas_unit_price) = options.gas_options.gas_unit_price {
        Ok((gas_unit_price, false))
    } else {
        let gas_unit_price = client.estimate_gas_price().await?.into_inner().gas_estimate;
        Ok((gas_unit_price, true))
    }
}

async fn submit_to_chain(
    options: &TransactionOptions,
    payload: TransactionPayload,
) -> CliTypedResult<TransactionSummary> {
    let client = options.rest_client()?;
    let (sender_public_key, sender_address) = options.get_public_key_and_address()?;

    // Estimate gas unit price if not specified
    let (gas_unit_price, ask_to_confirm_price) = gas_unit_price_to_use(options, &client).await?;

    // Get sequence number for account
    let (account, state) = get_account_with_state(&client, sender_address).await?;
    let sequence_number = account.sequence_number;
    let expiration_time_secs = expiration_time_secs(options, &state)?;

    let sender = ChainSender {
        options,
        client,
        public_key: sender_public_key,
        address: sender_address,
        chain_id: ChainId::new(state.chain_id),
        gas_unit_price,
        expiration_time_secs,
    };

    // Determine max gas
    let max_gas = if let Some(max_gas) = options.gas_options.max_gas {
        if ask_to_confirm_price {
            let message = format!(
                "Do you want to submit transaction for a maximum of {} Octas at a gas unit price of {} Octas?",
                max_gas * gas_unit_price,
                gas_unit_price
            );
            prompt_yes_with_override(&message, options.prompt_options)?;
        }
        max_gas
    } else {
        let estimate = sender
            .estimate_gas(payload.clone(), sequence_number)
            .await?;

        let upper_cost_bound = estimate.max_gas * gas_unit_price;
        let lower_cost_bound = estimate.gas_used * gas_unit_price;
        let message = format!(
            "Do you want to submit a transaction for a range of [{} - {}] Octas at a gas unit price of {} Octas?",
            lower_cost_bound, upper_cost_bound, gas_unit_price
        );
        prompt_yes_with_override(&message, options.prompt_options)?;
        estimate.max_gas
    };

    // Build the final transaction
    let transaction = sender.sign(payload, sequence_number, max_gas)?;

    // Submit the transaction
    sender
        .client
        .submit_bcs(&transaction)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    let transaction_hash = transaction.clone().committed_hash();

    // Print explorer link
    eprintln!(
        "Transaction submitted: {}",
        explorer_transaction_link(transaction_hash, explorer_network(options))
    );

    let response = sender
        .client
        .wait_for_signed_transaction(&transaction)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;

    Ok(TransactionSummary::from(response.into_inner()))
}

/// Submits transactions that don't depend on each other without waiting for one to be committed
/// before sending the next, and waits for all of them. Sequence-number transactions get
/// consecutive numbers; nonce-protected transactions can execute independently. Returns the
/// outcome of every transaction that was submitted, in
/// order; a transaction that can't be prepared, and so can't be submitted, ends the list.
async fn submit_many_to_chain(
    options: &TransactionOptions,
    payloads: Vec<TransactionPayload>,
) -> CliTypedResult<Vec<CliTypedResult<TransactionSummary>>> {
    let client = options.rest_client()?;
    let (sender_public_key, sender_address) = options.get_public_key_and_address()?;

    // Estimate gas unit price if not specified
    let (gas_unit_price, ask_to_confirm_price) = gas_unit_price_to_use(options, &client).await?;

    // Nonce-protected transactions share the fetched sequence number; sequence-number
    // transactions use it as the start of their consecutive range.
    let (account, state) = get_account_with_state(&client, sender_address).await?;
    let sequence_number = account.sequence_number;
    let expiration_time_secs = expiration_time_secs(options, &state)?;

    let sender = ChainSender {
        options,
        client,
        public_key: sender_public_key,
        address: sender_address,
        chain_id: ChainId::new(state.chain_id),
        gas_unit_price,
        expiration_time_secs,
    };
    let settings = GroupSettings {
        gas_unit_price,
        ask_to_confirm_price,
        max_gas: options.gas_options.max_gas,
        replay_protection_type: options.replay_protection_type,
        prompt_options: options.prompt_options,
        explorer_network: explorer_network(options),
    };
    submit_group(&sender, &settings, sequence_number, payloads).await
}

/// What `submit_group` needs to know from the command line.
struct GroupSettings {
    gas_unit_price: u64,
    /// Whether the gas unit price was estimated, so that the user has to confirm it.
    ask_to_confirm_price: bool,
    /// The max gas of every transaction, if it was given. Otherwise it is estimated for each.
    max_gas: Option<u64>,
    replay_protection_type: ReplayProtectionType,
    prompt_options: PromptOptions,
    /// The network that links to an explorer point to, if it is known.
    explorer_network: Option<Network>,
}

/// The logic of `submit_many_to_chain`, apart from the calls to the chain: `payloads` are signed as
/// consecutive sequence numbers in seqnum mode, or independent nonces in nonce mode. They are all
/// submitted at once and waited for at once.
async fn submit_group<C: GroupChain + Sync>(
    chain: &C,
    settings: &GroupSettings,
    sequence_number: u64,
    payloads: Vec<TransactionPayload>,
) -> CliTypedResult<Vec<CliTypedResult<TransactionSummary>>> {
    let gas_unit_price = settings.gas_unit_price;

    // The error that ends the list, because the transactions from there on can't be submitted
    let mut preparation_error = None;
    let mut payloads = payloads;

    // Determine max gas
    let max_gas_amounts: Vec<u64> = if let Some(max_gas) = settings.max_gas {
        if settings.ask_to_confirm_price {
            let message = format!(
                "Do you want to submit {} transactions for a maximum of {} Octas each at a gas unit price of {} Octas?",
                payloads.len(),
                max_gas * gas_unit_price,
                gas_unit_price
            );
            prompt_yes_with_override(&message, settings.prompt_options)?;
        }
        vec![max_gas; payloads.len()]
    } else {
        // Simulate each as the next transaction of the account, against the state of the chain now
        let estimates = join_all(
            payloads
                .iter()
                .map(|payload| chain.estimate_gas(payload.clone(), sequence_number)),
        )
        .await;
        let mut estimated = vec![];
        for estimate in estimates {
            match estimate {
                Ok(estimate) => estimated.push(estimate),
                Err(error) => {
                    preparation_error = Some(error);
                    break;
                },
            }
        }
        payloads.truncate(estimated.len());

        if !estimated.is_empty() {
            let upper_cost_bound: u64 =
                estimated.iter().map(|e| e.max_gas).sum::<u64>() * gas_unit_price;
            let lower_cost_bound: u64 =
                estimated.iter().map(|e| e.gas_used).sum::<u64>() * gas_unit_price;
            let message = format!(
                "Do you want to submit {} transactions for a range of [{} - {}] Octas at a gas unit price of {} Octas?",
                estimated.len(),
                lower_cost_bound,
                upper_cost_bound,
                gas_unit_price
            );
            prompt_yes_with_override(&message, settings.prompt_options)?;
        }
        estimated.iter().map(|estimate| estimate.max_gas).collect()
    };

    // Orderless transactions are independent: they all carry the current sequence number,
    // but are distinguished by fresh replay-protection nonces in their signed payloads.
    let mut transactions = Vec::with_capacity(payloads.len());
    let mut nonces = std::collections::HashSet::new();
    for (offset, (payload, max_gas)) in payloads.into_iter().zip(max_gas_amounts).enumerate() {
        let sequence = match settings.replay_protection_type {
            ReplayProtectionType::Nonce => sequence_number,
            ReplayProtectionType::Seqnum => sequence_number + offset as u64,
        };
        match chain.sign(payload, sequence, max_gas) {
            Ok(transaction) => {
                if settings.replay_protection_type == ReplayProtectionType::Nonce {
                    match transaction.replay_protector() {
                        ReplayProtector::Nonce(nonce) if nonces.insert(nonce) => {},
                        ReplayProtector::Nonce(_) => {
                            preparation_error = Some(CliError::UnexpectedError(
                                "Duplicate orderless nonce in transaction group".into(),
                            ));
                            break;
                        },
                        ReplayProtector::SequenceNumber(_) => {
                            preparation_error = Some(CliError::UnexpectedError(
                                "Orderless transaction was signed without a nonce".into(),
                            ));
                            break;
                        },
                    }
                }
                transactions.push(transaction);
            },
            Err(error) => {
                preparation_error = Some(error);
                break;
            },
        }
    }

    // Submit all of them at once
    let submissions = join_all(
        transactions
            .iter()
            .map(|transaction| chain.submit(transaction)),
    )
    .await;
    for (transaction, submission) in transactions.iter().zip(&submissions) {
        if submission.is_ok() {
            eprintln!(
                "Transaction submitted: {}",
                explorer_transaction_link(
                    transaction.clone().committed_hash(),
                    settings.explorer_network
                )
            );
        }
    }

    // And wait for all of them
    let mut outcomes: Vec<CliTypedResult<TransactionSummary>> =
        join_all(transactions.iter().zip(submissions).map(
            |(transaction, submission)| async move {
                submission?;
                chain.wait(transaction).await
            },
        ))
        .await;
    outcomes.extend(preparation_error.map(Err));
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
    use aptos_keygen::KeyGen;
    use aptos_types::transaction::{EntryFunction, RawTransaction};
    use move_core_types::{identifier::Identifier, language_storage::ModuleId};
    use std::{sync::Mutex, time::Duration};
    use tokio::{
        sync::Barrier,
        time::{sleep, timeout},
    };

    /// The sequence number of the first transaction of a group.
    const FIRST: u64 = 7;

    /// What goes wrong, if anything: estimating the gas fails for the payload with the index, and
    /// the other steps fail for the transaction with the sequence number.
    #[derive(Default)]
    struct Failures {
        estimate: Option<usize>,
        sign: Option<u64>,
        submit: Option<u64>,
        wait: Option<u64>,
    }

    /// What a [`FakeChain`] was asked to do, in order.
    #[derive(Default)]
    struct Calls {
        /// The index of the payloads that the gas was estimated for.
        estimated: Vec<usize>,
        /// The sequence number and the max gas of the transactions that were signed.
        signed: Vec<(u64, u64)>,
        /// The sequence numbers of the transactions that were submitted.
        submitted: Vec<u64>,
        /// The orderless nonces of transactions that reached submit.
        submitted_nonces: Vec<u64>,
        /// The sequence numbers of the transactions that were committed, as they were.
        committed: Vec<u64>,
    }

    struct FakeChain {
        private_key: Ed25519PrivateKey,
        nonce_mode: bool,
        duplicate_nonce: bool,
        failures: Failures,
        calls: Mutex<Calls>,
        /// A submission only completes when this many of them are in progress at the same time.
        submit_barrier: Option<Barrier>,
        /// The transactions with a lower sequence number take longer to be committed, so that they
        /// are committed in the opposite order.
        commit_in_reverse: bool,
    }

    impl FakeChain {
        fn new() -> Self {
            Self {
                private_key: KeyGen::from_seed([0; 32]).generate_ed25519_private_key(),
                nonce_mode: false,
                duplicate_nonce: false,
                failures: Failures::default(),
                calls: Mutex::new(Calls::default()),
                submit_barrier: None,
                commit_in_reverse: false,
            }
        }

        fn failing(failures: Failures) -> Self {
            Self {
                failures,
                ..Self::new()
            }
        }

        fn calls(&self) -> std::sync::MutexGuard<'_, Calls> {
            self.calls.lock().unwrap()
        }
    }

    #[async_trait]
    impl GroupChain for FakeChain {
        async fn estimate_gas(
            &self,
            payload: TransactionPayload,
            _sequence_number: u64,
        ) -> CliTypedResult<GasEstimate> {
            let index = index_of(&payload);
            self.calls().estimated.push(index);
            if self.failures.estimate == Some(index) {
                return Err(CliError::SimulationError(
                    "the simulation failed".to_string(),
                ));
            }
            Ok(GasEstimate {
                gas_used: 1000 + index as u64,
                max_gas: 2000 + index as u64,
            })
        }

        fn sign(
            &self,
            payload: TransactionPayload,
            sequence_number: u64,
            max_gas: u64,
        ) -> CliTypedResult<SignedTransaction> {
            if self.failures.sign == Some(sequence_number) {
                return Err(CliError::UnexpectedError("the signing failed".to_string()));
            }
            self.calls().signed.push((sequence_number, max_gas));
            let payload = if self.nonce_mode {
                let nonce = if self.duplicate_nonce {
                    1
                } else {
                    index_of(&payload) as u64 + 1
                };
                payload.upgrade_payload_with_fn(true, Some(|| nonce))
            } else {
                payload
            };
            // SDK builders set the raw sequence to MAX for an orderless transaction;
            // the sequence argument is only a hint supplied to the signer.
            let raw_sequence = if self.nonce_mode {
                u64::MAX
            } else {
                sequence_number
            };
            let transaction = RawTransaction::new(
                AccountAddress::ONE,
                raw_sequence,
                payload,
                max_gas,
                100,
                u64::MAX,
                ChainId::test(),
            );
            Ok(transaction
                .sign(&self.private_key, self.private_key.public_key())
                .unwrap()
                .into_inner())
        }

        async fn submit(&self, transaction: &SignedTransaction) -> CliTypedResult<()> {
            let sequence_number = transaction.sequence_number();
            self.calls().submitted.push(sequence_number);
            if let ReplayProtector::Nonce(nonce) = transaction.replay_protector() {
                self.calls().submitted_nonces.push(nonce);
            }
            if let Some(barrier) = &self.submit_barrier {
                barrier.wait().await;
            }
            if self.failures.submit == Some(sequence_number) {
                return Err(CliError::ApiError("the submission failed".to_string()));
            }
            Ok(())
        }

        async fn wait(
            &self,
            transaction: &SignedTransaction,
        ) -> CliTypedResult<TransactionSummary> {
            let sequence_number = transaction.sequence_number();
            if self.commit_in_reverse {
                sleep(Duration::from_millis(20 * (FIRST + 10 - sequence_number))).await;
            }
            self.calls().committed.push(sequence_number);
            if self.failures.wait == Some(sequence_number) {
                return Err(CliError::ApiError("the transaction expired".to_string()));
            }
            Ok(summary(sequence_number))
        }
    }

    /// Payloads that are told apart by the name of their function: `f0`, `f1`, ...
    fn payloads(count: usize) -> Vec<TransactionPayload> {
        (0..count)
            .map(|index| {
                TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::new(AccountAddress::ONE, Identifier::new("m").unwrap()),
                    Identifier::new(format!("f{}", index)).unwrap(),
                    vec![],
                    vec![],
                ))
            })
            .collect()
    }

    fn index_of(payload: &TransactionPayload) -> usize {
        let TransactionPayload::EntryFunction(entry_function) = payload else {
            panic!("expected an entry function payload, got {:?}", payload);
        };
        entry_function.function().as_str()[1..].parse().unwrap()
    }

    fn summary(sequence_number: u64) -> TransactionSummary {
        TransactionSummary {
            transaction_hash: aptos_crypto::HashValue::zero().into(),
            gas_used: None,
            gas_unit_price: None,
            pending: None,
            sender: None,
            sequence_number: Some(sequence_number),
            replay_protector: None,
            success: Some(true),
            timestamp_us: None,
            version: None,
            vm_status: None,
            deployed_object_address: None,
            events: None,
            changes: None,
        }
    }

    fn settings(max_gas: Option<u64>) -> GroupSettings {
        GroupSettings {
            gas_unit_price: 100,
            ask_to_confirm_price: false,
            max_gas,
            replay_protection_type: ReplayProtectionType::Seqnum,
            prompt_options: PromptOptions::yes(),
            explorer_network: None,
        }
    }

    /// The sequence numbers that the outcomes are for, with `None` for an error.
    fn sequence_numbers(outcomes: &[CliTypedResult<TransactionSummary>]) -> Vec<Option<u64>> {
        outcomes
            .iter()
            .map(|outcome| {
                outcome
                    .as_ref()
                    .ok()
                    .and_then(|summary| summary.sequence_number)
            })
            .collect()
    }

    #[tokio::test]
    async fn group_signs_with_consecutive_sequence_numbers() {
        let chain = FakeChain::new();

        let outcomes = submit_group(&chain, &settings(Some(5000)), FIRST, payloads(4))
            .await
            .unwrap();

        assert_eq!(sequence_numbers(&outcomes), vec![
            Some(7),
            Some(8),
            Some(9),
            Some(10)
        ]);
        assert_eq!(chain.calls().signed, vec![
            (7, 5000),
            (8, 5000),
            (9, 5000),
            (10, 5000)
        ]);
        // The max gas was given, so there is nothing to estimate.
        assert!(chain.calls().estimated.is_empty());
        // The transactions are the payloads, in order: the one signed with 8 is the second.
        assert_eq!(chain.calls().submitted, vec![7, 8, 9, 10]);
    }

    #[tokio::test]
    async fn nonce_group_signs_independent_replay_protectors() {
        let chain = FakeChain {
            nonce_mode: true,
            ..FakeChain::new()
        };
        let settings = GroupSettings {
            replay_protection_type: ReplayProtectionType::Nonce,
            ..settings(Some(5000))
        };
        let outcomes = submit_group(&chain, &settings, FIRST, payloads(4))
            .await
            .unwrap();
        assert!(outcomes.iter().all(|outcome| outcome.is_ok()));
        assert_eq!(chain.calls().signed, vec![(FIRST, 5000); 4]);
        assert_eq!(chain.calls().submitted, vec![u64::MAX; 4]);
        let nonces: std::collections::HashSet<u64> =
            chain.calls().submitted_nonces.iter().copied().collect();
        assert_eq!(nonces.len(), 4);
    }

    #[tokio::test]
    async fn nonce_group_estimates_each_payload_independently() {
        let chain = FakeChain {
            nonce_mode: true,
            ..FakeChain::new()
        };
        let settings = GroupSettings {
            replay_protection_type: ReplayProtectionType::Nonce,
            ..settings(None)
        };
        let outcomes = submit_group(&chain, &settings, FIRST, payloads(3))
            .await
            .unwrap();
        assert!(outcomes.iter().all(|outcome| outcome.is_ok()));
        assert_eq!(chain.calls().estimated, vec![0, 1, 2]);
        assert_eq!(chain.calls().signed, vec![
            (FIRST, 2000),
            (FIRST, 2001),
            (FIRST, 2002)
        ]);
        assert_eq!(chain.calls().submitted, vec![u64::MAX; 3]);
        assert_eq!(chain.calls().submitted_nonces.len(), 3);
    }

    #[tokio::test]
    async fn nonce_group_rejects_duplicate_nonce_before_sending_it() {
        let chain = FakeChain {
            nonce_mode: true,
            duplicate_nonce: true,
            ..FakeChain::new()
        };
        let settings = GroupSettings {
            replay_protection_type: ReplayProtectionType::Nonce,
            ..settings(Some(5000))
        };
        let outcomes = submit_group(&chain, &settings, FIRST, payloads(3))
            .await
            .unwrap();
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes[0].is_ok());
        assert!(outcomes[1]
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Duplicate orderless nonce"));
        assert_eq!(chain.calls().submitted_nonces, vec![1]);
    }

    #[tokio::test]
    async fn nonce_group_rejects_signer_that_forgets_replay_nonce() {
        let chain = FakeChain::new();
        let settings = GroupSettings {
            replay_protection_type: ReplayProtectionType::Nonce,
            ..settings(Some(5000))
        };
        let outcomes = submit_group(&chain, &settings, FIRST, payloads(3))
            .await
            .unwrap();
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0]
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("without a nonce"));
        assert!(chain.calls().submitted.is_empty());
    }

    #[tokio::test]
    async fn group_estimates_the_gas_of_every_transaction() {
        let chain = FakeChain::new();

        let outcomes = submit_group(&chain, &settings(None), FIRST, payloads(3))
            .await
            .unwrap();

        assert_eq!(sequence_numbers(&outcomes), vec![Some(7), Some(8), Some(9)]);
        assert_eq!(chain.calls().estimated, vec![0, 1, 2]);
        // Each transaction is signed with the estimate for its payload.
        assert_eq!(chain.calls().signed, vec![(7, 2000), (8, 2001), (9, 2002)]);
    }

    #[tokio::test]
    async fn group_submits_all_transactions_at_the_same_time() {
        // Every submission waits for the others, so they only complete if they are in progress
        // together.
        let chain = FakeChain {
            submit_barrier: Some(Barrier::new(5)),
            ..FakeChain::new()
        };

        let outcomes = timeout(
            Duration::from_secs(10),
            submit_group(&chain, &settings(Some(5000)), FIRST, payloads(5)),
        )
        .await
        .expect("the transactions are submitted at the same time")
        .unwrap();

        assert_eq!(outcomes.len(), 5);
        assert!(outcomes.iter().all(|outcome| outcome.is_ok()));
    }

    #[tokio::test]
    async fn group_waits_for_all_transactions_at_the_same_time_and_keeps_their_order() {
        let chain = FakeChain {
            commit_in_reverse: true,
            ..FakeChain::new()
        };

        let outcomes = submit_group(&chain, &settings(Some(5000)), FIRST, payloads(4))
            .await
            .unwrap();

        // They were committed in the opposite order, which can only be if they were waited for at
        // the same time, and the outcomes are in the order of the payloads nevertheless.
        assert_eq!(chain.calls().committed, vec![10, 9, 8, 7]);
        assert_eq!(sequence_numbers(&outcomes), vec![
            Some(7),
            Some(8),
            Some(9),
            Some(10)
        ]);
    }

    #[tokio::test]
    async fn group_stops_at_a_transaction_whose_gas_cannot_be_estimated() {
        let chain = FakeChain::failing(Failures {
            estimate: Some(2),
            ..Failures::default()
        });

        let outcomes = submit_group(&chain, &settings(None), FIRST, payloads(5))
            .await
            .unwrap();

        // The transactions before it are submitted, and its error ends the list.
        assert_eq!(sequence_numbers(&outcomes), vec![Some(7), Some(8), None]);
        assert!(matches!(outcomes[2], Err(CliError::SimulationError(_))));
        assert_eq!(chain.calls().submitted, vec![7, 8]);
        assert_eq!(chain.calls().signed.len(), 2);
    }

    #[tokio::test]
    async fn group_stops_at_a_transaction_that_cannot_be_signed() {
        let chain = FakeChain::failing(Failures {
            sign: Some(9),
            ..Failures::default()
        });

        let outcomes = submit_group(&chain, &settings(Some(5000)), FIRST, payloads(5))
            .await
            .unwrap();

        assert_eq!(sequence_numbers(&outcomes), vec![Some(7), Some(8), None]);
        assert!(matches!(outcomes[2], Err(CliError::UnexpectedError(_))));
        assert_eq!(chain.calls().submitted, vec![7, 8]);
    }

    #[tokio::test]
    async fn group_reports_a_failed_submission_in_its_place() {
        let chain = FakeChain::failing(Failures {
            submit: Some(8),
            ..Failures::default()
        });

        let outcomes = submit_group(&chain, &settings(Some(5000)), FIRST, payloads(4))
            .await
            .unwrap();

        // The others were submitted and committed. The one that wasn't accepted isn't waited for.
        assert_eq!(sequence_numbers(&outcomes), vec![
            Some(7),
            None,
            Some(9),
            Some(10)
        ]);
        assert!(matches!(outcomes[1], Err(CliError::ApiError(_))));
        assert!(!chain.calls().committed.contains(&8));
        assert_eq!(chain.calls().committed.len(), 3);
    }

    #[tokio::test]
    async fn group_reports_a_transaction_that_does_not_commit_in_its_place() {
        let chain = FakeChain::failing(Failures {
            wait: Some(9),
            ..Failures::default()
        });

        let outcomes = submit_group(&chain, &settings(Some(5000)), FIRST, payloads(4))
            .await
            .unwrap();

        assert_eq!(sequence_numbers(&outcomes), vec![
            Some(7),
            Some(8),
            None,
            Some(10)
        ]);
        assert!(matches!(outcomes[2], Err(CliError::ApiError(_))));
    }

    #[tokio::test]
    async fn group_that_the_user_declines_submits_nothing() {
        let chain = FakeChain::new();
        let settings = GroupSettings {
            // The gas unit price was estimated, so the user is asked.
            ask_to_confirm_price: true,
            prompt_options: PromptOptions::no(),
            ..settings(Some(5000))
        };

        let result = submit_group(&chain, &settings, FIRST, payloads(3)).await;

        assert!(matches!(result, Err(CliError::AbortedError)));
        assert!(chain.calls().signed.is_empty());
        assert!(chain.calls().submitted.is_empty());
    }

    #[tokio::test]
    async fn group_without_transactions_has_no_outcomes() {
        let chain = FakeChain::new();

        let outcomes = submit_group(&chain, &settings(None), FIRST, vec![])
            .await
            .unwrap();

        assert!(outcomes.is_empty());
    }
}
