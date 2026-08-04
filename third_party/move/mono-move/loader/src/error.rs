// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loader subsystem error types.

use mono_move_core::{ExecutionErrorKind, IntoExecutionError};
use move_core_types::account_address::AccountAddress;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("Module {address}::{name} not found")]
    ModuleNotFound {
        address: AccountAddress,
        name: String,
    },

    #[error("Function {address}::{module}::{name} not found")]
    FunctionNotFound {
        address: AccountAddress,
        module: String,
        name: String,
    },

    /// TODO(completeness): temporary until natives are supported.
    #[error("Function IR missing")]
    FunctionIrMissing,

    /// TODO(completeness): temporary until nominal types are supported.
    #[error("Failed to lower function: {reason}")]
    LoweringSkipped { reason: &'static str },

    /// TODO(cleanup): replace once the global context has its own error type.
    #[error(transparent)]
    GlobalContext(anyhow::Error),

    #[error(transparent)]
    InvariantViolation(#[from] LoaderInvariantViolation),
}

impl IntoExecutionError for LoaderError {
    fn kind(&self) -> ExecutionErrorKind {
        use LoaderError::*;
        match self {
            ModuleNotFound { .. } | FunctionNotFound { .. } | FunctionIrMissing => {
                ExecutionErrorKind::LinkingError
            },

            // TODO(cleanup): delegate once GlobalContext has its own error type.
            GlobalContext(_) | LoweringSkipped { .. } => ExecutionErrorKind::Placeholder,

            InvariantViolation(_) => ExecutionErrorKind::InvariantViolation,
        }
    }
}

/// Read-set state-machine and cache-consistency assertions raised by the
/// loader. Surfaced rather than panicked so callers can produce a clean
/// per-transaction outcome and alert operationally on
/// [`ExecutionErrorKind::InvariantViolation`].
#[derive(Debug, Error)]
pub enum LoaderInvariantViolation {
    // ---- read_set transitions ----
    #[error("There should be no entry when marked as pending")]
    EntryAlreadyExists,

    #[error("Module must be recorded as pending")]
    ModuleExpectedPending,

    #[error("Module is already loaded")]
    ModuleAlreadyLoaded,

    #[error("Module must be loaded")]
    ModuleExpectedLoaded,

    #[error("Module must be at least loaded")]
    ModuleExpectedAtLeastLoaded,

    #[error("Module is already metered")]
    ModuleAlreadyMetered,

    #[error("Module must be metered")]
    ModuleExpectedMetered,

    #[error("Module is already ready for lowering")]
    ModuleAlreadyReady,

    // ---- loader cross-checks against the read-set ----
    #[error("All modules in the read-set must be metered")]
    ReadSetEntryNotMetered,

    #[error("All modules in the read-set must be loaded")]
    ReadSetEntryNotLoaded,

    #[error("Target module is not loaded")]
    TargetModuleNotLoaded,

    #[error("Target module is not metered and ready")]
    TargetModuleNotReady,

    #[error("All modules must be present in the read-set")]
    UnexpectedReadSetMiss,

    // ---- function slot ----
    #[error("Function slot has just been set")]
    FunctionSlotEmptyAfterSet,

    // ---- mandatory dependencies ----
    #[error("Mandatory dependencies must be set")]
    MandatoryDepsNotSet,

    #[error("Mandatory dependencies must always be lazy")]
    MandatoryDepsNotLazy,
}

/// Returns from the enclosing function with a [`LoaderError::InvariantViolation`]
/// wrapping the named [`LoaderInvariantViolation`] variant. Works for both
/// unit and struct variants:
///
/// ```ignore
/// invariant_violation!(PendingEntryAlreadyExists);
/// ```
#[macro_export]
macro_rules! invariant_violation {
    ($($body:tt)+) => {
        return ::core::result::Result::Err(::mono_move_core::VMInternalError::new(
            $crate::error::LoaderError::InvariantViolation(
                $crate::error::LoaderInvariantViolation::$($body)+,
            ),
        ))
    };
}
