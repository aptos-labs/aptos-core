// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_cli_base::types::{CliError, CliResult, CliTypedResult, ResultWrapper};
use aptos_logger::Level;
use aptos_rest_client::Client;
use aptos_telemetry::collect_build_information;
use aptos_types::chain_id::ChainId;
use move_deps::move_core_types::account_address::AccountAddress;
use reqwest::Url;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

/// Convert any successful response to Success
pub async fn to_common_success_result<T>(
    command: &str,
    start_time: Instant,
    result: CliTypedResult<T>,
) -> CliResult {
    to_common_result(command, start_time, result.map(|_| "Success")).await
}

/// For pretty printing outputs in JSON
pub async fn to_common_result<T: Serialize>(
    command: &str,
    start_time: Instant,
    result: CliTypedResult<T>,
) -> CliResult {
    let latency = start_time.elapsed();
    let is_err = result.is_err();
    let error = if let Err(ref error) = result {
        Some(error.to_string())
    } else {
        None
    };
    send_telemetry_event(command, latency, !is_err, error).await;
    let result: ResultWrapper<T> = result.into();
    let string = serde_json::to_string_pretty(&result).unwrap();
    if is_err {
        Err(string)
    } else {
        Ok(string)
    }
}

/// Sends a telemetry event about the CLI build, command and result
async fn send_telemetry_event(
    command: &str,
    latency: Duration,
    success: bool,
    error: Option<String>,
) {
    // Collect the build information
    let build_information = collect_build_information!();

    // Send the event
    aptos_telemetry::cli_metrics::send_cli_telemetry_event(
        build_information,
        command.into(),
        latency,
        success,
        error,
    )
    .await;
}

/// Retrieves sequence number from the rest client
pub async fn get_sequence_number(
    client: &aptos_rest_client::Client,
    address: AccountAddress,
) -> CliTypedResult<u64> {
    let account_response = client
        .get_account(address)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    let account = account_response.inner();
    Ok(account.sequence_number)
}

/// Retrieves the chain id from the rest client
pub async fn chain_id(rest_client: &Client) -> CliTypedResult<ChainId> {
    let state = rest_client
        .get_ledger_information()
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?
        .into_inner();
    Ok(ChainId::new(state.chain_id))
}

/// Fund account (and possibly create it) from a faucet
pub async fn fund_account(
    faucet_url: Url,
    num_coins: u64,
    address: AccountAddress,
) -> CliTypedResult<()> {
    let response = reqwest::Client::new()
        .post(format!(
            "{}mint?amount={}&auth_key={}",
            faucet_url, num_coins, address
        ))
        .send()
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    if response.status() == 200 {
        Ok(())
    } else {
        Err(CliError::ApiError(format!(
            "Faucet issue: {}",
            response.status()
        )))
    }
}

pub fn start_logger() {
    let mut logger = aptos_logger::Logger::new();
    logger
        .channel_size(1000)
        .is_async(false)
        .level(Level::Warn)
        .read_env();
    logger.build();
}
