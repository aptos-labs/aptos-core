// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interpreter-internal error types.

pub use mono_move_core::{
    ArithOp, GlobalStorageOp, RuntimeError, RuntimeInvariantViolation, RuntimeResult, Signedness,
    VecOp,
};

/// Successful terminal outcomes from `Interpreter::run`. Runtime
/// failures flow through the `Err` channel as [`RuntimeError`] — abort
/// and failure are structurally separate.
#[derive(Debug)]
pub enum RuntimeStatus {
    Success,
    // TODO(completeness): carry the abort's `Location` (which module raised it) once
    // we have a `Location` type defined.
    Aborted { code: u64, message: Option<String> },
}

/// Returns from the enclosing function with an [`RuntimeError::InvariantViolation`]
/// wrapping the named [`RuntimeInvariantViolation`] variant. Works for both
/// unit and struct variants:
///
/// ```ignore
/// invariant_violation!(NullFuncRefInClosure);
/// invariant_violation!(PcOutOfBounds { pc, func_name, code_len });
/// ```
#[macro_export]
macro_rules! invariant_violation {
    ($($body:tt)+) => {
        return ::core::result::Result::Err(
            $crate::error::RuntimeError::InvariantViolation(
                $crate::error::RuntimeInvariantViolation::$($body)+,
            ),
        )
    };
}
