// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ShiftedTxnIndex, TxnIndex};
use aptos_types::{executable::ModulePath, vm::modules::ModuleStorageEntry};
use claims::assert_some;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use move_binary_format::errors::VMResult;
use std::{collections::BTreeMap, fmt::Debug, hash::Hash, sync::Arc};

/// Result of a read query on the versioned module storage.
#[derive(Debug)]
pub enum ModuleStorageReadResult {
    /// An existing module at certain index (either base storage or corresponding to
    /// some committed transaction).
    Versioned(ShiftedTxnIndex, Arc<ModuleStorageEntry>),
    /// If module is not found in storage.
    DoesNotExist,
}

impl ModuleStorageReadResult {
    /// If the entry exists, returns it together with its index. Otherwise, returns [None].
    pub fn into_module_module_storage_entry_at_idx(
        self,
    ) -> Option<(ShiftedTxnIndex, Arc<ModuleStorageEntry>)> {
        match self {
            Self::Versioned(idx, entry) => Some((idx, entry)),
            Self::DoesNotExist => None,
        }
    }
}

/// Represents different versions of module storage information for different
/// transaction indices (including the base storage version).
struct VersionedEntry {
    versions: BTreeMap<ShiftedTxnIndex, CachePadded<Option<Arc<ModuleStorageEntry>>>>,
}

impl VersionedEntry {
    /// A new versioned entry with no written versions yet.
    fn empty() -> Self {
        Self {
            versions: BTreeMap::new(),
        }
    }

    /// Returns the "latest" module entry under the specified index. If such an
    /// entry does nto exist, [None] is returned.
    fn get(&self, txn_idx: ShiftedTxnIndex) -> Option<ModuleStorageReadResult> {
        use ModuleStorageReadResult::*;

        self.versions
            .range(ShiftedTxnIndex::zero_idx()..txn_idx)
            .next_back()
            .map(|(idx, entry)| match entry.as_ref() {
                Some(entry) => Versioned(*idx, entry.clone()),
                None => DoesNotExist,
            })
    }
}

/// Module storage, versioned so that we can keep track of module writes of each transaction. In
/// particular, for each key we keep track the writes of all transactions (see [VersionedEntry]).
pub struct VersionedModuleStorage<K> {
    entries: DashMap<K, VersionedEntry>,
}

impl<K: Debug + Hash + Clone + Eq + ModulePath> VersionedModuleStorage<K> {
    /// Returns a new empty versioned module storage.
    pub(crate) fn empty() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Returns the module entry from the module storage. If the entry does
    /// not exist, [ModuleStorageReadResult::DoesNotExist] is returned. If
    /// there is a pending code publish below the queried index, again the
    /// same [ModuleStorageReadResult::DoesNotExist] is returned as all
    /// pending publishes are treated as non-existent modules.
    pub fn get(&self, key: &K, txn_idx: ShiftedTxnIndex) -> ModuleStorageReadResult {
        let v = self
            .entries
            .entry(key.clone())
            .or_insert_with(VersionedEntry::empty);
        v.get(txn_idx)
            .unwrap_or(ModuleStorageReadResult::DoesNotExist)
    }

    /// Similar to [VersionedModuleStorage::get]. The difference is that if the module does not
    /// exist in module storage, the passed closure is used to initialize it. In contrast,
    /// [VersionedModuleStorage::get] returns [ModuleStorageReadResult::DoesNotExist].
    pub fn get_or_else<F>(
        &self,
        key: &K,
        txn_idx: ShiftedTxnIndex,
        init_func: F,
    ) -> VMResult<ModuleStorageReadResult>
    where
        F: FnOnce() -> VMResult<Option<Arc<ModuleStorageEntry>>>,
    {
        use ModuleStorageReadResult::*;

        let mut v = self
            .entries
            .entry(key.clone())
            .or_insert_with(VersionedEntry::empty);

        // Module entry exists in versioned entry, return it.
        if let Some(result) = v.get(txn_idx) {
            return Ok(result);
        }

        // Otherwise, use the passed closure to compute thw base storage value.
        let zero = ShiftedTxnIndex::zero_idx();
        let maybe_entry = init_func()?;
        let result = Ok(match &maybe_entry {
            Some(entry) => Versioned(zero, entry.clone()),
            None => DoesNotExist,
        });
        v.versions.insert(zero, CachePadded::new(maybe_entry));
        result
    }

    /// Removes an existing entry at a given index.
    pub fn remove(&self, key: &K, txn_idx: TxnIndex) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry should always exist before removal");
        let removed = versioned_entry
            .versions
            .remove(&ShiftedTxnIndex::new(txn_idx));
        assert_some!(removed, "Entry should always exist before removal");
    }

    /// Marks an entry in module storage as "pending", i.e., yet to be published.
    /// The implementation simply treats pending writes as non-existent modules,
    /// so that transactions with higher indices observe non-existent modules and
    /// deterministically fail with a non-speculative error.
    pub fn write_pending(&self, key: K, txn_idx: TxnIndex) {
        let mut v = self
            .entries
            .entry(key)
            .or_insert_with(VersionedEntry::empty);
        v.versions
            .insert(ShiftedTxnIndex::new(txn_idx), CachePadded::new(None));
    }

    /// Writes a published module to the storage, which is also visible for
    /// the transactions with higher indices.
    pub(crate) fn write_published(
        &self,
        key: &K,
        idx_to_publish: TxnIndex,
        entry: ModuleStorageEntry,
    ) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry must always exist before publishing");

        let prev = versioned_entry.versions.insert(
            ShiftedTxnIndex::new(idx_to_publish),
            CachePadded::new(Some(Arc::new(entry))),
        );
        assert_some!(prev);
    }

    /// Write the new module storage entry to the specified key-index pair unless
    /// the existing entry has been already verified. Note that the index at which
    /// the modules are verified must always be the index of a committed transaction.
    pub fn write_if_not_verified(
        &self,
        key: &K,
        committed_idx: ShiftedTxnIndex,
        entry: ModuleStorageEntry,
    ) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry must always exist before it is set as verified");

        let prev_entry = versioned_entry
            .versions
            .get(&committed_idx)
            .expect("At least the base storage version must exist")
            .as_ref()
            .expect("Entry must exist before it is marked as verified");
        if !prev_entry.is_verified() {
            versioned_entry
                .versions
                .insert(committed_idx, CachePadded::new(Some(Arc::new(entry))));
        }
    }
}

#[cfg(test)]
mod test {
    // TODO(loader_v2): Implement new set of tests.
}
