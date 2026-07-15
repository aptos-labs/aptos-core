// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Specializer subsystem error types.
//!
//! Following the two-layer error model (see
//! `mono-move/docs/error_design.md`), the specializer owns a fully-typed
//! internal error enum, [`SpecializerError`], with one variant per real
//! error site. Its exhaustive `IntoExecutionError` impl is the single
//! place where each variant is assigned a public `ExecutionErrorKind`
//! category — adding a variant fails to compile until that decision is
//! made.
//!
//! Like the loader and runtime subsystems, the definitions currently live
//! in `mono-move-core` (see `mono_move_core::vm_error`, tracked by the
//! `TODO(cleanup)` there to extract the internal enums into their own
//! crate) and are re-exported here so the specializer presents its own
//! error surface.

pub use mono_move_core::{SpecializerError, SpecializerInvariantViolation, SpecializerResult};

/// Returns from the enclosing function with a
/// [`SpecializerError::InvariantViolation`] wrapping the named
/// [`SpecializerInvariantViolation`] variant. Works for unit and struct
/// variants:
///
/// ```ignore
/// invariant_violation!(StackUnderflow);
/// invariant_violation!(VidOutOfRange { vid });
/// ```
///
/// For `Option`/`Result` sites, prefer
/// `.ok_or(SpecializerInvariantViolation::Foo)?`, which converts through
/// the [`From`] impl.
#[macro_export]
macro_rules! invariant_violation {
    ($($body:tt)+) => {
        return ::core::result::Result::Err(
            $crate::error::SpecializerError::InvariantViolation(
                $crate::error::SpecializerInvariantViolation::$($body)+,
            ),
        )
    };
}
