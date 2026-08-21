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
use mono_move_loader::LoaderError;
use mono_move_runtime::RuntimeError;
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

/// Converts a type-erased VM error into `VMStatus`: fine-grained for the
/// error types with known legacy mappings, by public category otherwise.
//
// TODO(correctness): extend the downcast set as more subsystem error types
// (verifier, deserializer, gas) need exact legacy codes.
// TODO(cleanup): refactor comparison replay benchmark mapping
fn internal_error_to_status(err: &VMInternalError) -> VMStatus {
    if let Some(err) = err.downcast_ref::<RuntimeError>() {
        return runtime_error_to_status(err);
    }
    if let Some(err) = err.downcast_ref::<LoaderError>() {
        return loader_error_to_status(err);
    }
    let code = match err.kind() {
        ExecutionErrorKind::OutOfGas => StatusCode::OUT_OF_GAS,
        ExecutionErrorKind::LinkingError => StatusCode::LINKER_ERROR,
        ExecutionErrorKind::InvariantViolation => StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        ExecutionErrorKind::RuntimeLimitExceeded
        | ExecutionErrorKind::InvalidOperation
        | ExecutionErrorKind::Placeholder => StatusCode::UNKNOWN_STATUS,
    };
    VMStatus::error(code, Some(err.to_string()))
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
        // misbehaved, which the legacy VM reports as an unexpected error.
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

/// Mirrors the legacy VM's `convert_prologue_error`: recognized validation
/// aborts map to their discard codes.
//
// TODO(completeness): the legacy VM also recognizes aborts from
// `transaction_limits` and the multisig account module. Neither is reachable
// here yet, so they land in the unexpected-abort branch below.
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

/// The status of a failure the legacy mapping does not recognize: the framework
/// misbehaved, which the legacy VM reports as an unexpected error.
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

// TODO(correctness): the mapping is coarse; several kinds should be
// `ExecutionFailure`, which needs the location, function and code offset the
// legacy VM reports. Currently,`RuntimeError` carries none of those.
fn runtime_error_to_status(err: &RuntimeError) -> VMStatus {
    use RuntimeError as E;
    let code = match err {
        E::ArithmeticOverflow { .. }
        | E::ArithmeticUnderflow { .. }
        | E::DivisionByZero { .. }
        | E::ShiftAmountOutOfRange { .. }
        | E::ArithmeticUnderOverflow { .. }
        | E::DivisionByZeroOrOverflow { .. }
        | E::NegateMinOverflow { .. }
        | E::CastOutOfRange { .. } => StatusCode::ARITHMETIC_ERROR,
        E::PopFromEmptyVector
        | E::VectorIndexOutOfBounds { .. }
        | E::VecUnpackLengthMismatch { .. } => StatusCode::VECTOR_OPERATION_ERROR,
        E::ResourceDoesNotExist { .. } => StatusCode::MISSING_DATA,
        E::ResourceAlreadyExists { .. } => StatusCode::RESOURCE_ALREADY_EXISTS,
        E::EnumVariantMismatch { .. } => StatusCode::TYPE_MISMATCH,
        E::StackOverflow => StatusCode::CALL_STACK_OVERFLOW,
        // Raised only by the runtime's own write-set builder, which this crate
        // does not use (its drain goes through the provider).
        E::StateKeyTypeTooDeep => StatusCode::TOO_MANY_TYPE_NODES,
        E::OutOfHeapMemory { .. } | E::AllocationTooLarge { .. } | E::VecAllocSizeOverflow => {
            StatusCode::MEMORY_LIMIT_EXCEEDED
        },
        E::InvalidAbortMessage
        | E::AbortMessageTooLong { .. }
        | E::BCSEof
        | E::BCSInvalidUleb
        | E::BCSSequenceTooLong { .. }
        | E::BCSRemainingInput { .. }
        | E::BCSInvalidBool { .. }
        | E::BCSSignerNotDeserializable => StatusCode::UNKNOWN_STATUS,
        E::Unsupported(_) | E::InvariantViolation(_) | E::ResourceProvider(_) => {
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
        },
    };
    VMStatus::error(code, Some(format!("{}", err)))
}

fn loader_error_to_status(err: &LoaderError) -> VMStatus {
    use LoaderError as L;
    let code = match err {
        // Kept (and charged) under the function-values feature, since a stale
        // function value makes this reachable at runtime.
        L::FunctionNotFound { .. } => StatusCode::FUNCTION_RESOLUTION_FAILURE,
        L::ModuleNotFound { .. } | L::NativeFunctionNotLoadable { .. } => StatusCode::LINKER_ERROR,
        L::LoweringSkipped { .. } => StatusCode::UNKNOWN_STATUS,
        L::GlobalContext(_) | L::InvariantViolation(_) => {
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
        },
    };
    VMStatus::error(code, Some(format!("{}", err)))
}
