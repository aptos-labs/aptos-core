// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::types::PanicOr;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::delayed_fields::PanicError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntentionalFallbackToSequential {
    /// The same module access path for module was both read & written during speculative executions.
    /// This may trigger a race due to the Move-VM loader cache implementation, and mitigation requires
    /// aborting the parallel execution pipeline and falling back to the sequential execution.
    /// TODO: (short-mid term) relax the limitation, and (mid-long term) provide proper multi-versioning
    /// for code (like data) for the cache.
    ModulePathReadWrite,
    /// We defensively check resource group serialization error in the commit phase.
    /// TODO: should trigger invariant violation in the transaction itself.
    ResourceGroupSerializationError(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockExecutionError<E> {
    FallbackToSequential(PanicOr<IntentionalFallbackToSequential>),
    /// Execution of a thread yields a non-recoverable error from the VM. Such an error will be propagated
    /// back to the caller (leading to the block execution getting aborted).
    FatalVMError((E, TxnIndex)),
}

pub type BlockExecutionResult<T, E> = Result<T, BlockExecutionError<E>>;

impl<E> From<PanicOr<IntentionalFallbackToSequential>> for BlockExecutionError<E> {
    fn from(err: PanicOr<IntentionalFallbackToSequential>) -> Self {
        BlockExecutionError::FallbackToSequential(err)
    }
}

impl<E> From<PanicError> for BlockExecutionError<E> {
    fn from(err: PanicError) -> Self {
        BlockExecutionError::FallbackToSequential(err.into())
    }
}
