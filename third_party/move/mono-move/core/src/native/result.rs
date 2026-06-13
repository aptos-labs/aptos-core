// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{ExecutionErrorKind, IntoExecutionError, RuntimeError, RuntimeInvariantViolation};
use thiserror::Error;

/// Terminal outcome of a native function invocation.
#[derive(Debug, Clone)]
pub enum NativeStatus {
    Success,
    Abort { code: u64, message: Option<String> },
}

/// A [`RuntimeError`] raised by a VM-internal mechanism a native invoked.
///
/// Intended ONLY for errors the native should propagate straight back to the VM
/// runtime, never inspect itself.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct VMInternalError(RuntimeError);

impl VMInternalError {
    /// Wraps an invariant violation raised by a native function itself.
    pub fn invariant_violation(message: String) -> Self {
        Self(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Native(message),
        ))
    }

    /// Unwraps the underlying runtime error.
    pub fn into_runtime_error(self) -> RuntimeError {
        self.0
    }
}

impl From<RuntimeError> for VMInternalError {
    fn from(err: RuntimeError) -> Self {
        Self(err)
    }
}

impl IntoExecutionError for VMInternalError {
    fn kind(&self) -> ExecutionErrorKind {
        self.0.kind()
    }
}

/// A BCS (de)serialization failure caused by a native's argument — the value
/// being serialized or the input bytes being deserialized.
#[derive(Debug, Error)]
pub enum BcsError {
    #[error("BCS: unexpected end of input")]
    UnexpectedEof,
    #[error("BCS: malformed ULEB128 length")]
    MalformedLength,
    #[error("BCS: sequence length {len} exceeds the maximum")]
    SequenceTooLong { len: u64 },
    #[error("BCS: {remaining} trailing byte(s) after the value")]
    TrailingBytes { remaining: usize },
}
