// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_types::vm_status::StatusCode;
use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

/// This is the generic struct we use for all API errors, it contains a string
/// message and an Aptos API specific error code.
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct AptosError {
    /// A message describing the error
    pub message: String,
    pub error_code: AptosErrorCode,
    /// A code providing VM error details when submitting transactions to the VM
    pub vm_error_code: Option<u64>,
}

impl std::fmt::Display for AptosError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error({:?}): {:#}", self.error_code, self.message)
    }
}

impl std::error::Error for AptosError {}

impl AptosError {
    pub fn new_with_error_code<ErrorType: std::fmt::Display>(
        error: ErrorType,
        error_code: AptosErrorCode,
    ) -> AptosError {
        Self {
            message: format!("{:#}", error),
            error_code,
            vm_error_code: None,
        }
    }

    pub fn new_with_vm_status<ErrorType: std::fmt::Display>(
        error: ErrorType,
        error_code: AptosErrorCode,
        vm_error_code: StatusCode,
    ) -> AptosError {
        Self {
            message: format!("{:#}", error),
            error_code,
            vm_error_code: Some(vm_error_code as u64),
        }
    }
}

/// These codes provide more granular error information beyond just the HTTP
/// status code of the response.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Enum)]
#[oai(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[repr(u32)]
pub enum AptosErrorCode {
    /// Account not found at the requested version
    AccountNotFound = 101,
    /// Resource not found at the requested version
    ResourceNotFound = 102,
    /// Module not found at the requested version
    ModuleNotFound = 103,
    /// Struct field not found at the requested version
    StructFieldNotFound = 104,
    /// Ledger version not found at the requested version
    ///
    /// Usually means that the version is ahead of the latest version
    VersionNotFound = 105,
    /// Transaction not found at the requested version or with the requested hash
    TransactionNotFound = 106,
    /// Table item not found at the requested version
    TableItemNotFound = 107,
    /// Block not found at the requested version or height
    ///
    /// Usually means the block is fully or partially pruned or the height / version is ahead
    /// of the latest version
    BlockNotFound = 108,
    ///  StateValue not found at the requested version
    StateValueNotFound = 109,

    /// Ledger version is pruned
    VersionPruned = 200,
    /// Block is fully or partially pruned
    BlockPruned = 201,

    /// The API's inputs were invalid
    InvalidInput = 300,

    /// The transaction was an invalid update to an already submitted transaction.
    InvalidTransactionUpdate = 401,
    /// The sequence number for the transaction is behind the latest sequence number.
    SequenceNumberTooOld = 402,
    /// The submitted transaction failed VM checks.
    VmError = 403,

    /// Health check failed.
    HealthCheckFailed = 500,
    /// The mempool is full, no new transactions can be submitted.
    MempoolIsFull = 501,

    /// Internal server error
    InternalError = 600,
    /// Error from the web framework
    WebFrameworkError = 601,
    /// BCS format is not supported on this API.
    BcsNotSupported = 602,
    /// API Disabled
    ApiDisabled = 603,
}

impl AptosErrorCode {
    pub fn as_u32(&self) -> u32 {
        *self as u32
    }
}

#[test]
fn test_serialize_deserialize() {
    let with_code = AptosError::new_with_vm_status(
        "Invalid transaction",
        AptosErrorCode::VmError,
        aptos_types::vm_status::StatusCode::UNKNOWN_MODULE,
    );
    let _: AptosError = bcs::from_bytes(&bcs::to_bytes(&with_code).unwrap()).unwrap();
    let _: AptosError = serde_json::from_str(&serde_json::to_string(&with_code).unwrap()).unwrap();

    let without_code =
        AptosError::new_with_error_code("some message", AptosErrorCode::MempoolIsFull);
    let _: AptosError = bcs::from_bytes(&bcs::to_bytes(&without_code).unwrap()).unwrap();
    let _: AptosError =
        serde_json::from_str(&serde_json::to_string(&without_code).unwrap()).unwrap();
}
