// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Flag, ShiftedTxnIndex, TxnIndex};
use claims::assert_some;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{collections::BTreeMap, hash::Hash, ops::Deref, sync::Arc};

pub enum ModuleReadError {
    /// There is no entry for this module yet, and it has to be initialized.
    Uninitialized,
    /// A dependency on some other transaction has been found, or there is a
    /// pending code publish not yet committed.
    Dependency(TxnIndex),
}

enum Entry<M> {
    Pending { flag: Flag, module: Arc<M> },
    Published { module: Arc<M> },
}

impl<M> Entry<M> {
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
        if let Entry::Pending { flag, module } = self {
            assert!(*flag == Flag::Done, "Estimates cannot be published");
            let module = Arc::clone(module);
            *self = Entry::Published { module };
        };
    }
}

struct VersionedEntry<M> {
    versions: BTreeMap<ShiftedTxnIndex, CachePadded<Entry<M>>>,
}

impl<M> VersionedEntry<M> {
    fn empty() -> Self {
        Self {
            versions: BTreeMap::new(),
        }
    }

    fn mark_estimate(&mut self, txn_idx: TxnIndex) {
        self.versions
            .get_mut(&ShiftedTxnIndex::new(txn_idx))
            .expect("Entry must exist to be marked as an estimated")
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
                        // If module is pending to be published, we ALWAYS mark it
                        // as an estimate to avoid speculative module loading. As a
                        // result, execution will wait until the module is published,
                        // and the dependency is resolved.
                        let idx = shifted_idx
                            .idx()
                            .expect("Storage version is always published");
                        Err(ModuleReadError::Dependency(idx))
                    },
                    Entry::Published { module } => Ok(Arc::clone(module)),
                }
            },
            None => Err(ModuleReadError::Uninitialized),
        }
    }
}

pub struct VersionedModuleStorage<K, M> {
    entries: DashMap<K, VersionedEntry<M>>,
}

impl<K: Hash + Clone + Eq, M> VersionedModuleStorage<K, M> {
    pub(crate) fn empty() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

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
            .expect("Versioned entry to be published is always present");
        versioned_entry
            .versions
            .get_mut(&ShiftedTxnIndex::new(idx_to_publish))
            .expect("There is always an entry at publish index")
            .publish_if_pending();
    }

    pub fn pending(&self, key: K, txn_idx: TxnIndex, module: Arc<M>) {
        let mut v = self
            .entries
            .entry(key)
            .or_insert_with(|| VersionedEntry::empty());
        v.versions.insert(
            ShiftedTxnIndex::new(txn_idx),
            CachePadded::new(Entry::Pending {
                flag: Flag::Done,
                module,
            }),
        );
    }

    pub fn set_base_value(&self, key: K, module: Arc<M>) {
        let mut v = self
            .entries
            .entry(key)
            .or_insert_with(|| VersionedEntry::empty());
        v.versions.insert(
            ShiftedTxnIndex::zero_idx(),
            CachePadded::new(Entry::Published { module }),
        );
    }
}
