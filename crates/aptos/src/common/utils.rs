// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{CliError, CliTypedResult, PromptOptions},
    CliResult,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_rest_client::{Client, Transaction};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_telemetry::constants::APTOS_CLI_PUSH_METRICS;
use aptos_types::{
    chain_id::ChainId,
    transaction::{authenticator::AuthenticationKey, TransactionPayload},
};
use itertools::Itertools;
use move_deps::move_core_types::account_address::AccountAddress;
use reqwest::Url;
use serde::Serialize;
use shadow_rs::shadow;
use std::{
    collections::{BTreeMap, HashMap},
    env,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, Instant},
};

shadow!(build);

/// Prompts for confirmation until a yes or no is given explicitly
pub fn prompt_yes(prompt: &str) -> bool {
    let mut result: Result<bool, ()> = Err(());

    // Read input until a yes or a no is given
    while result.is_err() {
        println!("{} [yes/no] >", prompt);
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        result = match input.trim().to_lowercase().as_str() {
            "yes" | "y" => Ok(true),
            "no" | "n" => Ok(false),
            _ => Err(()),
        };
    }
    result.unwrap()
}

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
    let error = if let Err(ref e) = result {
        e.to_str()
    } else {
        "None"
    };
    let metrics = collect_metrics(command, !is_err, latency, error);
    aptos_telemetry::send_env_data(
        APTOS_CLI_PUSH_METRICS.to_string(),
        uuid::Uuid::new_v4().to_string(),
        metrics,
    )
    .await;
    let result: ResultWrapper<T> = result.into();
    let string = serde_json::to_string_pretty(&result).unwrap();
    if is_err {
        Err(string)
    } else {
        Ok(string)
    }
}

/// Collect build and command metrics for better debugging of CLI
fn collect_metrics(
    command: &str,
    successful: bool,
    latency: Duration,
    error: &str,
) -> HashMap<String, String> {
    let mut metrics = HashMap::new();
    metrics.insert("Latency".to_string(), latency.as_millis().to_string());
    metrics.insert("Command".to_string(), command.to_string());
    metrics.insert("Successful".to_string(), successful.to_string());
    metrics.insert("Error".to_string(), error.to_string());
    metrics.insert("Version".to_string(), build::VERSION.to_string());
    metrics.insert("PkgVersion".to_string(), build::PKG_VERSION.to_string());
    metrics.insert(
        "ClapVersion".to_string(),
        build::CLAP_LONG_VERSION.to_string(),
    );
    metrics.insert("Commit".to_string(), build::COMMIT_HASH.to_string());
    metrics.insert("Branch".to_string(), build::BRANCH.to_string());
    metrics.insert("Tag".to_string(), build::TAG.to_string());
    metrics.insert("BUILD_OS".to_string(), build::BUILD_OS.to_string());
    metrics.insert(
        "BUILD_TARGET_OS".to_string(),
        build::BUILD_TARGET.to_string(),
    );
    metrics.insert(
        "BUILD_TARGET_ARCH".to_string(),
        build::BUILD_TARGET_ARCH.to_string(),
    );
    metrics.insert(
        "BUILD_RUST_CHANNEL".to_string(),
        build::BUILD_RUST_CHANNEL.to_string(),
    );
    metrics.insert("BUILD_TIME".to_string(), build::BUILD_TIME.to_string());
    metrics.insert("RUST_VERSION".to_string(), build::RUST_VERSION.to_string());
    metrics.insert(
        "RUST_TOOLCHAIN".to_string(),
        build::RUST_CHANNEL.to_string(),
    );
    metrics.insert(
        "CARGO_VERSION".to_string(),
        build::CARGO_VERSION.to_string(),
    );

    metrics
}

/// A result wrapper for displaying either a correct execution result or an error.
///
/// The purpose of this is to have a pretty easy to recognize JSON output format e.g.
///
/// {
///   "Result":{
///     "encoded":{ ... }
///   }
/// }
///
/// {
///   "Error":"Failed to run command"
/// }
///
#[derive(Debug, Serialize)]
enum ResultWrapper<T> {
    Result(T),
    Error(String),
}

impl<T> From<CliTypedResult<T>> for ResultWrapper<T> {
    fn from(result: CliTypedResult<T>) -> Self {
        match result {
            Ok(inner) => ResultWrapper::Result(inner),
            Err(inner) => ResultWrapper::Error(inner.to_string()),
        }
    }
}

/// Checks if a file exists, being overridden by `PromptOptions`
pub fn check_if_file_exists(file: &Path, prompt_options: PromptOptions) -> CliTypedResult<()> {
    if file.exists() {
        prompt_yes_with_override(
            &format!(
                "{:?} already exists, are you sure you want to overwrite it?",
                file.as_os_str(),
            ),
            prompt_options,
        )?
    }

    Ok(())
}

pub fn prompt_yes_with_override(prompt: &str, prompt_options: PromptOptions) -> CliTypedResult<()> {
    if prompt_options.assume_no || (!prompt_options.assume_yes && !prompt_yes(prompt)) {
        Err(CliError::AbortedError)
    } else {
        Ok(())
    }
}

pub fn read_from_file(path: &Path) -> CliTypedResult<Vec<u8>> {
    std::fs::read(path)
        .map_err(|e| CliError::UnableToReadFile(format!("{}", path.display()), e.to_string()))
}

/// Write a `&[u8]` to a file
pub fn write_to_file(path: &Path, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
    let mut file = File::create(path).map_err(|e| CliError::IO(name.to_string(), e))?;
    file.write_all(bytes)
        .map_err(|e| CliError::IO(name.to_string(), e))
}

/// Appends a file extension to a `Path` without overwriting the original extension.
pub fn append_file_extension(
    file: &Path,
    appended_extension: &'static str,
) -> CliTypedResult<PathBuf> {
    let extension = file
        .extension()
        .map(|extension| extension.to_str().unwrap_or_default());
    if let Some(extension) = extension {
        Ok(file.with_extension(extension.to_owned() + "." + appended_extension))
    } else {
        Ok(file.with_extension(appended_extension))
    }
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

/// Error message for parsing a map
const PARSE_MAP_SYNTAX_MSG: &str = "Invalid syntax for map. Example: Name=Value,Name2=Value";

/// Parses an inline map of values
///
/// Example: Name=Value,Name2=Value
pub fn parse_map<K: FromStr + Ord, V: FromStr>(str: &str) -> anyhow::Result<BTreeMap<K, V>>
where
    K::Err: 'static + std::error::Error + Send + Sync,
    V::Err: 'static + std::error::Error + Send + Sync,
{
    let mut map = BTreeMap::new();

    // Split pairs by commas
    for pair in str.split_terminator(',') {
        // Split pairs by = then trim off any spacing
        let (first, second): (&str, &str) = pair
            .split_terminator('=')
            .collect_tuple()
            .ok_or_else(|| anyhow::Error::msg(PARSE_MAP_SYNTAX_MSG))?;
        let first = first.trim();
        let second = second.trim();
        if first.is_empty() || second.is_empty() {
            return Err(anyhow::Error::msg(PARSE_MAP_SYNTAX_MSG));
        }

        // At this point, we just give error messages appropriate to parsing
        let key: K = K::from_str(first)?;
        let value: V = V::from_str(second)?;
        map.insert(key, value);
    }
    Ok(map)
}

/// Submits a [`TransactionPayload`] as signed by the `sender_key`
pub async fn submit_transaction(
    url: Url,
    chain_id: ChainId,
    sender_key: Ed25519PrivateKey,
    payload: TransactionPayload,
    max_gas: u64,
) -> CliTypedResult<Transaction> {
    let client = Client::new(url);

    // Get sender address
    let sender_address = AuthenticationKey::ed25519(&sender_key.public_key()).derived_address();
    let sender_address = AccountAddress::new(*sender_address);

    // Get sequence number for account
    let sequence_number = get_sequence_number(&client, sender_address).await?;

    // Sign and submit transaction
    let transaction_factory = TransactionFactory::new(chain_id)
        .with_gas_unit_price(1)
        .with_max_gas_amount(max_gas);
    let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
    let transaction =
        sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
    let response = client
        .submit_and_wait(&transaction)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;

    Ok(response.into_inner())
}

pub fn current_dir() -> PathBuf {
    env::current_dir().unwrap()
}

/// Reads a line from input
pub fn read_line(input_name: &'static str) -> CliTypedResult<String> {
    let mut input_buf = String::new();
    let _ = std::io::stdin()
        .read_line(&mut input_buf)
        .map_err(|err| CliError::IO(input_name.to_string(), err))?;

    Ok(input_buf)
}
