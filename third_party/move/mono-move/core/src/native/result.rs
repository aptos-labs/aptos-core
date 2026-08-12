// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{ExecutionErrorKind, IntoExecutionError, VMInternalError};
use thiserror::Error;

/// Terminal outcome of a native function invocation.
#[derive(Debug, Clone)]
pub enum NativeStatus {
    Success,
    Abort { code: u64, message: Option<String> },
}

#[derive(Debug, Error)]
#[error("native function invariant violation: {0}")]
struct NativeInvariantViolation(String);

impl IntoExecutionError for NativeInvariantViolation {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::InvariantViolation
    }
}

/// Wraps an invariant violation raised by a native function itself.
pub fn native_invariant_violation(message: String) -> VMInternalError {
    VMInternalError::new(NativeInvariantViolation(message))
}
