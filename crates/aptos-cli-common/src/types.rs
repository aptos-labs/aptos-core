// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Core CLI types: error types, result aliases, the `CliCommand` trait,
//! `TransactionOptions`, `TransactionSummary`, and related definitions.

use aptos_crypto::encoding_type::EncodingError;
use aptos_logger::Level;
use aptos_rest_client::{aptos_api_types::HashValue, error::RestError, Transaction};
use aptos_types::transaction::ReplayProtector;
use async_trait::async_trait;
use clap::ValueEnum;
use hex::FromHexError;
use indoc::indoc;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    time::Instant,
};
use thiserror::Error;

pub const US_IN_SECS: u64 = 1_000_000;
pub const ACCEPTED_CLOCK_SKEW_US: u64 = 5 * US_IN_SECS;
pub const DEFAULT_EXPIRATION_SECS: u64 = 30;
pub const DEFAULT_PROFILE: &str = "default";
pub const GIT_IGNORE: &str = ".gitignore";

pub const APTOS_FOLDER_GIT_IGNORE: &str = indoc! {"
    *
    testnet/
    config.yaml
"};
pub const MOVE_FOLDER_GIT_IGNORE: &str = indoc! {"
  .aptos/
  build/
  .coverage_map.mvcov
  .trace"
};

pub const CONFIG_FOLDER: &str = ".aptos";

/// A common result to be returned to users
pub type CliResult = Result<String, String>;

/// A common result to remove need for typing `Result<T, CliError>`
pub type CliTypedResult<T> = Result<T, CliError>;

/// CLI Errors for reporting through telemetry and outputs
#[derive(Debug, Error)]
pub enum CliError {
    #[error("Aborted command")]
    AbortedError,
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Error (de)serializing '{0}': {1}")]
    BCS(&'static str, #[source] bcs::Error),
    #[error("Invalid arguments: {0}")]
    CommandArgumentError(String),
    #[error("Unable to load config: {0} {1}")]
    ConfigLoadError(String, String),
    #[error("Unable to find config {0}, have you run `aptos init`?")]
    ConfigNotFoundError(String),
    #[error("Error accessing '{0}': {1}")]
    IO(String, #[source] std::io::Error),
    #[error("Move compilation failed: {0}")]
    MoveCompilationError(String),
    #[error("Move unit tests failed")]
    MoveTestError,
    #[error("Move Prover failed: {0}")]
    MoveProverError(String),
    #[error(
        "The package is larger than {1} bytes ({0} bytes)! \
        To lower the size you may want to include less artifacts via `--included-artifacts`. \
        You can also override this check with `--override-size-check`. \
        Alternatively, you can use the `--chunked-publish` to enable chunked publish mode, \
        which chunks down the package and deploys it in several stages."
    )]
    PackageSizeExceeded(usize, usize),
    #[error("Unable to parse '{0}': error: {1}")]
    UnableToParse(&'static str, String),
    #[error("Unable to read file '{0}', error: {1}")]
    UnableToReadFile(String, String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
    #[error("Simulation failed with status: {0}")]
    SimulationError(String),
    #[error("Coverage failed with status: {0}")]
    CoverageError(String),
    #[error("Type {0} is a struct, not an enum. Use struct syntax instead.")]
    StructNotEnumError(String),
}

impl CliError {
    pub fn to_str(&self) -> &'static str {
        match self {
            CliError::AbortedError => "AbortedError",
            CliError::ApiError(_) => "ApiError",
            CliError::BCS(_, _) => "BCS",
            CliError::CommandArgumentError(_) => "CommandArgumentError",
            CliError::ConfigLoadError(_, _) => "ConfigLoadError",
            CliError::ConfigNotFoundError(_) => "ConfigNotFoundError",
            CliError::IO(_, _) => "IO",
            CliError::MoveCompilationError(_) => "MoveCompilationError",
            CliError::MoveTestError => "MoveTestError",
            CliError::MoveProverError(_) => "MoveProverError",
            CliError::PackageSizeExceeded(_, _) => "PackageSizeExceeded",
            CliError::UnableToParse(_, _) => "UnableToParse",
            CliError::UnableToReadFile(_, _) => "UnableToReadFile",
            CliError::UnexpectedError(_) => "UnexpectedError",
            CliError::SimulationError(_) => "SimulationError",
            CliError::CoverageError(_) => "CoverageError",
            CliError::StructNotEnumError(_) => "StructNotEnumError",
        }
    }
}

impl From<RestError> for CliError {
    fn from(e: RestError) -> Self {
        CliError::ApiError(e.to_string())
    }
}

impl From<aptos_config::config::Error> for CliError {
    fn from(e: aptos_config::config::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<aptos_github_client::Error> for CliError {
    fn from(e: aptos_github_client::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<serde_yaml::Error> for CliError {
    fn from(e: serde_yaml::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<base64::DecodeError> for CliError {
    fn from(e: base64::DecodeError) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for CliError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<aptos_crypto::CryptoMaterialError> for CliError {
    fn from(e: aptos_crypto::CryptoMaterialError) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<hex::FromHexError> for CliError {
    fn from(e: FromHexError) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<anyhow::Error> for CliError {
    fn from(e: anyhow::Error) -> Self {
        CliError::UnexpectedError(format!("{:#}", e))
    }
}

impl From<bcs::Error> for CliError {
    fn from(e: bcs::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<aptos_ledger::AptosLedgerError> for CliError {
    fn from(e: aptos_ledger::AptosLedgerError) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<EncodingError> for CliError {
    fn from(e: EncodingError) -> Self {
        match e {
            EncodingError::BCS(s, e) => CliError::BCS(s, e),
            EncodingError::UnableToParse(s, e) => CliError::UnableToParse(s, e),
            EncodingError::UnableToReadFile(s, e) => CliError::UnableToReadFile(s, e),
            EncodingError::UTF8(s) => CliError::UnexpectedError(s),
        }
    }
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<reqwest::Error> for CliError {
    fn from(e: reqwest::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

/// Types of Keys used by the blockchain
#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum KeyType {
    /// Ed25519 key used for signing
    Ed25519,
    /// X25519 key used for network handshakes and identity
    X25519,
    /// A BLS12381 key for consensus
    Bls12381,
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            KeyType::Ed25519 => "ed25519",
            KeyType::X25519 => "x25519",
            KeyType::Bls12381 => "bls12381",
        };
        write!(f, "{}", str)
    }
}

impl FromStr for KeyType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ed25519" => Ok(KeyType::Ed25519),
            "x25519" => Ok(KeyType::X25519),
            "bls12381" => Ok(KeyType::Bls12381),
            _ => Err("Invalid key type: Must be one of [ed25519, x25519]"),
        }
    }
}

/// A common trait for all CLI commands to have consistent outputs
#[async_trait]
pub trait CliCommand<T: Serialize + Send>: Sized + Send {
    /// Returns a name for logging purposes
    fn command_name(&self) -> &'static str;

    /// Returns whether the error should be JSONifyed.
    fn jsonify_error_output(&self) -> bool {
        true
    }

    /// Executes the command, returning a command specific type
    async fn execute(self) -> CliTypedResult<T>;

    /// Executes the command, and serializes it to the common JSON output type
    async fn execute_serialized(self) -> CliResult {
        self.execute_serialized_with_logging_level(Level::Warn)
            .await
    }

    /// Execute the command with customized logging level
    async fn execute_serialized_with_logging_level(self, level: Level) -> CliResult {
        let command_name = self.command_name();
        crate::start_logger(level);
        let start_time = Instant::now();
        let jsonify_error_output = self.jsonify_error_output();
        crate::to_common_result(
            command_name,
            start_time,
            self.execute().await,
            jsonify_error_output,
        )
        .await
    }

    /// Same as execute serialized without setting up logging
    async fn execute_serialized_without_logger(self) -> CliResult {
        let command_name = self.command_name();
        let start_time = Instant::now();
        let jsonify_error_output = self.jsonify_error_output();
        crate::to_common_result(
            command_name,
            start_time,
            self.execute().await,
            jsonify_error_output,
        )
        .await
    }

    /// Executes the command, and throws away Ok(result) for the string Success
    async fn execute_serialized_success(self) -> CliResult {
        crate::start_logger(Level::Warn);
        let command_name = self.command_name();
        let start_time = Instant::now();
        let jsonify_error_output = self.jsonify_error_output();
        crate::to_common_success_result(
            command_name,
            start_time,
            self.execute().await,
            jsonify_error_output,
        )
        .await
    }
}

/// A shortened transaction output
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransactionSummary {
    pub transaction_hash: HashValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_unit_price: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<AccountAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_protector: Option<ReplayProtector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_status: Option<String>,

    /// The address of the deployed code object. Only present for code object deployment transactions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployed_object_address: Option<AccountAddress>,
}

impl From<Transaction> for TransactionSummary {
    fn from(transaction: Transaction) -> Self {
        TransactionSummary::from(&transaction)
    }
}

impl From<&Transaction> for TransactionSummary {
    fn from(transaction: &Transaction) -> Self {
        match transaction {
            Transaction::PendingTransaction(txn) => TransactionSummary {
                transaction_hash: txn.hash,
                pending: Some(true),
                sender: Some(*txn.request.sender.inner()),
                sequence_number: match txn.request.replay_protector() {
                    ReplayProtector::SequenceNumber(sequence_number) => Some(sequence_number),
                    _ => None,
                },
                replay_protector: Some(txn.request.replay_protector()),
                gas_used: None,
                gas_unit_price: None,
                success: None,
                version: None,
                vm_status: None,
                timestamp_us: None,
                deployed_object_address: None,
            },
            Transaction::UserTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                sender: Some(*txn.request.sender.inner()),
                gas_used: Some(txn.info.gas_used.0),
                gas_unit_price: Some(txn.request.gas_unit_price.0),
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                sequence_number: match txn.request.replay_protector() {
                    ReplayProtector::SequenceNumber(sequence_number) => Some(sequence_number),
                    _ => None,
                },
                replay_protector: Some(txn.request.replay_protector()),
                timestamp_us: Some(txn.timestamp.0),
                pending: None,
                deployed_object_address: None,
            },
            Transaction::GenesisTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                sender: None,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sequence_number: None,
                replay_protector: None,
                timestamp_us: None,
                deployed_object_address: None,
            },
            Transaction::BlockMetadataTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                timestamp_us: Some(txn.timestamp.0),
                sender: None,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sequence_number: None,
                replay_protector: None,
                deployed_object_address: None,
            },
            Transaction::StateCheckpointTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                timestamp_us: Some(txn.timestamp.0),
                sender: None,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sequence_number: None,
                replay_protector: None,
                deployed_object_address: None,
            },
            Transaction::BlockEpilogueTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                timestamp_us: Some(txn.timestamp.0),
                sender: None,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sequence_number: None,
                replay_protector: None,
                deployed_object_address: None,
            },
            Transaction::ValidatorTransaction(txn) => TransactionSummary {
                transaction_hash: txn.transaction_info().hash,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sender: None,
                sequence_number: None,
                replay_protector: None,
                success: Some(txn.transaction_info().success),
                timestamp_us: Some(txn.timestamp().0),
                version: Some(txn.transaction_info().version.0),
                vm_status: Some(txn.transaction_info().vm_status.clone()),
                deployed_object_address: None,
            },
        }
    }
}

/// A summary of a `WriteSetChange` for easy printing
#[derive(Clone, Debug, Default, Serialize)]
pub struct ChangeSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<AccountAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}
