// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::logging::AdapterLogSchema;
use aptos_logger::prelude::*;
use aptos_types::account_config::TransactionValidation;
use move_deps::{
    move_binary_format::errors::VMError,
    move_core_types::vm_status::{StatusCode, VMStatus},
};

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

const INVALID_ARGUMENT: u8 = 1;
const LIMIT_EXCEEDED: u8 = 2;

fn error_split(code: u64) -> (u8, u64) {
    let reason = code & 0xffff;
    let category = ((code >> 16) & 0xff) as u8;
    (category, reason)
}

/// Converts particular Move abort codes to specific validation error codes for the prologue
/// Any non-abort non-execution code is considered an invariant violation, specifically
/// `UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION`
pub fn convert_prologue_error(
    transaction_validation: &TransactionValidation,
    error: VMError,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    let status = error.into_vm_status();
    Err(match status {
        VMStatus::Executed => VMStatus::Executed,
        VMStatus::MoveAbort(location, code)
            if !transaction_validation.is_account_module_abort(&location) =>
        {
            let (category, reason) = error_split(code);
            log_context.alert();
            error!(
                *log_context,
                "[aptos_vm] Unexpected prologue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                location, code, category, reason,
            );
            VMStatus::Error(StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION)
        }
        VMStatus::MoveAbort(location, code) => {
            let new_major_status = match error_split(code) {
                // Invalid authentication key
                (INVALID_ARGUMENT, EBAD_ACCOUNT_AUTHENTICATION_KEY) => StatusCode::INVALID_AUTH_KEY,
                // Sequence number too old
                (INVALID_ARGUMENT, ESEQUENCE_NUMBER_TOO_OLD) => StatusCode::SEQUENCE_NUMBER_TOO_OLD,
                // Sequence number too new
                (INVALID_ARGUMENT, ESEQUENCE_NUMBER_TOO_NEW) => StatusCode::SEQUENCE_NUMBER_TOO_NEW,
                // Sequence number too new
                (INVALID_ARGUMENT, EACCOUNT_DOES_NOT_EXIST) => {
                    StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST
                }
                // Can't pay for transaction gas deposit/fee
                (INVALID_ARGUMENT, ECANT_PAY_GAS_DEPOSIT) => {
                    StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE
                }
                (INVALID_ARGUMENT, ETRANSACTION_EXPIRED) => StatusCode::TRANSACTION_EXPIRED,
                (INVALID_ARGUMENT, EBAD_CHAIN_ID) => StatusCode::BAD_CHAIN_ID,
                // Sequence number will overflow
                (LIMIT_EXCEEDED, ESEQUENCE_NUMBER_TOO_BIG) => StatusCode::SEQUENCE_NUMBER_TOO_BIG,
                (INVALID_ARGUMENT, ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH) => {
                    StatusCode::SECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH
                }
                (category, reason) => {
                    log_context.alert();
                    error!(
                        *log_context,
                        "[aptos_vm] Unexpected prologue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                        location, code, category, reason,
                    );
                    return Err(VMStatus::Error(
                        StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                    ));
                }
            };
            VMStatus::Error(new_major_status)
        }
        status @ VMStatus::ExecutionFailure { .. } | status @ VMStatus::Error(_) => {
            log_context.alert();
            error!(
                *log_context,
                "[aptos_vm] Unexpected prologue error: {:?}", status
            );
            VMStatus::Error(StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION)
        }
    })
}

/// Checks for only Move aborts or successful execution.
/// Any other errors are mapped to the invariant violation
/// `UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION`
pub fn convert_epilogue_error(
    transaction_validation: &TransactionValidation,
    error: VMError,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    let status = error.into_vm_status();
    Err(match status {
        VMStatus::Executed => VMStatus::Executed,
        VMStatus::MoveAbort(location, code)
            if !transaction_validation.is_account_module_abort(&location) =>
        {
            let (category, reason) = error_split(code);
            log_context.alert();
            error!(
                *log_context,
                "[aptos_vm] Unexpected success epilogue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                location, code, category, reason,
            );
            VMStatus::Error(StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION)
        }

        VMStatus::MoveAbort(location, code) => match error_split(code) {
            (LIMIT_EXCEEDED, ECANT_PAY_GAS_DEPOSIT) => VMStatus::MoveAbort(location, code),
            (category, reason) => {
                log_context.alert();
                error!(
                    *log_context,
                    "[aptos_vm] Unexpected success epilogue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                    location, code, category, reason,
                );
                VMStatus::Error(StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION)
            }
        },

        status => {
            log_context.alert();
            error!(
                *log_context,
                "[aptos_vm] Unexpected success epilogue error: {:?}", status,
            );
            VMStatus::Error(StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION)
        }
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

        status => {
            log_context.alert();
            error!(
                *log_context,
                "[aptos_vm] Unexpected error from known Move function, '{}'. Error: {:?}",
                function_name,
                status,
            );
            VMStatus::Error(StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION)
        }
    })
}
