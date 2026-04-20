// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction set of executables recorded by the loader, pinning one
//! version per module for the duration of the transaction.

use fxhash::FxHashMap;
use mono_move_core::ExecutableId;
use mono_move_global_context::{ArenaRef, Executable};

/// Tracks how this read depends on a particular executable.
#[derive(Copy, Clone)]
pub struct ExecutableRead<'guard> {
    executable: &'guard Executable,
}

/// Maps from executable ID to the version the transaction is using for the
/// duration of this transaction.
#[derive(Default)]
pub struct ExecutableReadSet<'guard> {
    inner: FxHashMap<ArenaRef<'guard, ExecutableId>, ExecutableRead<'guard>>,
}

impl<'guard> ExecutableReadSet<'guard> {
    /// Creates an empty read-set.
    pub fn new() -> Self {
        Self {
            inner: FxHashMap::default(),
        }
    }

    /// Returns the recorded executable or [`None`] otherwise.
    pub fn get(&self, key: ArenaRef<'guard, ExecutableId>) -> Option<&'guard Executable> {
        Some(self.inner.get(&key)?.executable)
    }

    /// Records executable version this transaction will use. Panics if the
    /// executable was already recorded.
    pub(crate) fn record(
        &mut self,
        key: ArenaRef<'guard, ExecutableId>,
        executable: &'guard Executable,
    ) {
        let prev = self.inner.insert(key, ExecutableRead { executable });
        assert!(prev.is_none());
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the read-set is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
