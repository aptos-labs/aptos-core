// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::transaction_validation::APTOS_TRANSACTION_VALIDATION;
use aptos_logger::{enabled, Level};
use aptos_types::{
    error::{
        split_canonical, INVALID_ARGUMENT, INVALID_STATE, NOT_FOUND, OUT_OF_RANGE,
        PERMISSION_DENIED,
    },
    transaction::{
        validation::{
            EACCOUNT_DOES_NOT_EXIST, EBAD_ACCOUNT_AUTHENTICATION_KEY, EBAD_CHAIN_ID,
            ECANT_PAY_GAS_DEPOSIT, EGAS_PAYER_ACCOUNT_MISSING,
            EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT, ENONCE_ALREADY_USED,
            ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH, ESEQUENCE_NUMBER_TOO_BIG,
            ESEQUENCE_NUMBER_TOO_NEW, ESEQUENCE_NUMBER_TOO_OLD,
            ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE, ETRANSACTION_EXPIRED,
        },
        TransactionStatus,
    },
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::output::VMOutput;
use move_binary_format::errors::VMError;
use move_core_types::vm_status::{StatusCode, VMStatus};

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

// Module error codes for transaction_limits.move (must match Move constants).

// No stake pool exists at the specified address.
const ESTAKE_POOL_NOT_FOUND: u64 = 1;
// Sender is not the owner of the specified stake pool.
const ENOT_STAKE_POOL_OWNER: u64 = 2;
// Sender is not the delegated voter of the specified stake pool.
const ENOT_DELEGATED_VOTER: u64 = 3;
// No delegation pool exists at the specified address.
const EDELEGATION_POOL_NOT_FOUND: u64 = 4;
// Sender's committed stake is insufficient for the requested multiplier tier.
const EINSUFFICIENT_STAKE: u64 = 5;
// Multiplier must be > 100 (> 1x).
const EINVALID_MULTIPLIER: u64 = 7;
// Requested multiplier is not available in any configured tier.
const EMULTIPLIER_NOT_AVAILABLE: u64 = 8;
// Stake pool is not in the current-epoch validator set.
const EPOOL_NOT_IN_VALIDATOR_SET: u64 = 9;

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
        VMStatus::MoveAbort {
            location,
            code,
            message,
        } if APTOS_TRANSACTION_VALIDATION.is_transaction_limits_module_abort(&location) => {
            let new_major_status = match split_canonical(code) {
                (PERMISSION_DENIED, ENOT_STAKE_POOL_OWNER) => StatusCode::NOT_STAKE_POOL_OWNER,
                (PERMISSION_DENIED, ENOT_DELEGATED_VOTER) => StatusCode::NOT_DELEGATED_VOTER,
                (PERMISSION_DENIED, EINSUFFICIENT_STAKE) => StatusCode::INSUFFICIENT_STAKE,
                (PERMISSION_DENIED, EPOOL_NOT_IN_VALIDATOR_SET) => {
                    StatusCode::STAKE_POOL_NOT_IN_VALIDATOR_SET
                },
                (NOT_FOUND, ESTAKE_POOL_NOT_FOUND) => StatusCode::STAKE_POOL_NOT_FOUND,
                (NOT_FOUND, EDELEGATION_POOL_NOT_FOUND) => StatusCode::DELEGATION_POOL_NOT_FOUND,
                (INVALID_ARGUMENT, EINVALID_MULTIPLIER) => {
                    StatusCode::INVALID_HIGH_TXN_LIMITS_MULTIPLIER
                },
                (INVALID_ARGUMENT, EMULTIPLIER_NOT_AVAILABLE) => {
                    StatusCode::MULTIPLIER_NOT_AVAILABLE
                },
                (category, reason) => {
                    let mut err_msg = format!(
                        "[aptos_vm] Unexpected prologue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                        location, code, category, reason
                    );
                    if let Some(abort_msg) = message {
                        err_msg.push_str(" Message: ");
                        err_msg.push_str(&abort_msg);
                    }
                    speculative_error!(log_context, err_msg.clone());
                    return Err(VMStatus::error(
                        StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                        Some(err_msg),
                    ));
                },
            };
            VMStatus::error(new_major_status, None)
        },
        VMStatus::MoveAbort {
            location,
            code,
            message,
        } if !APTOS_TRANSACTION_VALIDATION.is_account_module_abort(&location) => {
            let new_major_status = match split_canonical(code) {
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
                    let mut err_msg = format!(
                        "[aptos_vm] Unexpected prologue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                        location, code, category, reason
                    );
                    if let Some(abort_msg) = message {
                        err_msg.push_str(" Message: ");
                        err_msg.push_str(&abort_msg);
                    }
                    speculative_error!(log_context, err_msg.clone());
                    return Err(VMStatus::error(
                        StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                        Some(err_msg),
                    ));
                },
            };
            VMStatus::error(new_major_status, None)
        },
        VMStatus::MoveAbort {
            location,
            code,
            message,
        } => {
            let new_major_status = match split_canonical(code) {
                // Invalid authentication key
                (INVALID_ARGUMENT, EBAD_ACCOUNT_AUTHENTICATION_KEY) => StatusCode::INVALID_AUTH_KEY,
                // Sequence number too old
                (INVALID_ARGUMENT, ESEQUENCE_NUMBER_TOO_OLD) => StatusCode::SEQUENCE_NUMBER_TOO_OLD,
                // Sequence number too new
                (INVALID_ARGUMENT, ESEQUENCE_NUMBER_TOO_NEW) => StatusCode::SEQUENCE_NUMBER_TOO_NEW,
                // Sequence number too new
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
                (OUT_OF_RANGE, ESEQUENCE_NUMBER_TOO_BIG) => StatusCode::SEQUENCE_NUMBER_TOO_BIG,
                (INVALID_ARGUMENT, ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH) => {
                    StatusCode::SECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH
                },
                (INVALID_ARGUMENT, EGAS_PAYER_ACCOUNT_MISSING) => {
                    StatusCode::GAS_PAYER_ACCOUNT_MISSING
                },
                (INVALID_STATE, EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT) => {
                    StatusCode::INSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT
                },
                (INVALID_ARGUMENT, ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE) => {
                    StatusCode::TRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE
                },
                (INVALID_ARGUMENT, ENONCE_ALREADY_USED) => StatusCode::NONCE_ALREADY_USED,
                (category, reason) => {
                    let mut err_msg = format!(
                        "[aptos_vm] Unexpected prologue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                        location, code, category, reason
                    );
                    if let Some(abort_msg) = message {
                        err_msg.push_str(" Message: ");
                        err_msg.push_str(&abort_msg);
                    }
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
            let err_msg = format!("[aptos_vm] Unexpected prologue error: {:?}", status);
            speculative_error!(log_context, err_msg.clone());
            VMStatus::Error {
                status_code: StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                sub_status: status.sub_status(),
                message: Some(err_msg),
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
        VMStatus::MoveAbort {
            location,
            code,
            message,
        } if !APTOS_TRANSACTION_VALIDATION.is_account_module_abort(&location) => {
            let (category, reason) = split_canonical(code);
            let mut err_msg = format!(
                "[aptos_vm] Unexpected success epilogue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                location, code, category, reason
            );
            if let Some(abort_msg) = message {
                err_msg.push_str(" Message: ");
                err_msg.push_str(&abort_msg);
            }
            speculative_error!(log_context, err_msg.clone());
            VMStatus::error(
                StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                Some(err_msg),
            )
        },

        VMStatus::MoveAbort {
            location,
            code,
            message,
        } => match split_canonical(code) {
            (OUT_OF_RANGE, ECANT_PAY_GAS_DEPOSIT) => VMStatus::MoveAbort {
                location,
                code,
                message,
            },
            (category, reason) => {
                let mut err_msg = format!(
                    "[aptos_vm] Unexpected success epilogue Move abort: {:?}::{:?} (Category: {:?} Reason: {:?})",
                    location, code, category, reason
                );
                if let Some(abort_msg) = message {
                    err_msg.push_str(" Message: ");
                    err_msg.push_str(&abort_msg);
                }
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
