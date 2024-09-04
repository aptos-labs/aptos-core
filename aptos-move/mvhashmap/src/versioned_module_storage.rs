// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ShiftedTxnIndex, StorageVersion, TxnIndex};
use aptos_types::{executable::ModulePath, vm::modules::ModuleStorageEntry};
use claims::assert_some;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use derivative::Derivative;
use move_binary_format::errors::VMResult;
use std::{collections::BTreeMap, fmt::Debug, hash::Hash, sync::Arc};

/// Represents a version of a module - either written by some transaction, or fetched from storage.
pub type ModuleVersion = Result<TxnIndex, StorageVersion>;

/// Result of a read query on the versioned module storage.
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""), PartialEq(bound = ""))]
pub enum ModuleStorageRead {
    /// An existing module at certain index of committed transaction or from the base storage.
    Versioned(
        ModuleVersion,
        #[derivative(PartialEq = "ignore", Debug = "ignore")] Arc<ModuleStorageEntry>,
    ),
    /// If module is not found in storage.
    DoesNotExist,
}

impl ModuleStorageRead {
    pub fn storage_version(entry: Arc<ModuleStorageEntry>) -> Self {
        Self::Versioned(Err(StorageVersion), entry)
    }

    pub fn before_txn_idx(txn_idx: TxnIndex, entry: Arc<ModuleStorageEntry>) -> Self {
        let version = if txn_idx > 0 {
            Ok(txn_idx - 1)
        } else {
            Err(StorageVersion)
        };
        Self::Versioned(version, entry)
    }

    /// If the entry exists, returns it together with its index. Otherwise, returns [None].
    pub fn into_versioned(self) -> Option<(ModuleVersion, Arc<ModuleStorageEntry>)> {
        match self {
            Self::Versioned(version, entry) => Some((version, entry)),
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
    fn get(&self, txn_idx: ShiftedTxnIndex) -> Option<ModuleStorageRead> {
        use ModuleStorageRead::*;

        self.versions
            .range(ShiftedTxnIndex::zero_idx()..txn_idx)
            .next_back()
            .map(|(idx, entry)| match entry.as_ref() {
                Some(entry) => Versioned(idx.idx(), entry.clone()),
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
    /// not exist, [ModuleStorageRead::DoesNotExist] is returned. If
    /// there is a pending code publish below the queried index, again the
    /// same [ModuleStorageRead::DoesNotExist] is returned as all
    /// pending publishes are treated as non-existent modules.
    pub fn get(&self, key: &K, txn_idx: TxnIndex) -> ModuleStorageRead {
        let v = self
            .entries
            .entry(key.clone())
            .or_insert_with(VersionedEntry::empty);
        v.get(ShiftedTxnIndex::new(txn_idx))
            .unwrap_or(ModuleStorageRead::DoesNotExist)
    }

    /// Similar to [VersionedModuleStorage::get]. The difference is that if the module does not
    /// exist in module storage, the passed closure is used to initialize it. In contrast,
    /// [VersionedModuleStorage::get] returns [ModuleStorageRead::DoesNotExist].
    pub fn get_or_else<F>(
        &self,
        key: &K,
        txn_idx: TxnIndex,
        init_func: F,
    ) -> VMResult<ModuleStorageRead>
    where
        F: FnOnce() -> VMResult<Option<Arc<ModuleStorageEntry>>>,
    {
        let mut v = self
            .entries
            .entry(key.clone())
            .or_insert_with(VersionedEntry::empty);

        // Module entry exists in versioned entry, return it.
        if let Some(result) = v.get(ShiftedTxnIndex::new(txn_idx)) {
            return Ok(result);
        }

        // Otherwise, use the passed closure to compute thw base storage value.
        let maybe_entry = init_func()?;
        let result = Ok(match &maybe_entry {
            Some(entry) => ModuleStorageRead::storage_version(entry.clone()),
            None => ModuleStorageRead::DoesNotExist,
        });
        v.versions
            .insert(ShiftedTxnIndex::zero_idx(), CachePadded::new(maybe_entry));
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
    pub fn write_published(&self, key: &K, idx_to_publish: TxnIndex, entry: ModuleStorageEntry) {
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
        version: ModuleVersion,
        entry: ModuleStorageEntry,
    ) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry must always exist before it is set as verified");

        let committed_idx = version
            .map(ShiftedTxnIndex::new)
            .unwrap_or(ShiftedTxnIndex::zero_idx());
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
