// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{ExecutableId, Function};
use mono_move_alloc::{ExecutableArenaPtr, GlobalArenaPtr};

/// Handles resolving cross-module functions. Returns [`None`] if executable
/// or function do not exist.
pub trait FunctionResolver {
    fn resolve_function(
        &self,
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
    ) -> Option<ExecutableArenaPtr<Function>>;
}

/// Per-transaction context used by the interpreter. Bundles all state that
/// lives for the duration of a single transaction execution:
///   - gas metering counters,
///   - read-set records (to cache reads and for Block-STM tracking),
///   - heap / memory management,
pub trait TransactionContext: FunctionResolver {}

/// A no-op [`TransactionContext`] for testing.
pub struct NoopTransactionContext;

impl FunctionResolver for NoopTransactionContext {
    fn resolve_function(
        &self,
        _executable_id: GlobalArenaPtr<ExecutableId>,
        _name: GlobalArenaPtr<str>,
    ) -> Option<ExecutableArenaPtr<Function>> {
        None
    }
}

impl TransactionContext for NoopTransactionContext {}
