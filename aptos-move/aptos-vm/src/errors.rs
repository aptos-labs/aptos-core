// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_validation::APTOS_TRANSACTION_VALIDATION;
use aptos_logger::{enabled, Level};
use aptos_types::transaction::TransactionStatus;
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::output::VMOutput;
use move_binary_format::errors::VMError;
use move_core_types::vm_status::{StatusCode, VMStatus};

/// Error codes that can be emitted by the prologue. These have special significance to the VM when
/// they are raised during the prologue.
/// These errors are only expected from the module that is registered as the account module for the system.
/// The prologue should not emit any other error codes or fail for any reason, doing so will result
/// in the VM throwing an invariant violation
// Auth key in transaction is invalid.
pub const EBAD_ACCOUNT_AUTHENTICATION_KEY: u64 = 1001;
// Transaction sequence number is too old.
pub const ESEQUENCE_NUMBER_TOO_OLD: u64 = 1002;
// Transaction sequence number is too new.
pub const ESEQUENCE_NUMBER_TOO_NEW: u64 = 1003;
// Transaction sender's account does not exist.
pub const EACCOUNT_DOES_NOT_EXIST: u64 = 1004;
// Insufficient balance (to pay for gas deposit).
pub const ECANT_PAY_GAS_DEPOSIT: u64 = 1005;
// Transaction expiration time exceeds block time.
pub const ETRANSACTION_EXPIRED: u64 = 1006;
// chain_id in transaction doesn't match the one on-chain.
pub const EBAD_CHAIN_ID: u64 = 1007;
// Transaction sequence number exceeds u64 max.
pub const ESEQUENCE_NUMBER_TOO_BIG: u64 = 1008;
// Counts of secondary keys and addresses don't match.
pub const ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH: u64 = 1009;
// Gas payer account missing in gas payer tx
pub const EGAS_PAYER_ACCOUNT_MISSING: u64 = 1010;
// Insufficient balance to cover the required deposit.
pub const EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT: u64 = 1011;
// Nonce in orderless transaction already used in a previous transaction.
pub const ENONCE_ALREADY_USED: u64 = 1012;
// Expiration time for orderless transaction is too far in future.
// An orderless transaction should expire within 60 seconds.
pub const ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE: u64 = 1013;

// Specified account is not a multisig account.
const EACCOUNT_NOT_MULTISIG: u64 = 2002;
// Account executing this operation is not an owner of the multisig account.
const ENOT_MULTISIG_OWNER: u64 = 2003;
// Multisig transaction with specified id cannot be found.
const EMULTISIG_TRANSACTION_NOT_FOUND: u64 = 2006;
// Provided target function does not match the hash stored in the on-chain multisig transaction.
const EMULTISIG_PAYLOAD_DOES_NOT_MATCH_HASH: u64 = 2008;
// Multisig transaction has not received enough approvals to be executed.
const EMULTISIG_NOT_ENOUGH_APPROVALS: u64 = 2009;
// Provided target function does not match the payload stored in the on-chain transaction.
const EPAYLOAD_DOES_NOT_MATCH: u64 = 2010;

const INVALID_ARGUMENT: u8 = 0x1;
const LIMIT_EXCEEDED: u8 = 0x2;
const INVALID_STATE: u8 = 0x3;
const PERMISSION_DENIED: u8 = 0x5;
const NOT_FOUND: u8 = 0x6;

fn error_split(code: u64) -> (u8, u64) {
    let reason = code & 0xFFFF;
    let category = ((code >> 16) & 0xFF) as u8;
    (category, reason)
}

/// Converts particular Move abort codes to specific validation error codes for the prologue
/// Any non-abort non-execution code is considered an invariant violation, specifically
/// `UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION`
pub fn convert_prologue_error(
    error: VMError,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    let status = error.into_vm_status();
    Err(match status {
        VMStatus::Executed => VMStatus::Executed,
        VMStatus::MoveAbort(location, code)
            if !APTOS_TRANSACTION_VALIDATION.is_account_module_abort(&location) =>
        {
            let new_major_status = match error_split(code) {
                // TODO: Update these after adding the appropriate error codes into StatusCode
                // in the Move repo.
                (INVALID_STATE, EACCOUNT_NOT_MULTISIG) => StatusCode::ACCOUNT_NOT_MULTISIG,
                (PERMISSION_DENIED, ENOT_MULTISIG_OWNER) => StatusCode::NOT_MULTISIG_OWNER,
                (NOT_FOUND, EMULTISIG_TRANSACTION_NOT_FOUND) => {
                    StatusCode::MULTISIG_TRANSACTION_NOT_FOUND
                },
                (INVALID_ARGUMENT, EMULTISIG_NOT_ENOUGH_APPROVALS) => {
                    StatusCode::MULTISIG_TRANSACTION_INSUFFICIENT_APPROVALS
                },
                (INVALID_ARGUMENT, EMULTISIG_PAYLOAD_DOES_NOT_MATCH_HASH) => {
                    StatusCode::MULTISIG_TRANSACTION_PAYLOAD_DOES_NOT_MATCH_HASH
                },
                (INVALID_ARGUMENT, EPAYLOAD_DOES_NOT_MATCH) => {
                    StatusCode::MULTISIG_TRANSACTION_PAYLOAD_DOES_NOT_MATCH
                },
                (category, reason) => {
                    let err_msg = format!("[aptos_vm] Unexpected prologue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                    location, code, category, reason);
                    speculative_error!(log_context, err_msg.clone());
                    return Err(VMStatus::error(
                        StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                        Some(err_msg),
                    ));
                },
            };
            VMStatus::error(new_major_status, None)
        },
        VMStatus::MoveAbort(location, code) => {
            let new_major_status = match error_split(code) {
                // Invalid authentication key
                (INVALID_ARGUMENT, EBAD_ACCOUNT_AUTHENTICATION_KEY) => StatusCode::INVALID_AUTH_KEY,
                // Sequence number too old
                (INVALID_ARGUMENT, ESEQUENCE_NUMBER_TOO_OLD) => StatusCode::SEQUENCE_NUMBER_TOO_OLD,
                // Sequence number too new
                (INVALID_ARGUMENT, ESEQUENCE_NUMBER_TOO_NEW) => StatusCode::SEQUENCE_NUMBER_TOO_NEW,
                (INVALID_ARGUMENT, EACCOUNT_DOES_NOT_EXIST) => {
                    StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST
                },
                // Can't pay for transaction gas deposit/fee
                (INVALID_ARGUMENT, ECANT_PAY_GAS_DEPOSIT) => {
                    StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE
                },
                (INVALID_ARGUMENT, ETRANSACTION_EXPIRED) => StatusCode::TRANSACTION_EXPIRED,
                (INVALID_ARGUMENT, EBAD_CHAIN_ID) => StatusCode::BAD_CHAIN_ID,
                // Sequence number will overflow
                (LIMIT_EXCEEDED, ESEQUENCE_NUMBER_TOO_BIG) => StatusCode::SEQUENCE_NUMBER_TOO_BIG,
                (INVALID_ARGUMENT, ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH) => {
                    StatusCode::SECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH
                },
                (INVALID_ARGUMENT, EGAS_PAYER_ACCOUNT_MISSING) => {
                    StatusCode::GAS_PAYER_ACCOUNT_MISSING
                },
                (INVALID_STATE, EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT) => {
                    StatusCode::INSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT
                },
                // Nonce in orderless transaction is already used in a previous transaction
                (INVALID_ARGUMENT, ENONCE_ALREADY_USED) => StatusCode::NONCE_ALREADY_USED,
                (INVALID_ARGUMENT, ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE) => {
                    StatusCode::TRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE
                },
                (category, reason) => {
                    let err_msg = format!("[aptos_vm] Unexpected prologue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                    location, code, category, reason);
                    speculative_error!(log_context, err_msg.clone());
                    return Err(VMStatus::Error {
                        status_code: StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                        sub_status: None,
                        message: Some(err_msg),
                    });
                },
            };
            VMStatus::error(new_major_status, None)
        },
        // Speculative errors are returned for caller to handle.
        e @ VMStatus::Error {
            status_code:
                StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
                | StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
            ..
        } => e,
        status @ VMStatus::ExecutionFailure { .. } | status @ VMStatus::Error { .. } => {
            speculative_error!(
                log_context,
                format!("[aptos_vm] Unexpected prologue error: {:?}", status),
            );
            VMStatus::Error {
                status_code: StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                sub_status: status.sub_status(),
                message: None,
            }
        },
    })
}

/// Checks for only Move aborts or successful execution.
/// Any other errors are mapped to the invariant violation
/// `UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION`
pub fn convert_epilogue_error(
    error: VMError,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    let status = error.into_vm_status();
    Err(match status {
        VMStatus::Executed => VMStatus::Executed,
        VMStatus::MoveAbort(location, code)
            if !APTOS_TRANSACTION_VALIDATION.is_account_module_abort(&location) =>
        {
            let (category, reason) = error_split(code);
            let err_msg = format!("[aptos_vm] Unexpected success epilogue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
			location, code, category, reason);
            speculative_error!(log_context, err_msg.clone());
            VMStatus::error(
                StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                Some(err_msg),
            )
        },

        VMStatus::MoveAbort(location, code) => match error_split(code) {
            (LIMIT_EXCEEDED, ECANT_PAY_GAS_DEPOSIT) => VMStatus::MoveAbort(location, code),
            (category, reason) => {
                let err_msg = format!("[aptos_vm] Unexpected success epilogue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
			    location, code, category, reason);
                speculative_error!(log_context, err_msg.clone());
                VMStatus::error(
                    StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                    Some(err_msg),
                )
            },
        },
        // Speculative errors are returned for caller to handle.
        e @ VMStatus::Error {
            status_code:
                StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
                | StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
            ..
        } => e,
        status => {
            let err_msg = format!("[aptos_vm] Unexpected success epilogue error: {:?}", status);
            speculative_error!(log_context, err_msg.clone());
            VMStatus::Error {
                status_code: StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                sub_status: status.sub_status(),
                message: Some(err_msg),
            }
        },
    })
}

/// Checks for only successful execution
/// Any errors are mapped to the invariant violation
/// `UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION`
pub fn expect_only_successful_execution(
    error: VMError,
    function_name: &str,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    let status = error.into_vm_status();
    Err(match status {
        VMStatus::Executed => VMStatus::Executed,
        // Speculative errors are returned for caller to handle.
        e @ VMStatus::Error {
            status_code:
                StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
                | StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
            ..
        } => e,
        status => {
            // Only trigger a warning here as some errors could be a result of the speculative parallel execution.
            // We will report the errors after we obtained the final transaction output in update_counters_for_processed_chunk
            let err_msg = format!(
                "[aptos_vm] Unexpected error from known Move function, '{}'. Error: {:?}",
                function_name, status
            );
            speculative_warn!(log_context, err_msg.clone());
            VMStatus::Error {
                status_code: StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                sub_status: status.sub_status(),
                message: Some(err_msg),
            }
        },
    })
}

pub(crate) fn discarded_output(status_code: StatusCode) -> VMOutput {
    VMOutput::empty_with_status(TransactionStatus::Discard(status_code))
}
