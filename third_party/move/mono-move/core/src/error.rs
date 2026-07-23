// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Public, top-level VM result types.
//!
//! Errors flow through two layers — a typed internal error per
//! subsystem, then this small public surface that callers consume.
//! This module defines the *public* layer:
//!
//! - [`ExecutionResult`] is the top-level outcome (success / user abort /
//!   VM failure) returned by the caller.
//! - [`ExecutionError`] is the public error wrapped inside
//!   [`ExecutionResult::Failed`]. It pairs an [`ExecutionErrorKind`]
//!   category with a human-readable message.
//! - [`ExecutionErrorKind`] is the small stable category enum that
//!   callers are meant to branch on.
//!
//! Subsystems (interpreter, verifier, loader, etc.) define their own
//! typed internal error enums and provide an exhaustive `From` impl that
//! maps each internal variant to one of these categories. The internal
//! enums and conversions live in their own crates.

use std::{any::Any, fmt};
use thiserror::Error;

/// Stable public category enum. Callers branch on this; they do not
/// inspect internal error types or parse message strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionErrorKind {
    /// Gas budget exhausted. Fixable by raising the budget.
    OutOfGas,
    /// A static or dynamic structural limit was hit (call stack depth,
    /// value depth, etc.). Not fixable by raising gas.
    RuntimeLimitExceeded,
    /// Program attempted an operation that failed at runtime (vector
    /// OOB, arithmetic overflow, missing resource, etc.).
    InvalidOperation,
    /// A referenced module, function, or struct could not be resolved
    /// or had an incompatible signature.
    LinkingError,
    /// A condition that should never occur — a VM bug. Production
    /// deployments should alert on these; users should not see them
    /// surface as transaction failures with diagnostic detail. Can
    /// also be raised speculatively under parallel execution.
    InvariantViolation,
    /// Placeholder for subsystems that have not yet defined a typed
    /// error. Should be removed once every subsystem maps into one of
    /// the categories above.
    Placeholder,
}

impl fmt::Display for ExecutionErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionErrorKind::OutOfGas => write!(f, "OutOfGas"),
            ExecutionErrorKind::RuntimeLimitExceeded => write!(f, "RuntimeLimitExceeded"),
            ExecutionErrorKind::InvalidOperation => write!(f, "InvalidOperation"),
            ExecutionErrorKind::LinkingError => write!(f, "LinkingError"),
            ExecutionErrorKind::InvariantViolation => write!(f, "InvariantViolation"),
            ExecutionErrorKind::Placeholder => write!(f, "Placeholder"),
        }
    }
}

/// Public error type returned for non-success, non-abort outcomes.
///
/// The message is human-readable and deterministic for a given internal
/// variant + payload, but must not be parsed programmatically. Callers
/// that need to branch use [`ExecutionErrorKind`].
///
/// TODO(correctness): before production, audit every message reaching this type
/// (across all `IntoExecutionError` impls). We plan to persist them, so once
/// live a message change is breaking and needs feature gating.
#[derive(Debug, Error)]
#[error("{kind}: {message}")]
pub struct ExecutionError {
    pub kind: ExecutionErrorKind,
    pub message: String,
}

/// Common shape for each subsystem's typed internal error.
///
/// Subsystems (interpreter, verifier, deserializer, loader, ...) own
/// their own error enum and implement this trait by writing one
/// exhaustive `match` that assigns each variant a public category.
/// Adding a variant fails to compile until that decision is made.
///
/// The blanket [`From<E: IntoExecutionError> for ExecutionError`] impl
/// below then folds the per-variant message (via [`fmt::Display`]) and
/// the chosen category into a public [`ExecutionError`] uniformly, so
/// each subsystem only writes the kind mapping — never the wrapping.
pub trait IntoExecutionError: fmt::Display + Any + Send + Sync {
    fn kind(&self) -> ExecutionErrorKind;
}

impl<E: IntoExecutionError> From<E> for ExecutionError {
    fn from(err: E) -> Self {
        ExecutionError {
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

/// Top-level result of executing a transaction or script.
///
/// User aborts and VM failures are structurally separate: `Aborted` is
/// the program asking to stop; `Failed` is the VM unable to continue.
#[derive(Debug)]
pub enum ExecutionResult {
    /// Normal return from the entry function.
    Success,

    /// User abort (`Abort` opcode or abort-returning native function).
    Aborted {
        /// Follows the `errors.move` convention: upper byte categorises,
        /// lower bytes carry a module-specific reason.
        code: u64,
        /// Populated when the abort uses the message form; [`None`] for
        /// code-only aborts.
        message: Option<String>,
    },

    /// VM-detected failure. See [`ExecutionError`] for the category and
    /// message.
    Failed(ExecutionError),
}
