// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{AccountType, CliError, CliTypedResult, TransactionOptions},
    utils::{get_account_with_state, prompt_yes_with_override},
};
use aptos_experimental_bulk_txn_submit::{
    coordinator::execute_txn_list, workloads::FixedPayloadSignedTransactionBuilder,
};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{chain_id::ChainId, transaction::TransactionPayload};
use std::time::Duration;

/// For transaction payload and options, either get gas profile or submit for execution.
pub async fn submit_repeatedly(
    txn_options_ref: &TransactionOptions,
    payload: TransactionPayload,
    num_times: usize,
    single_request_api_batch_size: usize,
    parallel_requests_outstanding: usize,
) -> CliTypedResult<usize> {
    if txn_options_ref.profile_gas || txn_options_ref.benchmark || txn_options_ref.local {
        return Err(CliError::UnexpectedError(
            "Cannot perform profiling, benchmarking or local execution for submit repeatedly."
                .to_string(),
        ));
    }

    let client = txn_options_ref.rest_client()?;
    let (sender_public_key, sender_address) = txn_options_ref.get_public_key_and_address()?;

    // Get sequence number for account
    let (account, state) = get_account_with_state(&client, sender_address).await?;
    let sequence_number = account.sequence_number;

    let sender_account = match txn_options_ref.get_transaction_account_type()? {
        AccountType::Local => {
            let (private_key, _) = txn_options_ref.get_key_and_address()?;
            LocalAccount::new(sender_address, private_key, sequence_number)
        },
        AccountType::HardwareWallet => {
            return Err(CliError::UnexpectedError(
                "Cannot use hardware wallet to submit repeatedly.".to_string(),
            ));
        },
    };

    let now = txn_options_ref.get_now_timestamp_checked(state.timestamp_usecs)?;
    let expiration_time_secs = now + txn_options_ref.gas_options.expiration_secs;

    let chain_id = ChainId::new(state.chain_id);
    // TODO: Check auth key against current private key and provide a better message

    let (gas_unit_price, max_gas) = txn_options_ref
        .compute_gas_price_and_max_gas(
            &payload,
            &client,
            &sender_address,
            &sender_public_key,
            sequence_number,
            chain_id,
            expiration_time_secs,
        )
        .await?;

    // Sign and submit transaction
    let transaction_factory = TransactionFactory::new(chain_id)
        .with_gas_unit_price(gas_unit_price)
        .with_max_gas_amount(max_gas)
        .with_transaction_expiration_time(txn_options_ref.gas_options.expiration_secs);

    prompt_yes_with_override(
        &format!(
            "About to submit {} transactions and spend up to {} APT. Continue?",
            num_times,
            num_times as f32 * gas_unit_price as f32 * max_gas as f32 / 1e8
        ),
        txn_options_ref.prompt_options,
    )?;

    let results = execute_txn_list(
        vec![sender_account],
        vec![client],
        (0..num_times).map(|_| ()).collect::<Vec<()>>(),
        single_request_api_batch_size,
        parallel_requests_outstanding,
        Duration::from_secs_f32(0.05),
        transaction_factory,
        FixedPayloadSignedTransactionBuilder::new(payload),
        true,
    )
    .await?;

    Ok(results.into_iter().filter(|v| *v == "success").count())
}
