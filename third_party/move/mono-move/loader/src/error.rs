// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loader subsystem error types.

use mono_move_core::{ExecutionErrorKind, IntoExecutionError, VerificationError};
use move_binary_format::errors::VMError;
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

    /// TODO(completeness): temporary until natives are loadable as functions.
    #[error("Function {address}::{module}::{name} is a native and cannot be loaded as code")]
    NativeFunctionNotLoadable {
        address: AccountAddress,
        module: String,
        name: String,
    },

    /// TODO(completeness): temporary until nominal types are supported.
    #[error("Failed to lower function: {reason}")]
    LoweringSkipped { reason: &'static str },

    /// The layout of a resource type read outside lowered code could not be
    /// derived.
    #[error("Resource type layout is not derivable")]
    ResourceLayoutNotDerivable,

    #[error("Script does not deserialize: {message}")]
    ScriptDeserializationFailed { message: String },

    /// The script failed bytecode verification or dependency linking.
    /// Preserves the original verifier error.
    #[error("Script failed verification: {:?}", .error.major_status())]
    ScriptVerificationFailed { error: VMError },

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
            ModuleNotFound { .. } | FunctionNotFound { .. } | NativeFunctionNotLoadable { .. } => {
                ExecutionErrorKind::LinkingError
            },

            // TODO(cleanup): delegate once GlobalContext has its own error type.
            GlobalContext(_) | LoweringSkipped { .. } | ResourceLayoutNotDerivable => {
                ExecutionErrorKind::Placeholder
            },

            // TODO(cleanup): needs deserialization and verification categories.
            ScriptDeserializationFailed { .. } | ScriptVerificationFailed { .. } => {
                ExecutionErrorKind::Placeholder
            },

            InvariantViolation(_) => ExecutionErrorKind::InvariantViolation,
        }
    }
}

/// Joins the verifier's findings into a single diagnostic line.
fn format_verification_errors(errors: &[VerificationError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Module-table and cache-consistency assertions raised by the loader.
/// Surfaced rather than panicked so callers can produce a clean
/// per-transaction outcome and alert operationally on
/// [`ExecutionErrorKind::InvariantViolation`].
#[derive(Debug, Error)]
pub enum LoaderInvariantViolation {
    // ---- module table ----
    #[error("Module index has no row in the module table")]
    ModuleIndexNotInTable,

    #[error("Module was charged for but is not loaded")]
    ModuleNotLoaded,

    #[error("Target module is not loaded")]
    TargetModuleNotLoaded,

    // ---- function slot ----
    #[error("Function slot has just been set")]
    FunctionSlotEmptyAfterSet,

    // ---- lowering ----
    /// The specializer produced a function the micro-op verifier rejects.
    /// The bytecode already passed the Move bytecode verifier, so this is a
    /// bug in the lowering pipeline, not in the user's code.
    #[error("Lowered function failed micro-op verification: {}", format_verification_errors(.errors))]
    MicroOpVerificationFailed { errors: Vec<VerificationError> },
}

/// Returns from the enclosing function with a [`LoaderError::InvariantViolation`]
/// wrapping the named [`LoaderInvariantViolation`] variant. Works for both
/// unit and struct variants:
///
/// ```ignore
/// invariant_violation!(ModuleNotLoaded);
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
