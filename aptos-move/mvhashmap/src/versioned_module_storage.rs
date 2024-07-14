// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Flag, ShiftedTxnIndex, TxnIndex};
use claims::assert_some;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{collections::BTreeMap, hash::Hash, ops::Deref, sync::Arc};

#[derive(Debug, Eq, PartialEq)]
pub enum ModuleReadError {
    /// There is no entry for this module yet, and it has to be initialized.
    Uninitialized,
    /// A dependency on some other transaction has been found, or there is a
    /// pending code publish which has not yet been committed.
    Dependency(TxnIndex),
}

/// An entry in versioned module storage. We distinguish between modules (and
/// whatever other associated information that is cached) which are:
///   1. Pending to be published, i.e., writes created but the transaction has not
///      yet been committed. Reading a pending module always yields a dependency.
///   2. Published - a module that has been committed, and we are guaranteed there
///      is no speculative information.
enum Entry<M> {
    Pending { flag: Flag, entry: Arc<M> },
    Published { entry: Arc<M> },
}

impl<M: Hash> Entry<M> {
    fn mark_estimate(&mut self) {
        match self {
            Entry::Pending { flag, .. } => {
                *flag = Flag::Estimate;
            },
            Entry::Published { .. } => {
                unreachable!("Published entries can no longer be marked as estimates")
            },
        }
    }

    fn publish_if_pending(&mut self) {
        if let Entry::Pending { flag, entry } = self {
            assert!(
                *flag == Flag::Done,
                "Estimated module storage entries cannot be published"
            );
            let entry = Arc::clone(entry);
            *self = Entry::Published { entry };
        };
    }
}

/// Represents different versions of module storage information for different
/// transaction indices (including the storage version).
struct VersionedEntry<M> {
    versions: BTreeMap<ShiftedTxnIndex, CachePadded<Entry<M>>>,
}

impl<M: Hash> VersionedEntry<M> {
    fn empty() -> Self {
        Self {
            versions: BTreeMap::new(),
        }
    }

    fn mark_estimate(&mut self, txn_idx: TxnIndex) {
        self.versions
            .get_mut(&ShiftedTxnIndex::new(txn_idx))
            .expect("Entry must exist to be marked as an estimate")
            .mark_estimate();
    }

    fn read(&self, txn_idx: TxnIndex) -> Result<Arc<M>, ModuleReadError> {
        match self
            .versions
            .range(ShiftedTxnIndex::zero_idx()..ShiftedTxnIndex::new(txn_idx))
            .next_back()
        {
            Some((shifted_idx, entry)) => {
                match entry.deref() {
                    Entry::Pending { .. } => {
                        // If module is pending to be published, we ALWAYS treat it
                        // as an estimate to avoid speculative module loading. As a
                        // result, execution will wait until the module is published,
                        // and the dependency is resolved.
                        let idx = shifted_idx
                            .idx()
                            .expect("Storage version is always published");
                        Err(ModuleReadError::Dependency(idx))
                    },
                    Entry::Published { entry } => Ok(Arc::clone(entry)),
                }
            },
            None => Err(ModuleReadError::Uninitialized),
        }
    }
}

pub struct VersionedModuleStorage<K, M> {
    entries: DashMap<K, VersionedEntry<M>>,
}

impl<K: Hash + Clone + Eq, M: Hash> VersionedModuleStorage<K, M> {
    pub(crate) fn empty() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn num_keys(&self) -> usize {
        self.entries.len()
    }

    pub fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        self.entries
            .get_mut(key)
            .expect("Versioned entry must always exist to be marked as an estimate")
            .mark_estimate(txn_idx);
    }

    pub fn remove(&self, key: &K, txn_idx: TxnIndex) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry must always exist before removal");
        assert_some!(
            versioned_entry
                .versions
                .remove(&ShiftedTxnIndex::new(txn_idx)),
            "Entry should always exist before removal"
        );
    }

    pub fn read(&self, key: &K, txn_idx: TxnIndex) -> Result<Arc<M>, ModuleReadError> {
        self.entries
            .get(key)
            .map(|v| v.read(txn_idx))
            .unwrap_or(Err(ModuleReadError::Uninitialized))
    }

    pub fn publish(&self, key: &K, idx_to_publish: TxnIndex) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry must always exist before publishing");
        versioned_entry
            .versions
            .get_mut(&ShiftedTxnIndex::new(idx_to_publish))
            .expect("There is always an entry at publish index")
            .publish_if_pending();
    }

    pub fn add_pending(&self, key: K, txn_idx: TxnIndex, entry: Arc<M>) {
        let mut v = self
            .entries
            .entry(key)
            .or_insert_with(|| VersionedEntry::empty());
        v.versions.insert(
            ShiftedTxnIndex::new(txn_idx),
            CachePadded::new(Entry::Pending {
                flag: Flag::Done,
                entry,
            }),
        );
    }

    pub fn set_base_value(&self, key: K, entry: Arc<M>) {
        let mut v = self
            .entries
            .entry(key)
            .or_insert_with(|| VersionedEntry::empty());
        v.versions.insert(
            ShiftedTxnIndex::zero_idx(),
            CachePadded::new(Entry::Published { entry }),
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::test::KeyType;
    use claims::{assert_err_eq, assert_ok_eq};

    #[test]
    fn test_uninitialized_and_storage_entries() {
        let map = VersionedModuleStorage::<KeyType<Vec<u8>>, usize>::empty();

        let key = KeyType(b"/foo/a".to_vec());
        assert_err_eq!(map.read(&key, 4), ModuleReadError::Uninitialized);

        // Now, set the base value. It must be initialized and be published.
        map.set_base_value(key.clone(), Arc::new(0));
        assert_ok_eq!(map.read(&key, 0), Arc::new(0));
        assert_ok_eq!(map.read(&key, 4), Arc::new(0));
    }

    #[test]
    fn test_pending_publish() {
        let map = VersionedModuleStorage::<KeyType<Vec<u8>>, usize>::empty();

        let key = KeyType(b"/foo/a".to_vec());
        assert_err_eq!(map.read(&key, 4), ModuleReadError::Uninitialized);

        // If there is a pending write, it must be visible for transactions of higher versions.
        map.add_pending(key.clone(), 3, Arc::new(3));
        assert_err_eq!(map.read(&key, 2), ModuleReadError::Uninitialized);
        assert_err_eq!(map.read(&key, 4), ModuleReadError::Dependency(3));

        map.set_base_value(key.clone(), Arc::new(0));
        assert_ok_eq!(map.read(&key, 2), Arc::new(0));
        assert_err_eq!(map.read(&key, 4), ModuleReadError::Dependency(3));

        // Resolve code publishing.
        map.publish(&key, 3);
        assert_ok_eq!(map.read(&key, 2), Arc::new(0));
        assert_ok_eq!(map.read(&key, 4), Arc::new(3));
    }

    #[test]
    fn test_mark_estimate() {
        let map = VersionedModuleStorage::<KeyType<Vec<u8>>, usize>::empty();

        let key = KeyType(b"/foo/a".to_vec());
        assert_err_eq!(map.read(&key, 4), ModuleReadError::Uninitialized);

        map.add_pending(key.clone(), 3, Arc::new(3));
        map.mark_estimate(&key, 3);

        assert_err_eq!(map.read(&key, 2), ModuleReadError::Uninitialized);
        assert_err_eq!(map.read(&key, 4), ModuleReadError::Dependency(3));

        // New pending entry would still create a dependency.
        map.add_pending(key.clone(), 3, Arc::new(33));
        assert_err_eq!(map.read(&key, 2), ModuleReadError::Uninitialized);
        assert_err_eq!(map.read(&key, 4), ModuleReadError::Dependency(3));

        map.publish(&key, 3);
        assert_ok_eq!(map.read(&key, 4), Arc::new(33));
    }
}
