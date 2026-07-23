// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Errors raised while building a transaction output.

use mono_move_core::{ExecutionErrorKind, IntoExecutionError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("event type is not a valid type tag")]
    InvalidEventType,

    #[error("event guid did not decode to an EventKey: {0}")]
    InvalidEventGuid(#[from] bcs::Error),

    #[error("failed to build contract event: {0}")]
    InvalidEvent(String),
}

impl IntoExecutionError for OutputError {
    fn kind(&self) -> ExecutionErrorKind {
        match self {
            OutputError::InvalidEventType => ExecutionErrorKind::InvariantViolation,
            OutputError::InvalidEventGuid(_) => ExecutionErrorKind::InvalidOperation,
            OutputError::InvalidEvent(_) => ExecutionErrorKind::RuntimeLimitExceeded,
        }
    }
}
