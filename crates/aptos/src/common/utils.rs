// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{CliError, CliTypedResult, PromptOptions},
    CliResult,
};
use aptos_build_info::build_information;
use aptos_crypto::HashValue;
use aptos_logger::{debug, Level};
use aptos_rest_client::{Account, Client};
use aptos_types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey};
use itertools::Itertools;
use move_deps::move_core_types::account_address::AccountAddress;
use reqwest::Url;
use serde::Serialize;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::BTreeMap,
    env,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, Instant},
};

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

pub fn cli_build_information() -> BTreeMap<String, String> {
    build_information!()
}

/// Sends a telemetry event about the CLI build, command and result
async fn send_telemetry_event(
    command: &str,
    latency: Duration,
    success: bool,
    error: Option<String>,
) {
    // Collect the build information
    let build_information = cli_build_information();

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
    write_to_file_with_opts(path, name, bytes, &mut OpenOptions::new())
}

/// Write a User only read / write file
pub fn write_to_user_only_file(path: &Path, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
    let mut opts = OpenOptions::new();
    #[cfg(unix)]
    opts.mode(0o600);
    write_to_file_with_opts(path, name, bytes, &mut opts)
}

/// Write a `&[u8]` to a file with the given options
pub fn write_to_file_with_opts(
    path: &Path,
    name: &str,
    bytes: &[u8],
    opts: &mut OpenOptions,
) -> CliTypedResult<()> {
    let mut file = opts
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|e| CliError::IO(name.to_string(), e))?;
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

/// Retrieves account resource from the rest client
pub async fn get_account(
    client: &aptos_rest_client::Client,
    address: AccountAddress,
) -> CliTypedResult<Account> {
    let account_response = client
        .get_account(address)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    let account = account_response.inner();
    Ok(account.clone())
}

/// Retrieves sequence number from the rest client
pub async fn get_sequence_number(
    client: &aptos_rest_client::Client,
    address: AccountAddress,
) -> CliTypedResult<u64> {
    Ok(get_account(client, address).await?.sequence_number)
}

/// Retrieves the auth key from the rest client
pub async fn get_auth_key(
    client: &aptos_rest_client::Client,
    address: AccountAddress,
) -> CliTypedResult<AuthenticationKey> {
    Ok(get_account(client, address).await?.authentication_key)
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

pub fn current_dir() -> CliTypedResult<PathBuf> {
    env::current_dir().map_err(|err| {
        CliError::UnexpectedError(format!("Failed to get current directory {}", err))
    })
}

pub fn dir_default_to_current(maybe_dir: Option<PathBuf>) -> CliTypedResult<PathBuf> {
    if let Some(dir) = maybe_dir {
        Ok(dir)
    } else {
        current_dir()
    }
}

pub fn create_dir_if_not_exist(dir: &Path) -> CliTypedResult<()> {
    // Check if the directory exists, if it's not a dir, it will also fail here
    if !dir.exists() || !dir.is_dir() {
        std::fs::create_dir_all(dir).map_err(|e| CliError::IO(dir.display().to_string(), e))?;
        debug!("Created {} folder", dir.display());
    } else {
        debug!("{} folder already exists", dir.display());
    }
    Ok(())
}

/// Reads a line from input
pub fn read_line(input_name: &'static str) -> CliTypedResult<String> {
    let mut input_buf = String::new();
    let _ = std::io::stdin()
        .read_line(&mut input_buf)
        .map_err(|err| CliError::IO(input_name.to_string(), err))?;

    Ok(input_buf)
}

/// Fund account (and possibly create it) from a faucet
pub async fn fund_account(
    faucet_url: Url,
    num_octas: u64,
    address: AccountAddress,
) -> CliTypedResult<Vec<HashValue>> {
    let response = reqwest::Client::new()
        .post(format!(
            "{}mint?amount={}&auth_key={}",
            faucet_url, num_octas, address
        ))
        .send()
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    if response.status() == 200 {
        let hashes: Vec<HashValue> = response
            .json()
            .await
            .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
        Ok(hashes)
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
