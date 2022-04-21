// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{
        account_address_from_public_key, CliError, CliTypedResult, EncodingOptions,
        WriteTransactionOptions,
    },
    CliResult,
};
use aptos_crypto::PrivateKey;
use aptos_rest_client::{Client, Response, Transaction};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::transaction::TransactionPayload;
use move_core_types::account_address::AccountAddress;
use serde::Serialize;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

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
pub fn to_common_success_result(result: CliTypedResult<()>) -> CliResult {
    to_common_result(result.map(|()| "Success"))
}

/// For pretty printing outputs in JSON
pub fn to_common_result<T: Serialize>(result: CliTypedResult<T>) -> CliResult {
    let is_err = result.is_err();
    let result: ResultWrapper<T> = result.into();
    let string = serde_json::to_string_pretty(&result).unwrap();
    if is_err {
        Err(string)
    } else {
        Ok(string)
    }
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

/// Retrieves the current sequence number
pub async fn get_sequence_number(
    rest_client: &Client,
    account: AccountAddress,
) -> CliTypedResult<u64> {
    let account_response = rest_client
        .get_account(account)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    Ok(account_response.inner().sequence_number)
}

/// Sends a signed transaction and waits for a response
pub async fn send_transaction(
    encoding_options: EncodingOptions,
    write_options: WriteTransactionOptions,
    transaction_payload: TransactionPayload,
) -> Result<Response<Transaction>, CliError> {
    let rest_client = Client::new(write_options.rest_options.url);
    let sender_key = write_options
        .private_key_options
        .extract_private_key(encoding_options.encoding)?;
    let sender_public_key = sender_key.public_key();
    let sender_address = account_address_from_public_key(&sender_public_key);
    let sequence_number = get_sequence_number(&rest_client, sender_address).await?;

    let transaction_factory = TransactionFactory::new(write_options.chain_id)
        .with_gas_unit_price(1)
        .with_max_gas_amount(write_options.max_gas);
    let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
    let transaction = sender_account
        .sign_with_transaction_builder(transaction_factory.payload(transaction_payload));
    rest_client
        .submit_and_wait(&transaction)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))
}
