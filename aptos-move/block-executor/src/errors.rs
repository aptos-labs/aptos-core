// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::types::PanicOr;
use aptos_types::aggregator::PanicError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntentionalFallbackToSequential {
    /// The same module access path for module was both read & written during speculative executions.
    /// This may trigger a race due to the Move-VM loader cache implementation, and mitigation requires
    /// aborting the parallel execution pipeline and falling back to the sequential execution.
    /// TODO: (short-mid term) relax the limitation, and (mid-long term) provide proper multi-versioning
    /// for code (like data) for the cache.
    ModulePathReadWrite,
    /// We defensively check certain resource group related invariant violations.
    ResourceGroupError(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error<E> {
    FallbackToSequential(PanicOr<IntentionalFallbackToSequential>),
    /// Execution of a thread yields a non-recoverable error, such error will be propagated back to
    /// the caller (leading to the block execution getting aborted). TODO: revisit name (UserError).
    UserError(E),
}

pub type Result<T, E> = ::std::result::Result<T, Error<E>>;

impl<E> From<PanicOr<IntentionalFallbackToSequential>> for Error<E> {
    fn from(err: PanicOr<IntentionalFallbackToSequential>) -> Self {
        Error::FallbackToSequential(err)
    }
}

impl<E> From<PanicError> for Error<E> {
    fn from(err: PanicError) -> Self {
        Error::FallbackToSequential(err.into())
    }
}
