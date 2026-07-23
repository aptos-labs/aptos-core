// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::error::PanicError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParallelBlockExecutionError {
    /// unrecoverable VM error
    FatalVMError,
    // Incarnation number that is higher than a threshold is observed during parallel execution.
    // This might be indicative of some sort of livelock, or at least some sort of inefficiency
    // that would warrants investigating the root cause. Execution can fallback to sequential.
    IncarnationTooHigh,
}

// This is separate error because we need to match the error variant to provide a specialized
// fallback logic if a resource group serialization error occurs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceGroupSerializationError;

#[derive(Clone, Debug, PartialEq, Eq)]
/// Logging is bottlenecked in constructors.
pub(crate) enum SequentialBlockExecutionError {
    // This is separate error because we need to match the error variant to provide a specialized
    // fallback logic if a resource group serialization error occurs.
    ResourceGroupSerializationError,
    ErrorToReturn(BlockExecutionError),
}

/// If the unrecoverable error occurs during sequential execution (e.g. fallback),
/// the error is propagated back to the caller (block execution is aborted).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockExecutionError {
    /// unrecoverable BlockSTM error
    FatalBlockExecutorError(PanicError),
    /// unrecoverable VM error (stringified; the block fails regardless of the specific error).
    FatalVMError(String),
}

pub type BlockExecutionResult<T> = Result<T, BlockExecutionError>;

impl From<PanicError> for BlockExecutionError {
    fn from(err: PanicError) -> Self {
        BlockExecutionError::FatalBlockExecutorError(err)
    }
}

impl From<PanicError> for SequentialBlockExecutionError {
    fn from(err: PanicError) -> Self {
        SequentialBlockExecutionError::ErrorToReturn(BlockExecutionError::FatalBlockExecutorError(
            err,
        ))
    }
}
