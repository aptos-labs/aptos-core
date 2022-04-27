// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{CliError, CliTypedResult},
    CliResult,
};
use aptos_telemetry::constants::APTOS_CLI_PUSH_METRICS;
use move_core_types::account_address::AccountAddress;
use serde::Serialize;
use shadow_rs::shadow;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

shadow!(build);

/// Prompts for confirmation until a yes or no is given explicitly
/// TODO: Capture interrupts
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

/// Convert an empty response to Success
pub async fn to_common_success_result(command: &str, result: CliTypedResult<()>) -> CliResult {
    to_common_result(command, result.map(|()| "Success")).await
}

/// For pretty printing outputs in JSON
pub async fn to_common_result<T: Serialize>(command: &str, result: CliTypedResult<T>) -> CliResult {
    let is_err = result.is_err();
    let error = if let Err(ref e) = result {
        e.to_str()
    } else {
        "None"
    };
    let metrics = collect_metrics(command, !is_err, error);
    aptos_telemetry::send_data(
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
fn collect_metrics(command: &str, successful: bool, error: &str) -> HashMap<String, String> {
    let mut metrics = HashMap::new();
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

/// Checks if a file exists, being overridden by `--assume-yes`
pub fn check_if_file_exists(file: &Path, assume_yes: bool) -> CliTypedResult<()> {
    if file.exists()
        && !assume_yes
        && !prompt_yes(
            format!(
                "{:?} already exists, are you sure you want to overwrite it?",
                file.as_os_str()
            )
            .as_str(),
        )
    {
        Err(CliError::AbortedError)
    } else {
        Ok(())
    }
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
