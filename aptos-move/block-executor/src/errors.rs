// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::types::PanicOr;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IntentionalFallbackToSequential {
    /// The same module access path for module was both read & written during speculative executions.
    /// This may trigger a race due to the Move-VM loader cache implementation, and mitigation requires
    /// aborting the parallel execution pipeline and falling back to the sequential execution.
    /// TODO: (short-mid term) relax the limitation, and (mid-long term) provide proper multi-versioning
    /// for code (like data) for the cache.
    ModulePathReadWrite,
    // WriteSetPayload::Direct cannot be handled in mode where delayed_field_optimization is enabled,
    // because delayed fields do value->identifier exchange on reads, and identifier->value exhcange
    // on writes. WriteSetPayload::Direct cannot be processed to do so, as we get outputs directly.
    // We communicate to the executor to retry with capability disabled.
    DirectWriteSetTransaction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error<E> {
    FallbackToSequential(PanicOr<IntentionalFallbackToSequential>),
    /// Execution of a thread yields a non-recoverable error, such error will be propagated back to
    /// the caller.
    UserError(E),
}

pub type Result<T, E> = ::std::result::Result<T, Error<E>>;
