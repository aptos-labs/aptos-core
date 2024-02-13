// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_types::delayed_fields::PanicError;

// The same module access path for module was both read & written during speculative executions.
// This may trigger a race due to the Move-VM loader cache implementation, and mitigation requires
// aborting the parallel execution pipeline and falling back to the sequential execution.
// TODO: provide proper multi-versioning for code (like data) for the cache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModulePathReadWriteError;

// This is not PanicError because we need to match the error variant to provide a specialized
// fallback logic if a resource group serialization error occurs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceGroupSerializationError;

#[derive(Clone, Debug, PartialEq, Eq)]
/// Logging is bottlenecked in constructors.
pub enum SequentialBlockExecutionError<E> {
    // This is not PanicError because we need to match the error variant to provide a specialized
    // fallback logic if a resource group serialization error occurs.
    ResourceGroupSerializationError,
    ErrorToReturn(BlockExecutionError<E>)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockExecutionError<E> {
    FatalBlockExecutorError(PanicError),
    /// If the unrecoverable VM error occurs during sequential execution (e.g. fallback),
    /// the error is propagated back to the caller (block execution is aborted).
    FatalVMError(E),
}

pub type BlockExecutionResult<T, E> = Result<T, BlockExecutionError<E>>;

impl<E> From<PanicError> for BlockExecutionError<E> {
    fn from(err: PanicError) -> Self {
        BlockExecutionError::FatalBlockExecutorError(err)
    }
}

impl<E> From<PanicError> for SequentialBlockExecutionError<E> {
    fn from(err: PanicError) -> Self {
        SequentialBlockExecutionError::ErrorToReturn(BlockExecutionError::FatalBlockExecutorError(err))
    }
}
