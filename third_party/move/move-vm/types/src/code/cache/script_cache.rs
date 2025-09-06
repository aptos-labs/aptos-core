// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::code::Code;
use ambassador::delegatable_trait;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use hashbrown::HashMap;
use std::{cell::RefCell, hash::Hash, ops::Deref};
use triomphe::Arc;

/// Interface used by any script cache implementation.
#[delegatable_trait]
pub trait ScriptCache {
    type Key: Eq + Hash + Clone;
    type Deserialized;
    type Verified;

    /// If the entry associated with the key is vacant, inserts the script and returns its copy.
    /// Otherwise, there is no insertion and the copy of existing entry is returned.
    fn insert_deserialized_script(
        &self,
        key: Self::Key,
        deserialized_script: Self::Deserialized,
    ) -> Arc<Self::Deserialized>;

    /// If the entry associated with the key is vacant, inserts the script and returns its copy.
    /// If the entry associated with the key is occupied, but the entry is not verified, inserts
    /// the script returning the copy. Otherwise, there is no insertion and the copy of existing
    /// (verified) entry is returned.
    fn insert_verified_script(
        &self,
        key: Self::Key,
        verified_script: Self::Verified,
    ) -> Arc<Self::Verified>;

    /// Returns the script if it has been cached before, or [None] otherwise.
    fn get_script(&self, key: &Self::Key) -> Option<Code<Self::Deserialized, Self::Verified>>;

    /// Returns the number of scripts stored in cache.
    fn num_scripts(&self) -> usize;
}

/// Non-[Sync] implementation of script cache suitable for single-threaded execution.
pub struct UnsyncScriptCache<K, D, V> {
    script_cache: RefCell<HashMap<K, Code<D, V>>>,
}

impl<K, D, V> UnsyncScriptCache<K, D, V>
where
    K: Eq + Hash + Clone,
    V: Deref<Target = Arc<D>>,
{
    /// Returns an empty script cache.
    pub fn empty() -> Self {
        Self {
            script_cache: RefCell::new(HashMap::new()),
        }
    }
}

impl<K, D, V> ScriptCache for UnsyncScriptCache<K, D, V>
where
    K: Eq + Hash + Clone,
    V: Deref<Target = Arc<D>>,
{
    type Deserialized = D;
    type Key = K;
    type Verified = V;

    fn insert_deserialized_script(
        &self,
        key: Self::Key,
        deserialized_script: Self::Deserialized,
    ) -> Arc<Self::Deserialized> {
        use hashbrown::hash_map::Entry::*;

        match self.script_cache.borrow_mut().entry(key) {
            Occupied(entry) => entry.get().deserialized().clone(),
            Vacant(entry) => entry
                .insert(Code::from_deserialized(deserialized_script))
                .deserialized()
                .clone(),
        }
    }

    fn insert_verified_script(
        &self,
        key: Self::Key,
        verified_script: Self::Verified,
    ) -> Arc<Self::Verified> {
        use hashbrown::hash_map::Entry::*;

        match self.script_cache.borrow_mut().entry(key) {
            Occupied(mut entry) => {
                if !entry.get().is_verified() {
                    let new_script = Code::from_verified(verified_script);
                    let verified_script = new_script.verified().clone();
                    entry.insert(new_script);
                    verified_script
                } else {
                    entry.get().verified().clone()
                }
            },
            Vacant(entry) => entry
                .insert(Code::from_verified(verified_script))
                .verified()
                .clone(),
        }
    }

    fn get_script(&self, key: &Self::Key) -> Option<Code<Self::Deserialized, Self::Verified>> {
        self.script_cache.borrow().get(key).cloned()
    }

    fn num_scripts(&self) -> usize {
        self.script_cache.borrow().len()
    }
}

/// [Sync] implementation of script cache suitable for multithreaded execution.
pub struct SyncScriptCache<K, D, V> {
    script_cache: DashMap<K, CachePadded<Code<D, V>>>,
}

impl<K, D, V> SyncScriptCache<K, D, V>
where
    K: Eq + Hash + Clone,
    V: Deref<Target = Arc<D>>,
{
    /// Returns an empty script cache.
    pub fn empty() -> Self {
        Self {
            script_cache: DashMap::new(),
        }
    }
}

impl<K, D, V> ScriptCache for SyncScriptCache<K, D, V>
where
    K: Eq + Hash + Clone,
    V: Deref<Target = Arc<D>>,
{
    type Deserialized = D;
    type Key = K;
    type Verified = V;

    fn insert_deserialized_script(
        &self,
        key: Self::Key,
        deserialized_script: Self::Deserialized,
    ) -> Arc<Self::Deserialized> {
        use dashmap::mapref::entry::Entry::*;

        match self.script_cache.entry(key) {
            Occupied(entry) => entry.get().deserialized().clone(),
            Vacant(entry) => entry
                .insert(CachePadded::new(Code::from_deserialized(
                    deserialized_script,
                )))
                .deserialized()
                .clone(),
        }
    }

    fn insert_verified_script(
        &self,
        key: Self::Key,
        verified_script: Self::Verified,
    ) -> Arc<Self::Verified> {
        use dashmap::mapref::entry::Entry::*;

        match self.script_cache.entry(key) {
            Occupied(mut entry) => {
                if !entry.get().is_verified() {
                    let new_script = Code::from_verified(verified_script);
                    let verified_script = new_script.verified().clone();
                    entry.insert(CachePadded::new(new_script));
                    verified_script
                } else {
                    entry.get().verified().clone()
                }
            },
            Vacant(entry) => entry
                .insert(CachePadded::new(Code::from_verified(verified_script)))
                .verified()
                .clone(),
        }
    }

    fn get_script(&self, key: &Self::Key) -> Option<Code<Self::Deserialized, Self::Verified>> {
        let script = &**self.script_cache.get(key)?;
        Some(script.clone())
    }

    fn num_scripts(&self) -> usize {
        self.script_cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{MockDeserializedCode, MockVerifiedCode};
    use claims::{assert_ok, assert_some};
    use std::collections::BTreeSet;

    fn insert_deserialized_test_case(
        script_cache: &impl ScriptCache<
            Key = usize,
            Deserialized = MockDeserializedCode,
            Verified = MockVerifiedCode,
        >,
    ) {
        // New entries.
        let deserialized_script_1 =
            script_cache.insert_deserialized_script(1, MockDeserializedCode::new(1));
        let deserialized_script_2 =
            script_cache.insert_deserialized_script(2, MockDeserializedCode::new(2));
        assert_eq!(script_cache.num_scripts(), 2);
        assert_eq!(deserialized_script_1.value(), 1);
        assert_eq!(deserialized_script_2.value(), 2);

        // Script cache already stores a deserialized script.
        let deserialized_script =
            script_cache.insert_deserialized_script(1, MockDeserializedCode::new(100));
        assert_eq!(script_cache.num_scripts(), 2);
        assert_eq!(deserialized_script.value(), 1);

        script_cache.insert_verified_script(3, MockVerifiedCode::new(3));
        assert_eq!(script_cache.num_scripts(), 3);

        // Script cache already stores a verified script.
        let deserialized_script =
            script_cache.insert_deserialized_script(3, MockDeserializedCode::new(300));
        assert_eq!(script_cache.num_scripts(), 3);
        assert_eq!(deserialized_script.value(), 3);

        // Check states.
        let script_1 = assert_some!(script_cache.get_script(&1));
        let script_2 = assert_some!(script_cache.get_script(&2));
        let script_3 = assert_some!(script_cache.get_script(&3));
        assert!(matches!(script_1, Code::Deserialized(s) if s.value() == 1));
        assert!(matches!(script_2, Code::Deserialized(s) if s.value() == 2));
        assert!(matches!(script_3, Code::Verified(s) if s.value() == 3));
    }

    fn insert_verified_test_case(
        script_cache: &impl ScriptCache<
            Key = usize,
            Deserialized = MockDeserializedCode,
            Verified = MockVerifiedCode,
        >,
    ) {
        // New entries.
        let verified_script_1 = script_cache.insert_verified_script(1, MockVerifiedCode::new(1));
        let verified_script_2 = script_cache.insert_verified_script(2, MockVerifiedCode::new(2));
        assert_eq!(script_cache.num_scripts(), 2);
        assert_eq!(verified_script_1.value(), 1);
        assert_eq!(verified_script_2.value(), 2);

        // Script cache already stores a verified script.
        let verified_script = script_cache.insert_verified_script(1, MockVerifiedCode::new(100));
        assert_eq!(script_cache.num_scripts(), 2);
        assert_eq!(verified_script.value(), 1);

        script_cache.insert_deserialized_script(3, MockDeserializedCode::new(300));
        assert_eq!(script_cache.num_scripts(), 3);

        // Script cache only has a deserialized script, it is fine to override it.
        let verified_script = script_cache.insert_verified_script(3, MockVerifiedCode::new(3));
        assert_eq!(script_cache.num_scripts(), 3);
        assert_eq!(verified_script.value(), 3);

        // Check states.
        let script_1 = assert_some!(script_cache.get_script(&1));
        let script_2 = assert_some!(script_cache.get_script(&2));
        let script_3 = assert_some!(script_cache.get_script(&3));
        assert!(matches!(script_1, Code::Verified(s) if s.value() == 1));
        assert!(matches!(script_2, Code::Verified(s) if s.value() == 2));
        assert!(matches!(script_3, Code::Verified(s) if s.value() == 3));
    }

    fn test_get_script_test_case(
        script_cache: &impl ScriptCache<
            Key = usize,
            Deserialized = MockDeserializedCode,
            Verified = MockVerifiedCode,
        >,
    ) {
        assert_eq!(script_cache.num_scripts(), 0);
        assert!(script_cache.get_script(&1).is_none());

        script_cache.insert_deserialized_script(1, MockDeserializedCode::new(1));
        script_cache.insert_verified_script(2, MockVerifiedCode::new(2));
        assert_eq!(script_cache.num_scripts(), 2);

        let script_1 = assert_some!(script_cache.get_script(&1));
        assert!(matches!(script_1, Code::Deserialized(s) if s.value() == 1));

        let script_2 = assert_some!(script_cache.get_script(&2));
        assert!(matches!(script_2, Code::Verified(s) if s.value() == 2));
    }

    #[test]
    fn test_insert_deserialized_script() {
        insert_deserialized_test_case(&UnsyncScriptCache::empty());
        insert_deserialized_test_case(&SyncScriptCache::empty());
    }

    #[test]
    fn test_insert_verified_script() {
        insert_verified_test_case(&UnsyncScriptCache::empty());
        insert_verified_test_case(&SyncScriptCache::empty());
    }

    #[test]
    fn test_get_script() {
        test_get_script_test_case(&UnsyncScriptCache::empty());
        test_get_script_test_case(&SyncScriptCache::empty());
    }

    #[test]
    fn test_sync_insert_deserialized_multithreaded() {
        let script_cache = Arc::new(SyncScriptCache::<usize, _, MockVerifiedCode>::empty());
        let key = 1;

        // Each thread tries to cache the same script.
        let mut handles = vec![];
        for i in 0..16 {
            let handle = std::thread::spawn({
                let script_cache = script_cache.clone();
                move || {
                    script_cache
                        .insert_deserialized_script(key, MockDeserializedCode::new(i))
                        .value()
                }
            });
            handles.push(handle);
        }

        let mut values = BTreeSet::new();
        for handle in handles {
            let value = assert_ok!(handle.join());
            values.insert(value);
        }

        // All must return the same value.
        assert_eq!(values.len(), 1);
        let value = values.pop_first().unwrap();

        assert_eq!(script_cache.num_scripts(), 1);
        let script = assert_some!(script_cache.get_script(&key));
        assert!(matches!(script, Code::Deserialized(s) if s.value() == value));
    }

    #[test]
    fn test_sync_insert_verified_multithreaded() {
        let script_cache = Arc::new(SyncScriptCache::<usize, _, _>::empty());
        let key = 1;

        let mut handles = vec![];
        for i in 0..16 {
            let handle = std::thread::spawn({
                let script_cache = script_cache.clone();
                move || {
                    // Have one thread caching verified script. This script should be in the final
                    // cache.
                    if i == 8 {
                        let verified_script = MockVerifiedCode::new(i);
                        script_cache.insert_verified_script(key, verified_script);
                    } else {
                        script_cache.insert_deserialized_script(key, MockDeserializedCode::new(i));
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            assert_ok!(handle.join());
        }

        assert_eq!(script_cache.num_scripts(), 1);
        let script = assert_some!(script_cache.get_script(&key));
        assert!(matches!(script, Code::Verified(s) if s.value() == 8));
    }
}
