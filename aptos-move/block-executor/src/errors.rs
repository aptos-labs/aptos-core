// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::types::PanicOr;
use aptos_logger::{debug, error};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::delayed_fields::PanicError;
use aptos_vm_logging::{alert, prelude::*};

#[derive(Clone, Debug, PartialEq, Eq)]
/// Logging is bottlenecked in constructors.
pub enum IntentionalFallbackToSequential {
    // The same module access path for module was both read & written during speculative executions.
    // This may trigger a race due to the Move-VM loader cache implementation, and mitigation requires
    // aborting the parallel execution pipeline and falling back to the sequential execution.
    // TODO: provide proper multi-versioning for code (like data) for the cache.
    ModulePathReadWrite,
    // This is not PanicError because we need to match the error variant to provide a specialized
    // fallback logic if a resource group serialization error occurs.
    ResourceGroupSerializationError,
    // If multiple workers encounter conditions that qualify for a sequential fallback during parallel
    // execution, it is not clear what is the "right" one to fallback with. Instead, we use the
    // variant below. TODO: pass a vector of all encountered conditions (mainly for tests).
    FallbackFromParallel,
}

impl IntentionalFallbackToSequential {
    pub(crate) fn module_path_read_write(error_msg: String, txn_idx: TxnIndex) -> Self {
        // Module R/W is an expected fallback behavior, no alert is required.
        debug!("[Execution] At txn {}, {:?}", txn_idx, error_msg);

        IntentionalFallbackToSequential::ModulePathReadWrite
    }

    pub(crate) fn resource_group_serialization_error(error_msg: String, txn_idx: TxnIndex) -> Self {
        alert!("[Execution] At txn {}, {:?}", txn_idx, error_msg);

        IntentionalFallbackToSequential::ResourceGroupSerializationError
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockExecutionError<E> {
    FallbackToSequential(PanicOr<IntentionalFallbackToSequential>),
    /// If the unrecoverable VM error occurs during sequential execution (e.g. fallback),
    /// the error is propagated back to the caller (block execution is aborted).
    FatalVMError(E),
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
