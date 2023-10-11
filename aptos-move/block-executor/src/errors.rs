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
    /// We defensively check certain resource group related invariant violations when committing outputs
    /// in parallel execution, in particular, (1) creating a resource that already exists, (2) deleting
    /// a resource that does not exist, and (3) deleting a group that does not exist. When such an error
    /// is observed, block execution falls back to the sequential execution. Sequential execution never
    /// returns this error to the caller (asserted in some cases).
    /// Note that:
    /// - similar errors observed during transaction execution (not during commit) lead to the
    ///   transaction getting aborted with INVARIANT_VIOLATION_ERROR.
    /// - serialization error for the group update leads to skipping the transaction (in both parallel
    ///   and sequential modes of block execution).
    ResourceGroupError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error<E> {
    FallbackToSequential(PanicOr<IntentionalFallbackToSequential>),
    /// Execution of a thread yields a non-recoverable error, such error will be propagated back to
    /// the caller (leading to the whole block execution getting aborted).
    UserError(E),
}

pub type Result<T, E> = ::std::result::Result<T, Error<E>>;
