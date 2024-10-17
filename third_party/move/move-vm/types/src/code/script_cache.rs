// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use hashbrown::HashMap;
use std::{cell::RefCell, hash::Hash, ops::Deref, sync::Arc};

/// An entry for the script cache that can be used by the code cache. Entries can live in cache in
/// different representations.
pub enum CachedScript<D, V> {
    /// Deserialized script, not yet verified with bytecode verifier.
    Deserialized(Arc<D>),
    /// Verified script.
    Verified(Arc<V>),
}

impl<D, V> CachedScript<D, V>
where
    V: Deref<Target = Arc<D>>,
{
    /// Returns new deserialized script.
    pub fn deserialized(deserialized_script: D) -> Self {
        Self::Deserialized(Arc::new(deserialized_script))
    }

    /// Returns new verified script.
    pub fn verified(verified_script: V) -> Self {
        Self::Verified(Arc::new(verified_script))
    }

    /// Returns true if the entry is verified.
    pub fn is_verified(&self) -> bool {
        match self {
            Self::Deserialized(_) => false,
            Self::Verified(_) => true,
        }
    }

    /// Returns the deserialized script.
    pub fn deserialized_script(&self) -> &Arc<D> {
        match self {
            Self::Deserialized(compiled_script) => compiled_script,
            Self::Verified(script) => script.deref(),
        }
    }

    /// Returns the verified script. Panics if the cached script has not been verified.
    pub fn verified_script(&self) -> &Arc<V> {
        match self {
            Self::Deserialized(_) => {
                unreachable!("This function must be called on verified scripts only")
            },
            Self::Verified(script) => script,
        }
    }
}

impl<D, V> Clone for CachedScript<D, V> {
    fn clone(&self) -> Self {
        match self {
            Self::Deserialized(d) => Self::Deserialized(d.clone()),
            Self::Verified(v) => Self::Verified(v.clone()),
        }
    }
}

/// Interface used by any script cache implementation.
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
    fn get_script(
        &self,
        key: &Self::Key,
    ) -> Option<CachedScript<Self::Deserialized, Self::Verified>>;

    /// Returns the number of scripts stored in cache.
    fn num_scripts(&self) -> usize;
}

/// Non-[Sync] implementation of script cache suitable for single-threaded execution.
pub struct UnsyncScriptCache<K, D, V> {
    script_cache: RefCell<HashMap<K, CachedScript<D, V>>>,
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
            Occupied(entry) => entry.get().deserialized_script().clone(),
            Vacant(entry) => entry
                .insert(CachedScript::deserialized(deserialized_script))
                .deserialized_script()
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
                    let new_script = CachedScript::verified(verified_script);
                    let verified_script = new_script.verified_script().clone();
                    entry.insert(new_script);
                    verified_script
                } else {
                    entry.get().verified_script().clone()
                }
            },
            Vacant(entry) => entry
                .insert(CachedScript::verified(verified_script))
                .verified_script()
                .clone(),
        }
    }

    fn get_script(
        &self,
        key: &Self::Key,
    ) -> Option<CachedScript<Self::Deserialized, Self::Verified>> {
        self.script_cache.borrow().get(key).cloned()
    }

    fn num_scripts(&self) -> usize {
        self.script_cache.borrow().len()
    }
}

/// [Sync] implementation of script cache suitable for multithreaded execution.
pub struct SyncScriptCache<K, D, V> {
    script_cache: DashMap<K, CachePadded<CachedScript<D, V>>>,
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
            Occupied(entry) => entry.get().deserialized_script().clone(),
            Vacant(entry) => entry
                .insert(CachePadded::new(CachedScript::deserialized(
                    deserialized_script,
                )))
                .deserialized_script()
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
                    let new_script = CachedScript::verified(verified_script);
                    let verified_script = new_script.verified_script().clone();
                    entry.insert(CachePadded::new(new_script));
                    verified_script
                } else {
                    entry.get().verified_script().clone()
                }
            },
            Vacant(entry) => entry
                .insert(CachePadded::new(CachedScript::verified(verified_script)))
                .verified_script()
                .clone(),
        }
    }

    fn get_script(
        &self,
        key: &Self::Key,
    ) -> Option<CachedScript<Self::Deserialized, Self::Verified>> {
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
    use claims::{assert_ok, assert_some};
    use std::collections::BTreeSet;

    struct MockDeserializedScript(usize);

    impl MockDeserializedScript {
        fn value(&self) -> usize {
            self.0
        }
    }

    struct MockVerifiedScript(Arc<MockDeserializedScript>);

    impl Deref for MockVerifiedScript {
        type Target = Arc<MockDeserializedScript>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    fn insert_deserialized_test_case(
        script_cache: &impl ScriptCache<
            Key = usize,
            Deserialized = MockDeserializedScript,
            Verified = MockVerifiedScript,
        >,
    ) {
        // New entries.
        let deserialized_script_1 =
            script_cache.insert_deserialized_script(1, MockDeserializedScript(1));
        let deserialized_script_2 =
            script_cache.insert_deserialized_script(2, MockDeserializedScript(2));
        assert_eq!(script_cache.num_scripts(), 2);
        assert_eq!(deserialized_script_1.value(), 1);
        assert_eq!(deserialized_script_2.value(), 2);

        // Script cache already stores a deserialized script.
        let deserialized_script =
            script_cache.insert_deserialized_script(1, MockDeserializedScript(100));
        assert_eq!(script_cache.num_scripts(), 2);
        assert_eq!(deserialized_script.value(), 1);

        script_cache
            .insert_verified_script(3, MockVerifiedScript(Arc::new(MockDeserializedScript(3))));
        assert_eq!(script_cache.num_scripts(), 3);

        // Script cache already stores a verified script.
        let deserialized_script =
            script_cache.insert_deserialized_script(3, MockDeserializedScript(300));
        assert_eq!(script_cache.num_scripts(), 3);
        assert_eq!(deserialized_script.value(), 3);

        // Check states.
        let script_1 = assert_some!(script_cache.get_script(&1));
        let script_2 = assert_some!(script_cache.get_script(&2));
        let script_3 = assert_some!(script_cache.get_script(&3));
        assert!(matches!(script_1, CachedScript::Deserialized(s) if s.value() == 1));
        assert!(matches!(script_2, CachedScript::Deserialized(s) if s.value() == 2));
        assert!(matches!(script_3, CachedScript::Verified(s) if s.value() == 3));
    }

    fn insert_verified_test_case(
        script_cache: &impl ScriptCache<
            Key = usize,
            Deserialized = MockDeserializedScript,
            Verified = MockVerifiedScript,
        >,
    ) {
        // New entries.
        let verified_script_1 = script_cache
            .insert_verified_script(1, MockVerifiedScript(Arc::new(MockDeserializedScript(1))));
        let verified_script_2 = script_cache
            .insert_verified_script(2, MockVerifiedScript(Arc::new(MockDeserializedScript(2))));
        assert_eq!(script_cache.num_scripts(), 2);
        assert_eq!(verified_script_1.value(), 1);
        assert_eq!(verified_script_2.value(), 2);

        // Script cache already stores a verified script.
        let verified_script = script_cache
            .insert_verified_script(1, MockVerifiedScript(Arc::new(MockDeserializedScript(100))));
        assert_eq!(script_cache.num_scripts(), 2);
        assert_eq!(verified_script.value(), 1);

        script_cache.insert_deserialized_script(3, MockDeserializedScript(300));
        assert_eq!(script_cache.num_scripts(), 3);

        // Script cache only has a deserialized script, it is fine to override it.
        let verified_script = script_cache
            .insert_verified_script(3, MockVerifiedScript(Arc::new(MockDeserializedScript(3))));
        assert_eq!(script_cache.num_scripts(), 3);
        assert_eq!(verified_script.value(), 3);

        // Check states.
        let script_1 = assert_some!(script_cache.get_script(&1));
        let script_2 = assert_some!(script_cache.get_script(&2));
        let script_3 = assert_some!(script_cache.get_script(&3));
        assert!(matches!(script_1, CachedScript::Verified(s) if s.value() == 1));
        assert!(matches!(script_2, CachedScript::Verified(s) if s.value() == 2));
        assert!(matches!(script_3, CachedScript::Verified(s) if s.value() == 3));
    }

    fn test_get_script_test_case(
        script_cache: &impl ScriptCache<
            Key = usize,
            Deserialized = MockDeserializedScript,
            Verified = MockVerifiedScript,
        >,
    ) {
        assert_eq!(script_cache.num_scripts(), 0);
        assert!(script_cache.get_script(&1).is_none());

        script_cache.insert_deserialized_script(1, MockDeserializedScript(1));
        script_cache
            .insert_verified_script(2, MockVerifiedScript(Arc::new(MockDeserializedScript(2))));
        assert_eq!(script_cache.num_scripts(), 2);

        let script_1 = assert_some!(script_cache.get_script(&1));
        assert!(matches!(script_1, CachedScript::Deserialized(s) if s.value() == 1));

        let script_2 = assert_some!(script_cache.get_script(&2));
        assert!(matches!(script_2, CachedScript::Verified(s) if s.value() == 2));
    }

    #[test]
    fn test_deserialized_cached_script() {
        let script: CachedScript<_, MockVerifiedScript> =
            CachedScript::deserialized(MockDeserializedScript(1));
        assert!(!script.is_verified());
        assert_eq!(script.deserialized_script().value(), 1);
        assert!(matches!(script, CachedScript::Deserialized(..)));
    }

    #[test]
    #[should_panic]
    fn test_deserialized_cached_script_panics() {
        let script: CachedScript<_, MockVerifiedScript> =
            CachedScript::deserialized(MockDeserializedScript(1));
        script.verified_script();
    }

    #[test]
    fn test_verified_cached_script() {
        let script =
            CachedScript::verified(MockVerifiedScript(Arc::new(MockDeserializedScript(1))));
        assert!(script.is_verified());
        assert_eq!(script.deserialized_script().value(), 1);
        assert_eq!(script.verified_script().value(), 1);
        assert!(matches!(script, CachedScript::Verified(..)));
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
        let script_cache = Arc::new(SyncScriptCache::<usize, _, MockVerifiedScript>::empty());
        let key = 1;

        // Each thread tries to cache the same script.
        let mut handles = vec![];
        for i in 0..16 {
            let handle = std::thread::spawn({
                let script_cache = script_cache.clone();
                move || {
                    script_cache
                        .insert_deserialized_script(key, MockDeserializedScript(i))
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
        assert!(matches!(script, CachedScript::Deserialized(s) if s.value() == value));
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
                        let verified_script =
                            MockVerifiedScript(Arc::new(MockDeserializedScript(i)));
                        script_cache.insert_verified_script(key, verified_script);
                    } else {
                        script_cache.insert_deserialized_script(key, MockDeserializedScript(i));
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
        assert!(matches!(script, CachedScript::Verified(s) if s.value() == 8));
    }
}
