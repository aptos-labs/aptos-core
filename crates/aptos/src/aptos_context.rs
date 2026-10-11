// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of [`aptos_move_cli::AptosContext`] for the full Aptos CLI.
//!
//! This bridges `aptos-move-cli` commands (which use `aptos_cli_common::TransactionOptions`)
//! to the full Aptos CLI environment for network operations, local simulation,
//! gas profiling, benchmarking, and session-based execution.

use aptos_cli_common::{
    explorer_transaction_link, format_txn_status, get_account_with_state, prompt_yes_with_override,
    AccountType, CliError, CliTypedResult, Network, ReplayProtectionType, TransactionOptions,
    TransactionSummary, ACCEPTED_CLOCK_SKEW_US, US_IN_SECS,
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
        PersistedAuxiliaryInfo, SignedTransaction, TransactionPayload, TransactionStatus,
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
}

/// The gas that a transaction is estimated to need.
struct GasEstimate {
    gas_used: u64,
    max_gas: u64,
}

impl ChainSender<'_> {
    /// Simulates `payload` as the transaction of the sender with this sequence number, to estimate
    /// the gas that it needs. Fails if the simulation fails.
    async fn estimate_gas(
        &self,
        payload: TransactionPayload,
        sequence_number: u64,
        expiration_time_secs: u64,
    ) -> CliTypedResult<GasEstimate> {
        let transaction_factory =
            TransactionFactory::new(self.chain_id).with_gas_unit_price(self.gas_unit_price);
        let txn_builder = transaction_factory
            .payload(payload)
            .sender(self.address)
            .sequence_number(sequence_number)
            .expiration_timestamp_secs(expiration_time_secs);

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

    /// Signs `payload` as the transaction of the sender with this sequence number.
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
            .estimate_gas(payload.clone(), sequence_number, expiration_time_secs)
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
/// before sending the next, and waits for all of them. They get consecutive sequence numbers, so the
/// chain executes them in order. Returns the outcome of every transaction that was submitted, in
/// order; a transaction that can't be prepared, and so can't be submitted, ends the list.
async fn submit_many_to_chain(
    options: &TransactionOptions,
    payloads: Vec<TransactionPayload>,
) -> CliTypedResult<Vec<CliTypedResult<TransactionSummary>>> {
    let client = options.rest_client()?;
    let (sender_public_key, sender_address) = options.get_public_key_and_address()?;

    // Estimate gas unit price if not specified
    let (gas_unit_price, ask_to_confirm_price) = gas_unit_price_to_use(options, &client).await?;

    // The transactions get the sequence numbers that follow the one of the account
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
    };

    // The error that ends the list, because the transactions from there on can't be submitted
    let mut preparation_error = None;
    let mut payloads = payloads;

    // Determine max gas
    let max_gas_amounts: Vec<u64> = if let Some(max_gas) = options.gas_options.max_gas {
        if ask_to_confirm_price {
            let message = format!(
                "Do you want to submit {} transactions for a maximum of {} Octas each at a gas unit price of {} Octas?",
                payloads.len(),
                max_gas * gas_unit_price,
                gas_unit_price
            );
            prompt_yes_with_override(&message, options.prompt_options)?;
        }
        vec![max_gas; payloads.len()]
    } else {
        // Simulate each as the next transaction of the account, against the state of the chain now
        let estimates = join_all(payloads.iter().map(|payload| {
            sender.estimate_gas(payload.clone(), sequence_number, expiration_time_secs)
        }))
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
            prompt_yes_with_override(&message, options.prompt_options)?;
        }
        estimated.iter().map(|estimate| estimate.max_gas).collect()
    };

    // Sign them with consecutive sequence numbers
    let mut transactions = Vec::with_capacity(payloads.len());
    for (offset, (payload, max_gas)) in payloads.into_iter().zip(max_gas_amounts).enumerate() {
        match sender.sign(payload, sequence_number + offset as u64, max_gas) {
            Ok(transaction) => transactions.push(transaction),
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
            .map(|transaction| sender.client.submit_bcs(transaction)),
    )
    .await;
    let network = explorer_network(options);
    for (transaction, submission) in transactions.iter().zip(&submissions) {
        if submission.is_ok() {
            eprintln!(
                "Transaction submitted: {}",
                explorer_transaction_link(transaction.clone().committed_hash(), network)
            );
        }
    }

    // And wait for all of them
    let client = &sender.client;
    let mut outcomes: Vec<CliTypedResult<TransactionSummary>> =
        join_all(transactions.iter().zip(submissions).map(
            |(transaction, submission)| async move {
                submission.map_err(|err| CliError::ApiError(err.to_string()))?;
                let response = client
                    .wait_for_signed_transaction(transaction)
                    .await
                    .map_err(|err| CliError::ApiError(err.to_string()))?;
                Ok(TransactionSummary::from(response.into_inner()))
            },
        ))
        .await;
    outcomes.extend(preparation_error.map(Err));
    Ok(outcomes)
}
