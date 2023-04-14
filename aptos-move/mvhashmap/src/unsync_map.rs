// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{types::MVCodeOutput, utils::module_hash};
use aptos_crypto::hash::HashValue;
use aptos_types::{
    executable::{Executable, ExecutableDescriptor, ModulePath},
    write_set::TransactionWrite,
};
use std::{
    cell::RefCell,
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap, HashMap,
    },
    hash::Hash,
};

struct Entry<V: TransactionWrite, X: Executable> {
    // data is Option since we may just use the entry to cache base (storage-version) executable.
    data: Option<V>,
    hash: Option<HashValue>,
    maybe_base_executable: Option<X>,
}

impl<V: TransactionWrite + Clone, X: Executable> Entry<V, X> {
    fn from_data(data: V) -> Self {
        Self {
            data: Some(data),
            hash: None,
            maybe_base_executable: None,
        }
    }

    fn from_base_executable(executable: X) -> Self {
        Self {
            data: None,
            hash: None,
            maybe_base_executable: Some(executable),
        }
    }
}

/// UnsyncMap is designed to mimic the functionality of MVHashMap for sequential execution.
/// In this case only the latest recorded version is relevant, simplifying the implementation.
/// The functionality also includes Executable caching based on ExecutableDescriptor (i.e.
/// module hash / storage version). Note that in the sequential mode, this cache is not re-used
/// across blocks (to keep it simple, as such performance optimization isn't as relevant).
/// UnsyncMap utilizes RefCell to provide access with interior mutability.
pub struct UnsyncMap<K: ModulePath, V: TransactionWrite, X: Executable> {
    data_map: RefCell<BTreeMap<K, Entry<V, X>>>,
    executable_cache: RefCell<HashMap<HashValue, X>>,
    executable_bytes: RefCell<usize>,
}

impl<K: ModulePath + Hash + Clone + Eq + Ord, V: TransactionWrite + Clone, X: Executable> Default
    for UnsyncMap<K, V, X>
{
    fn default() -> Self {
        Self {
            data_map: RefCell::new(BTreeMap::new()),
            executable_cache: RefCell::new(HashMap::new()),
            executable_bytes: RefCell::new(0),
        }
    }
}

impl<K: ModulePath + Hash + Clone + Eq + Ord, V: TransactionWrite + Clone, X: Executable>
    UnsyncMap<K, V, X>
{
    pub fn fetch_data(&self, key: &K) -> Option<V> {
        self.data_map
            .borrow()
            .get(key)
            .and_then(|entry| entry.data.clone())
    }

    pub fn fetch_code(&self, key: &K) -> Option<MVCodeOutput<V, X>> {
        use MVCodeOutput::*;
        debug_assert!(key.module_path().is_some());

        self.data_map
            .borrow_mut()
            .get_mut(key)
            .and_then(|entry| match &entry.data {
                Some(module) => {
                    let hash = entry.hash.get_or_insert(module_hash(module));

                    Some(self.executable_cache.borrow().get(hash).map_or_else(
                        || Module((module.clone(), *hash)),
                        |x| Executable((x.clone(), ExecutableDescriptor::Published(*hash))),
                    ))
                },
                None => entry
                    .maybe_base_executable
                    .as_ref()
                    .map(|x| Executable((x.clone(), ExecutableDescriptor::Storage))),
            })
    }

    pub fn insert(&self, key: K, value: V) {
        self.data_map
            .borrow_mut()
            .insert(key, Entry::from_data(value));
    }

    /// We return false if the executable was already stored, as this isn't supposed to happen
    /// during sequential execution (and the caller may choose to e.g. log a message).
    pub fn store_executable(
        &self,
        key: &K,
        descriptor: ExecutableDescriptor,
        executable: X,
    ) -> bool {
        let size = executable.size_bytes();
        let ret = match descriptor {
            ExecutableDescriptor::Published(hash) => self
                .executable_cache
                .borrow_mut()
                .insert(hash, executable)
                .is_none(),
            ExecutableDescriptor::Storage => match self.data_map.borrow_mut().entry(key.clone()) {
                Occupied(mut entry) => entry
                    .get_mut()
                    .maybe_base_executable
                    .replace(executable)
                    .is_none(),
                Vacant(entry) => {
                    entry.insert(Entry::from_base_executable(executable));
                    true
                },
            },
        };

        if ret {
            *self.executable_bytes.borrow_mut() += size;
        }

        ret
    }

    pub fn executable_size(&self) -> usize {
        *self.executable_bytes.borrow()
    }
}
