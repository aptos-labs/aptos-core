// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction set of executables recorded by the loader, pinning one
//! version per module for the duration of the transaction.

use anyhow::{bail, Result};
use mono_move_core::ExecutableId;
use mono_move_global_context::{ArenaRef, LoadedModule};
use shared_dsa::UnorderedMap;
use std::collections::hash_map::Entry;

/// Tracks how this read depends on a particular loaded module.
#[derive(Copy, Clone)]
pub enum ExecutableRead<'guard> {
    Loaded(&'guard LoadedModule),
}

/// Maps from executable ID to the version the transaction is using for the
/// duration of this transaction.
#[derive(Default)]
pub struct ExecutableReadSet<'guard> {
    inner: UnorderedMap<ArenaRef<'guard, ExecutableId>, ExecutableRead<'guard>>,
}

impl<'guard> ExecutableReadSet<'guard> {
    /// Creates an empty read-set.
    pub fn new() -> Self {
        Self {
            inner: UnorderedMap::new(),
        }
    }

    /// Returns the recorded loaded module or [`None`] otherwise.
    pub fn get(&self, key: ArenaRef<'guard, ExecutableId>) -> Option<&'guard LoadedModule> {
        match self.inner.get(&key)? {
            ExecutableRead::Loaded(loaded) => Some(*loaded),
        }
    }

    /// Records executable version this transaction will use. Returns an error
    /// if the executable was already recorded.
    pub(crate) fn record(
        &mut self,
        key: ArenaRef<'guard, ExecutableId>,
        read: ExecutableRead<'guard>,
    ) -> Result<()> {
        match self.inner.entry(key) {
            Entry::Vacant(e) => {
                e.insert(read);
                Ok(())
            },
            Entry::Occupied(_) => {
                bail!("Read is already recorded")
            },
        }
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
