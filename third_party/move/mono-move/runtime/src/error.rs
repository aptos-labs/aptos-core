// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Execution error types for the interpreter runtime.

use mono_move_gas::GasExhaustedError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    /// Gas exhausted.
    #[error(transparent)]
    GasExhausted(#[from] GasExhaustedError),
    /// Placeholder for all other runtime errors (to be refined).
    #[error(transparent)]
    Placeholder(#[from] anyhow::Error),
}

/// Result type for interpreter operations.
pub type ExecutionResult<T> = Result<T, ExecutionError>;

/// Like `anyhow::bail!` but returns `ExecutionError::Placeholder`.
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::error::ExecutionError::Placeholder(anyhow::anyhow!($($arg)*)))
    };
}
