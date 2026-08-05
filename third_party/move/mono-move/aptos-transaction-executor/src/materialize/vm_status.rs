// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Converts the typed outcome taxonomy into `VMStatus`.

use crate::errors::{DiscardReason, ExecutionStatus, PrologueFailure};
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
use move_core_types::vm_status::{StatusCode, VMStatus};

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
        DiscardReason::InvalidTypeArgument(detail) => {
            VMStatus::error(StatusCode::TYPE_RESOLUTION_FAILURE, Some(detail))
        },
        DiscardReason::Prologue(failure) => prologue_failure_to_status(failure),
        DiscardReason::Epilogue { run, failure } => {
            epilogue_failure_status(run, &format!("{failure:?}"))
        },
        DiscardReason::InvariantViolation(detail) => invariant_status(detail),
    }
}

/// Converts an executed transaction's conclusion into its `VMStatus`.
pub(crate) fn executed_vm_status(status: &ExecutionStatus) -> VMStatus {
    match status {
        ExecutionStatus::Success => VMStatus::Executed,
        ExecutionStatus::Abort {
            code,
            message,
            location,
        } => VMStatus::MoveAbort {
            location: location.clone(),
            code: *code,
            message: message.clone(),
        },
        ExecutionStatus::Failure(err) => internal_error_to_status(err),
        ExecutionStatus::RecoveredEpilogueFailure(failure) => {
            epilogue_failure_status("after a successful payload", &format!("{failure:?}"))
        },
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
// TODO(correctness): MonoMove aborts carry no location yet, so every prologue
// abort is treated as coming from the transaction-validation module. The
// multisig- and transaction-limits-module branches need locations to be
// distinguishable.
fn prologue_failure_to_status(failure: PrologueFailure) -> VMStatus {
    let (code, message) = match failure {
        PrologueFailure::Abort { code, message } => (code, message),
        PrologueFailure::Unexpected(detail) => {
            return unexpected_validation_error("prologue", detail)
        },
    };
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
        (category, reason) => {
            let mut err_msg = format!(
                "Unexpected prologue Move abort: {:?} (Category: {:?} Reason: {:?})",
                code, category, reason
            );
            if let Some(abort_msg) = message {
                err_msg.push_str(" Message: ");
                err_msg.push_str(&abort_msg);
            }
            return VMStatus::error(
                StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                Some(err_msg),
            );
        },
    };
    VMStatus::error(new_major_status, None)
}

fn unexpected_validation_error(msg: &str, detail: String) -> VMStatus {
    VMStatus::error(
        StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
        Some(format!(
            "[aptos_txn_executor] Unexpected {msg} error: {detail}"
        )),
    )
}

/// The status of an epilogue failure the fee abort does not cover, whether it
/// ends up kept or discarded. `run` says which epilogue run failed.
fn epilogue_failure_status(run: &str, detail: &str) -> VMStatus {
    unexpected_validation_error(&format!("epilogue {run}"), detail.to_string())
}

// TODO(correctness): the mapping is coarse; several kinds need location info
// (`ExecutionFailure`) and exact status codes to match the legacy VM.
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
        E::InvariantViolation(_) | E::ResourceProvider(_) => {
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
        L::ModuleNotFound { .. } | L::FunctionIrMissing => StatusCode::LINKER_ERROR,
        L::LoweringSkipped { .. } => StatusCode::UNKNOWN_STATUS,
        L::GlobalContext(_) | L::InvariantViolation(_) => {
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
        },
    };
    VMStatus::error(code, Some(format!("{}", err)))
}
