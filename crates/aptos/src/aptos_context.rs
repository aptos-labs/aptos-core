// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of [`aptos_move_cli::AptosContext`] for the full Aptos CLI.
//!
//! This bridges `aptos-move-cli` commands (which use `aptos_cli_common::TransactionOptions`)
//! to the full Aptos CLI environment for network operations, local simulation,
//! gas profiling, benchmarking, and session-based execution.

use aptos_cli_common::{
    explorer_transaction_link, get_account_with_state, prompt_yes_with_override, AccountType,
    CliError, CliTypedResult, Network, ReplayProtectionType, TransactionOptions,
    TransactionSummary, ACCEPTED_CLOCK_SKEW_US, US_IN_SECS,
};
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_global_constants::adjust_gas_headroom;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{HardwareWalletAccount, HardwareWalletType, LocalAccount, TransactionSigner},
};
use aptos_types::{
    account_config::AccountResource,
    chain_id::ChainId,
    transaction::{
        PersistedAuxiliaryInfo, SignedTransaction, TransactionPayload, TransactionStatus,
    },
};
use aptos_vm_types::output::VMOutput;
use async_trait::async_trait;
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
        vm_status: Some(vm_status.to_string()),
        deployed_object_address: None,
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

    let (vm_status, txn_output) = sess.execute_transaction(transaction, false)?;

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
        vm_status: Some(vm_status.to_string()),
        deployed_object_address: None,
    })
}

// ── Remote chain submission ──

async fn submit_to_chain(
    options: &TransactionOptions,
    payload: TransactionPayload,
) -> CliTypedResult<TransactionSummary> {
    let client = options.rest_client()?;
    let (sender_public_key, sender_address) = options.get_public_key_and_address()?;

    // Estimate gas unit price if not specified
    let ask_to_confirm_price;
    let gas_unit_price = if let Some(gas_unit_price) = options.gas_options.gas_unit_price {
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
    let expiration_time_secs = now + options.gas_options.expiration_secs;

    let chain_id = ChainId::new(state.chain_id);

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
        let transaction_factory =
            TransactionFactory::new(chain_id).with_gas_unit_price(gas_unit_price);
        let txn_builder = transaction_factory
            .payload(payload.clone())
            .sender(sender_address)
            .sequence_number(sequence_number)
            .expiration_timestamp_secs(expiration_time_secs);

        let unsigned_transaction = if options.replay_protection_type == ReplayProtectionType::Nonce
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

        // Check if the transaction will pass
        if !simulated_txn.info.success {
            return Err(CliError::SimulationError(
                simulated_txn.info.vm_status.clone(),
            ));
        }

        let gas_used = simulated_txn.info.gas_used.0;
        let adjusted_max_gas =
            adjust_gas_headroom(gas_used, max(simulated_txn.request.max_gas_amount.0, 530));

        let upper_cost_bound = adjusted_max_gas * gas_unit_price;
        let lower_cost_bound = gas_used * gas_unit_price;
        let message = format!(
            "Do you want to submit a transaction for a range of [{} - {}] Octas at a gas unit price of {} Octas?",
            lower_cost_bound, upper_cost_bound, gas_unit_price
        );
        prompt_yes_with_override(&message, options.prompt_options)?;
        adjusted_max_gas
    };

    // Build the final transaction
    let transaction_factory = TransactionFactory::new(chain_id)
        .with_gas_unit_price(gas_unit_price)
        .with_max_gas_amount(max_gas)
        .with_transaction_expiration_time(options.gas_options.expiration_secs);

    // Sign with the appropriate signer
    let transaction = match options.get_transaction_account_type() {
        Ok(AccountType::Local) => {
            let (private_key, _) = options.get_key_and_address()?;
            let sender_account =
                &mut LocalAccount::new(sender_address, private_key, sequence_number);
            let mut txn_builder = transaction_factory.payload(payload);
            if options.replay_protection_type == ReplayProtectionType::Nonce {
                let mut rng = rand::thread_rng();
                txn_builder = txn_builder.upgrade_payload_with_rng(&mut rng, true, true);
            }
            sender_account.sign_with_transaction_builder(txn_builder)
        },
        Ok(AccountType::HardwareWallet) => {
            let sender_account = &mut HardwareWalletAccount::new(
                sender_address,
                sender_public_key,
                options
                    .profile_options
                    .derivation_path()
                    .expect("derivative path is missing from profile")
                    .unwrap(),
                HardwareWalletType::Ledger,
                sequence_number,
            );
            let mut txn_builder = transaction_factory.payload(payload);
            if options.replay_protection_type == ReplayProtectionType::Nonce {
                let mut rng = rand::thread_rng();
                txn_builder = txn_builder.upgrade_payload_with_rng(&mut rng, true, true);
            }
            sender_account.sign_with_transaction_builder(txn_builder)?
        },
        Err(err) => return Err(err),
    };

    // Submit the transaction
    client
        .submit_bcs(&transaction)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    let transaction_hash = transaction.clone().committed_hash();

    // Print explorer link
    let network = options.profile_options.profile().ok().and_then(|profile| {
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
    });
    eprintln!(
        "Transaction submitted: {}",
        explorer_transaction_link(transaction_hash, network)
    );

    let response = client
        .wait_for_signed_transaction(&transaction)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;

    Ok(TransactionSummary::from(response.into_inner()))
}
