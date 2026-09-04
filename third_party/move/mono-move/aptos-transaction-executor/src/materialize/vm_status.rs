// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Converts the typed outcome taxonomy into `VMStatus`.

use crate::errors::{
    is_cant_pay_fee_abort, DiscardReason, ExecutionStage, ExecutionStatus, MoveExecutionFailure,
    PreExecutionCheckFailure,
};
use aptos_types::{
    error::{split_canonical, INVALID_ARGUMENT, INVALID_STATE, OUT_OF_RANGE},
    transaction::validation::{
        EACCOUNT_DOES_NOT_EXIST, EBAD_ACCOUNT_AUTHENTICATION_KEY, EBAD_CHAIN_ID,
        ECANT_PAY_GAS_DEPOSIT, EGAS_PAYER_ACCOUNT_MISSING,
        EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT, ENONCE_ALREADY_USED,
        ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH, ESEQUENCE_NUMBER_TOO_BIG,
        ESEQUENCE_NUMBER_TOO_NEW, ESEQUENCE_NUMBER_TOO_OLD,
        ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE, ETRANSACTION_EXPIRED,
    },
};
use mono_move_core::{ExecutionErrorKind, VMInternalError};
use mono_move_output::v1_error::{self, V1Equivalent};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::ModuleId,
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use std::sync::LazyLock;

/// Where the transaction prologue and epilogue live.
static ABORT_LOC_VALIDATION_MODULE: LazyLock<AbortLocation> = LazyLock::new(|| {
    AbortLocation::Module(ModuleId::new(
        AccountAddress::ONE,
        ident_str!("transaction_validation").to_owned(),
    ))
});

/// Converts a type-erased VM error into `VMStatus`.
///
/// Only reached after the prologue, where the transaction always commits and is
/// charged, so every status below must be one that commits. Which code carries
/// that is MonoMove's own choice; it need not be the one V1 reports.
//
// TODO(correctness): the result is always `VMStatus::Error`, never
// `VMStatus::ExecutionFailure`, which additionally carries the location,
// function, and code offset of the faulting instruction. Mono's errors do not
// carry those yet.
fn internal_error_to_status(err: &VMInternalError) -> VMStatus {
    match v1_error::describe(err) {
        // Built directly: `VMStatus::error` hardcodes the sub-status to `None`.
        V1Equivalent::Described(info) => VMStatus::Error {
            status_code: info.status,
            sub_status: info.sub_status.known(),
            // Mono's own text where V1 renders none. Not persisted: it surfaces
            // in simulation responses and logs, and is the only report of
            // allocation sizes and out-of-bounds indices.
            message: Some(
                info.message
                    .text()
                    .map_or_else(|| err.to_string(), str::to_owned),
            ),
        },

        // V1 reports no failure, so there is no status to borrow. Charges
        // anyway: this runs after the prologue, where the fee and sequence
        // number are already committed, and a discarding code would drop them.
        //
        // TODO(correctness): reaching this means MonoMove cannot run an input V1
        // runs. The replay benchmark reports it as a status mismatch, but
        // production charges the sender and logs nothing.
        V1Equivalent::NoV1Failure => {
            VMStatus::error(StatusCode::UNKNOWN_RUNTIME_STATUS, Some(err.to_string()))
        },

        // Every code below commits and charges, including invariant violations,
        // which V1 discards: the executor has already committed the fee, and a
        // discarding code would drop it.
        //
        // TODO(correctness): four kinds collapse to `UNKNOWN_RUNTIME_STATUS`,
        // which is persisted, so telling them apart later is a breaking change.
        V1Equivalent::V1StatusUnknown => {
            let code = match err.kind() {
                ExecutionErrorKind::OutOfGas => StatusCode::OUT_OF_GAS,
                ExecutionErrorKind::LinkingError => StatusCode::LINKER_ERROR,
                ExecutionErrorKind::InvariantViolation
                | ExecutionErrorKind::RuntimeLimitExceeded
                | ExecutionErrorKind::InvalidOperation
                | ExecutionErrorKind::Placeholder => StatusCode::UNKNOWN_RUNTIME_STATUS,
            };
            VMStatus::error(code, Some(err.to_string()))
        },
    }
}

/// Converts a discard reason into the `VMStatus` the transaction is rejected
/// with.
pub(crate) fn discard_to_vm_status(reason: DiscardReason) -> VMStatus {
    match reason {
        DiscardReason::Unsupported(msg) => unsupported_status(msg),
        DiscardReason::PreExecutionCheck(failure) => pre_execution_check_status(failure),
        DiscardReason::InvalidTypeArgument(detail) => {
            VMStatus::error(StatusCode::TYPE_RESOLUTION_FAILURE, Some(detail))
        },
        DiscardReason::Failure {
            stage: ExecutionStage::Prologue,
            failure,
        } => prologue_failure_to_status(failure),
        DiscardReason::Failure { stage, failure } => {
            stage_failure_status(&stage, &format!("{failure:?}"))
        },
        DiscardReason::InvariantViolation(detail) => invariant_status(detail),
    }
}

/// Converts a violated pre-execution bound into the legacy validation status.
fn pre_execution_check_status(failure: PreExecutionCheckFailure) -> VMStatus {
    use PreExecutionCheckFailure as F;
    let code = match &failure {
        F::TransactionTooLarge { .. } => StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE,
        F::GasBudgetAboveBound { .. } => StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
        F::GasBudgetBelowIntrinsicCost { .. } => {
            StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS
        },
        F::GasPriceBelowMinimum { .. } => StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND,
        F::GasPriceAboveMaximum { .. } => StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND,
    };
    VMStatus::error(code, Some(failure.to_string()))
}

/// Converts an executed transaction's conclusion into its `VMStatus`.
pub(crate) fn executed_vm_status(status: &ExecutionStatus) -> VMStatus {
    let (stage, failure) = match status {
        ExecutionStatus::Success => return VMStatus::Executed,
        ExecutionStatus::Failure { stage, failure } => (stage, failure),
    };
    match failure {
        // The fee payer failing to cover the fee is the one epilogue abort that
        // survives as the transaction's own; any other means the framework
        // misbehaved, which V1 reports as an unexpected error.
        MoveExecutionFailure::Abort {
            code,
            message,
            location,
        } if matches!(stage, ExecutionStage::Payload) || is_cant_pay_fee_abort(*code) => {
            VMStatus::MoveAbort {
                location: location.clone(),
                code: *code,
                message: message.clone(),
            }
        },
        MoveExecutionFailure::RuntimeError(err) if matches!(stage, ExecutionStage::Payload) => {
            internal_error_to_status(err)
        },
        failure => stage_failure_status(stage, &format!("{failure:?}")),
    }
}

/// An executor-internal invariant violation.
fn invariant_status(detail: String) -> VMStatus {
    VMStatus::error(
        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        Some(format!("[aptos_txn_executor] {detail}")),
    )
}

fn unsupported_status(msg: &str) -> VMStatus {
    VMStatus::error(
        StatusCode::FEATURE_UNDER_GATING,
        Some(format!(
            "the MonoMove transaction executor does not support {msg}"
        )),
    )
}

/// Mirrors V1's `convert_prologue_error`: recognized validation aborts map to
/// their discard codes.
//
// TODO(completeness): V1 also recognizes aborts from `transaction_limits` and
// the multisig account module. Neither is reachable here yet, so they land in
// the unexpected-abort branch below.
fn prologue_failure_to_status(failure: MoveExecutionFailure) -> VMStatus {
    let (code, message, location) = match failure {
        MoveExecutionFailure::Abort {
            code,
            message,
            location,
        } => (code, message, location),
        MoveExecutionFailure::RuntimeError(err) => {
            return unexpected_validation_error("prologue", err.to_string())
        },
    };
    if location != *ABORT_LOC_VALIDATION_MODULE {
        return unexpected_prologue_abort(code, message, location);
    }
    let new_major_status = match split_canonical(code) {
        (INVALID_ARGUMENT, EBAD_ACCOUNT_AUTHENTICATION_KEY) => StatusCode::INVALID_AUTH_KEY,
        (INVALID_ARGUMENT, ESEQUENCE_NUMBER_TOO_OLD) => StatusCode::SEQUENCE_NUMBER_TOO_OLD,
        (INVALID_ARGUMENT, ESEQUENCE_NUMBER_TOO_NEW) => StatusCode::SEQUENCE_NUMBER_TOO_NEW,
        (INVALID_ARGUMENT, EACCOUNT_DOES_NOT_EXIST) => StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST,
        (INVALID_ARGUMENT, ECANT_PAY_GAS_DEPOSIT) => {
            StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE
        },
        (INVALID_ARGUMENT, ETRANSACTION_EXPIRED) => StatusCode::TRANSACTION_EXPIRED,
        (INVALID_ARGUMENT, EBAD_CHAIN_ID) => StatusCode::BAD_CHAIN_ID,
        (OUT_OF_RANGE, ESEQUENCE_NUMBER_TOO_BIG) => StatusCode::SEQUENCE_NUMBER_TOO_BIG,
        (INVALID_ARGUMENT, ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH) => {
            StatusCode::SECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH
        },
        (INVALID_ARGUMENT, EGAS_PAYER_ACCOUNT_MISSING) => StatusCode::GAS_PAYER_ACCOUNT_MISSING,
        (INVALID_STATE, EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT) => {
            StatusCode::INSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT
        },
        (INVALID_ARGUMENT, ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE) => {
            StatusCode::TRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE
        },
        (INVALID_ARGUMENT, ENONCE_ALREADY_USED) => StatusCode::NONCE_ALREADY_USED,
        _ => return unexpected_prologue_abort(code, message, location),
    };
    VMStatus::error(new_major_status, None)
}

fn unexpected_prologue_abort(
    code: u64,
    message: Option<String>,
    location: AbortLocation,
) -> VMStatus {
    let (category, reason) = split_canonical(code);
    let mut detail = format!("{location:?}::{code} (Category: {category:?} Reason: {reason:?})");
    if let Some(abort_msg) = message {
        detail.push_str(" Message: ");
        detail.push_str(&abort_msg);
    }
    unexpected_validation_error("prologue Move abort:", detail)
}

fn unexpected_validation_error(msg: &str, detail: String) -> VMStatus {
    VMStatus::error(
        StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
        Some(format!(
            "[aptos_txn_executor] Unexpected {msg} error: {detail}"
        )),
    )
}

/// The status of a failure the V1 mapping does not recognize: the framework
/// misbehaved, which V1 reports as an unexpected error.
fn stage_failure_status(stage: &ExecutionStage, detail: &str) -> VMStatus {
    let what = match stage {
        ExecutionStage::Prologue => "prologue",
        ExecutionStage::Payload => "payload",
        ExecutionStage::Epilogue => "epilogue",
        ExecutionStage::EpilogueAfterRollback => "epilogue after a rolled-back payload",
        ExecutionStage::EpilogueRetry => "epilogue retry",
    };
    unexpected_validation_error(what, detail.to_string())
}
