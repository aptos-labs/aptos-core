// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{ExecutionErrorKind, IntoExecutionError};
use thiserror::Error;

/// Terminal outcome of a native function invocation.
#[derive(Debug, Clone)]
pub enum NativeStatus {
    Success,
    Abort { code: u64, message: Option<String> },
}

/// Error originating from VM-internal mechanisms invoked by a native.
///
/// Intended ONLY for errors that should just be propagated back to the VM runtime
/// rather than being inspected by the native functions themselves.
#[derive(Debug, Clone, Error)]
pub enum VMInternalError {
    #[error("native function invariant violation: {0}")]
    InvariantViolation(String),
    #[error("out of heap memory (requested {requested} bytes)")]
    OutOfHeapMemory { requested: usize },
    #[error("allocation size {requested} exceeds the maximum")]
    AllocationTooLarge { requested: usize },
    #[error("vector allocation size overflow")]
    VecAllocSizeOverflow,
    // TODO: Gas Metering
}

impl IntoExecutionError for VMInternalError {
    fn kind(&self) -> ExecutionErrorKind {
        match self {
            VMInternalError::InvariantViolation(_) => ExecutionErrorKind::InvariantViolation,
            VMInternalError::OutOfHeapMemory { .. }
            | VMInternalError::AllocationTooLarge { .. }
            | VMInternalError::VecAllocSizeOverflow => ExecutionErrorKind::RuntimeLimitExceeded,
        }
    }
}
