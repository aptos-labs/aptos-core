// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_core::ExecutableId;
use mono_move_global_context::{ArenaRef, Executable};
use std::collections::{hash_map::Entry, HashMap};

/// Tracks how this read depends on a particular executable.
#[derive(Copy, Clone)]
pub enum ExecutableRead<'guard> {
    /// This module has been charged gas during execution.
    Charged(u64),
    /// This module has been loaded and visited during execution.
    Visited(&'guard Executable),
}

impl ExecutableRead<'_> {
    /// Returns true if this read can be upgraded to the other read. For
    /// example, if read recorded size, it can be upgraded to the actual
    /// executable code. However, existing executable read cannot be replaced.
    fn upgradable_to(&self, other: &ExecutableRead<'_>) -> bool {
        use ExecutableRead::*;

        match (self, other) {
            (Charged(_), Visited(_)) => true,
            (Visited(_), Charged(_)) | (Charged(_), Charged(_)) | (Visited(_), Visited(_)) => false,
        }
    }
}

/// Tracks every executable this transaction depends on and why. Serves as:
///   - Block-STM read-set for validation. On code upgrade, this read-set
///     checked and transaction is re-executed if there is a conflict.
///   - Executable local cache to avoid lookups in concurrent global cache.
///     Ensures there is a consistent view for every executable.
///   - Set of executables used for gas metering and already accounted for.
#[derive(Default)]
pub struct ExecutableReadSet<'guard> {
    inner: HashMap<ArenaRef<'guard, ExecutableId>, ExecutableRead<'guard>>,
}

impl<'guard> ExecutableReadSet<'guard> {
    /// Returns executable read if it has been cached before. Returns [`None`]
    /// otherwise.
    pub fn get(&self, id: ArenaRef<'guard, ExecutableId>) -> Option<ExecutableRead<'guard>> {
        self.inner.get(&id).copied()
    }

    /// Inserts read into the read-set. If previous read already exists in the
    /// read-set, it must be upgradable to the inserted one.
    pub fn insert(&mut self, id: ArenaRef<'guard, ExecutableId>, read: ExecutableRead<'guard>) {
        match self.inner.entry(id) {
            Entry::Vacant(e) => {
                e.insert(read);
            },
            Entry::Occupied(mut e) => {
                debug_assert!(e.get().upgradable_to(&read));
                e.insert(read);
            },
        }
    }
}
