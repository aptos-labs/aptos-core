// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loader subsystem error types.

pub use mono_move_core::{LoaderError, LoaderInvariantViolation, LoaderResult};

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
        return ::core::result::Result::Err(
            $crate::error::LoaderError::InvariantViolation(
                $crate::error::LoaderInvariantViolation::$($body)+,
            ),
        )
    };
}
