// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Table of loaded modules, addressed by a dense index.
//!
//! A module ID maps to a [`ModuleIdx`], and the index addresses a row in the
//! table. The index is minted the first time the ID is seen, which can be
//! before the module is loaded: a mandatory set names its members, and a
//! member may still be a cache miss.
//!
//! Rows are appended and never reordered or removed, so an index stays valid
//! until maintenance replaces the whole table. Other loaded modules hold
//! direct pointers into a cached module's lowered code, which is why nothing
//! short of a full reset may drop a row.

use crate::context::loaded_module::{LoadedModule, ModuleEntry};
use anyhow::{anyhow, Result};
use dashmap::{mapref::entry::Entry, DashMap};
use mono_move_alloc::LeakedBoxPtr;
use mono_move_core::interner::{InternedModuleId, ModuleIdx};

/// Concurrent long-living loaded module cache.
//
// TODO(security, metering): unbounded. Memory is reclaimed only by
// maintenance, so maintenance needs a size trigger.
pub(super) struct ModuleCache {
    /// Module ID to index.
    //
    // Uses fxhash because the keys are already well-distributed arena
    // pointers, so a simple, fast hash is sufficient.
    by_id: DashMap<InternedModuleId, ModuleIdx, fxhash::FxBuildHasher>,
    /// All module entries in index order. `boxcar::Vec` is an append-only
    /// concurrent vector: entries are pushed through a shared `&` reference
    /// and are never moved, so a [`ModuleIdx`] stays valid and concurrent
    /// reads need no lock.
    table: boxcar::Vec<ModuleEntry>,
}

impl ModuleCache {
    /// Creates an empty cache.
    pub(super) fn new() -> Self {
        Self {
            by_id: DashMap::with_hasher(fxhash::FxBuildHasher::default()),
            table: boxcar::Vec::new(),
        }
    }

    /// Returns the index of the module with the specified ID, creating an
    /// empty entry for it if absent. Takes a shard write lock on the create
    /// path.
    pub(super) fn get_or_create_idx(&self, id: InternedModuleId) -> Result<ModuleIdx> {
        if let Some(idx) = self.by_id.get(&id) {
            return Ok(*idx);
        }
        match self.by_id.entry(id) {
            Entry::Occupied(occupied) => Ok(*occupied.get()),
            Entry::Vacant(vacant) => {
                // Pushing under the shard lock keeps the index and its row
                // assigned together, exactly once per module ID.
                let row = self.table.push(ModuleEntry::new(id));
                let idx = ModuleIdx::from_table_row(row)
                    .ok_or_else(|| anyhow!("number of cached modules exceeds u32::MAX"))?;
                vacant.insert(idx);
                Ok(idx)
            },
        }
    }

    /// Returns the index of the module with the specified ID, if one has been
    /// minted.
    pub(super) fn idx(&self, id: InternedModuleId) -> Option<ModuleIdx> {
        self.by_id.get(&id).map(|idx| *idx)
    }

    /// Returns the index `raw` names, or [`None`] if the table has no such row.
    pub(super) fn checked_idx(&self, raw: u32) -> Option<ModuleIdx> {
        let idx = ModuleIdx::from_table_row(raw as usize)?;
        self.entry(idx).map(|_| idx)
    }

    /// Returns the entry at `idx`, or [`None`] if the index is not in the
    /// table.
    pub(super) fn entry(&self, idx: ModuleIdx) -> Option<&ModuleEntry> {
        self.table.get(idx.as_usize())
    }

    /// Returns the ID the index was minted for, or [`None`] if the index is
    /// not in the table.
    pub(super) fn id_at(&self, idx: ModuleIdx) -> Option<InternedModuleId> {
        Some(self.entry(idx)?.id())
    }

    /// Fills the entry for the module's ID, creating it if needed. On race,
    /// frees the caller's box and returns the winner, so every caller ends up
    /// with the same canonical pointer.
    pub(super) fn insert(&self, module: Box<LoadedModule>) -> Result<LeakedBoxPtr<LoadedModule>> {
        let id = module.id();
        if let Some(existing) = self.get(id) {
            return Ok(existing);
        }

        let idx = self.get_or_create_idx(id)?;
        let entry = self
            .entry(idx)
            .ok_or_else(|| anyhow!("cache invariant violated: no entry for a minted index"))?;

        let leaked = LeakedBoxPtr::from_box(module);
        match entry.init(leaked) {
            Ok(()) => Ok(leaked),
            Err(loser) => {
                // SAFETY: `loser` is exclusive to this call and has no aliases.
                unsafe { loser.free_unchecked() };
                entry.get_ptr().ok_or_else(|| {
                    anyhow!("cache invariant violated: entry empty after CAS failure")
                })
            },
        }
    }

    /// Returns the loaded module with the specified ID, if it is cached.
    pub(super) fn get(&self, id: InternedModuleId) -> Option<LeakedBoxPtr<LoadedModule>> {
        self.entry(self.idx(id)?)?.get_ptr()
    }

    /// Frees every cached module and reinstalls an empty table.
    ///
    /// Indices restart from zero afterwards. That is sound only because this
    /// is all-or-nothing: no entry and no recorded index survives it, so no
    /// stale index can alias a new one.
    pub(super) fn reset(&mut self) {
        // Exhaustive destructuring so that adding a new field forces a
        // compile-time error here.
        let Self { by_id, table } = self;
        by_id.clear();
        for (_, entry) in table.iter() {
            if let Some(content) = entry.clear() {
                // SAFETY: `&mut self` means no execution guard is alive, so
                // there are no references to the loaded modules.
                unsafe { content.free_unchecked() };
            }
        }
        // `boxcar::Vec` has no `clear`, so the table is replaced wholesale.
        *table = boxcar::Vec::new();
    }
}
