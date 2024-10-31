// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::code::Code;
use ambassador::delegatable_trait;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use hashbrown::HashMap;
use move_binary_format::errors::VMResult;
use std::{cell::RefCell, cmp::Ordering, hash::Hash, mem, ops::Deref, sync::Arc};

/// Represents module code stored in [ModuleCode].
pub struct ModuleCode<DC, VC, E, V> {
    /// Module's code, either deserialized or verified.
    code: Code<DC, VC>,
    /// Module's extension - any additional metadata associated with this module.
    extension: Arc<E>,
    /// Version of the code (e.g., which transaction within the block published this module).
    version: V,
}

impl<DC, VC, E, V> ModuleCode<DC, VC, E, V>
where
    VC: Deref<Target = Arc<DC>>,
    V: Clone + Ord,
{
    /// Creates new [ModuleCode] from deserialized code.
    pub fn from_deserialized(deserialized_code: DC, extension: Arc<E>, version: V) -> Self {
        Self {
            code: Code::from_deserialized(deserialized_code),
            extension,
            version,
        }
    }

    /// Creates new [ModuleCode] from verified code.
    pub fn from_verified(verified_code: VC, extension: Arc<E>, version: V) -> Self {
        Self {
            code: Code::from_verified(verified_code),
            extension,
            version,
        }
    }

    /// Returns module's code.
    pub fn code(&self) -> &Code<DC, VC> {
        &self.code
    }

    /// Returns module's extensions.
    pub fn extension(&self) -> &Arc<E> {
        &self.extension
    }

    /// Returns module's version.
    pub fn version(&self) -> V {
        self.version.clone()
    }

    /// Sets the module version.
    pub fn set_version(&mut self, version: V) {
        self.version = version;
    }
}

impl<DC, VC, E, V> Clone for ModuleCode<DC, VC, E, V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            code: self.code.clone(),
            extension: self.extension.clone(),
            version: self.version.clone(),
        }
    }
}

/// Interface for building module code to be stored in cache, e.g., if it is not yet cached.
pub trait ModuleCodeBuilder {
    type Key: Eq + Hash + Clone;
    type Deserialized;
    type Verified;
    type Extension;
    type Version: Clone + Ord;

    /// For the given key, returns [ModuleCode] if it exists, and [None] otherwise. In case
    /// initialization fails, returns an error.
    fn build(
        &self,
        key: &Self::Key,
    ) -> VMResult<
        Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>,
    >;
}

/// Interface used by any module cache implementation.
#[delegatable_trait]
pub trait ModuleCache {
    type Key: Eq + Hash + Clone;
    type Deserialized;
    type Verified;
    type Extension;
    type Version: Clone + Ord;

    /// Stores deserialized code at specified version to the module cache if there was no entry
    /// associated with this key before. If module cache already contains an entry, then:
    ///   1. returns an error if the version of existing entry is higher,
    ///   2. does not perform the insertion if the version is the same,
    ///   3. inserts the new code if the new version is higher.
    fn insert_deserialized_module(
        &self,
        key: Self::Key,
        deserialized_code: Self::Deserialized,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<()>;

    /// Stores verified code at specified version to the module cache if there was no entry
    /// associated with this key before. If module cache already contains an entry, then:
    ///   1. returns an error if the version of existing entry is higher,
    ///   2. does not perform the insertion if the version is the same and the entry is verified,
    ///   3. inserts the new code if the new version is higher, or if the version is the same but
    ///      the code is not verified.
    /// Returns the newly inserted (or existing) module at the specified key.
    fn insert_verified_module(
        &self,
        key: Self::Key,
        verified_code: Self::Verified,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>>;

    /// Ensures that the entry in the module cache is initialized using the provided initializer,
    /// if it was not stored before. Returns the stored module, or [None] if it does not exist. If
    /// initialization fails, returns the error. The caller can also provide the kind of read they
    /// intend to do.
    fn get_module_or_build_with(
        &self,
        key: &Self::Key,
        builder: &dyn ModuleCodeBuilder<
            Key = Self::Key,
            Deserialized = Self::Deserialized,
            Verified = Self::Verified,
            Extension = Self::Extension,
            Version = Self::Version,
        >,
    ) -> VMResult<
        Option<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>>,
    >;

    /// Returns the number of modules in cache.
    fn num_modules(&self) -> usize;
}

/// An error when inserting a module with a smaller version to module cache containing the higher
/// version.
macro_rules! version_too_small_error {
    () => {
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
        )
        .with_message("Trying to insert smaller version that exists in module cache".to_string())
        .finish(move_binary_format::errors::Location::Undefined)
    };
}

/// Non-[Sync] version of module cache suitable for sequential execution.
pub struct UnsyncModuleCache<K, DC, VC, E, V> {
    module_cache: RefCell<HashMap<K, Arc<ModuleCode<DC, VC, E, V>>>>,
}

impl<K, DC, VC, E, V> UnsyncModuleCache<K, DC, VC, E, V>
where
    K: Eq + Hash + Clone,
    VC: Deref<Target = Arc<DC>>,
    V: Clone + Ord,
{
    /// Returns an empty module cache.
    pub fn empty() -> Self {
        Self {
            module_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Returns the iterator to all keys and modules stored in the cache.
    pub fn into_modules_iter(self) -> impl Iterator<Item = (K, Arc<ModuleCode<DC, VC, E, V>>)> {
        self.module_cache.into_inner().into_iter()
    }
}

impl<K, DC, VC, E, V> ModuleCache for UnsyncModuleCache<K, DC, VC, E, V>
where
    K: Eq + Hash + Clone,
    VC: Deref<Target = Arc<DC>>,
    V: Clone + Ord,
{
    type Deserialized = DC;
    type Extension = E;
    type Key = K;
    type Verified = VC;
    type Version = V;

    fn insert_deserialized_module(
        &self,
        key: Self::Key,
        deserialized_code: Self::Deserialized,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<()> {
        use hashbrown::hash_map::Entry::*;

        match self.module_cache.borrow_mut().entry(key) {
            Occupied(mut entry) => match version.cmp(&entry.get().version()) {
                Ordering::Less => Err(version_too_small_error!()),
                Ordering::Equal => Ok(()),
                Ordering::Greater => {
                    let module = Arc::new(ModuleCode::from_deserialized(
                        deserialized_code,
                        extension,
                        version,
                    ));
                    entry.insert(module);
                    Ok(())
                },
            },
            Vacant(entry) => {
                let module = ModuleCode::from_deserialized(deserialized_code, extension, version);
                entry.insert(Arc::new(module));
                Ok(())
            },
        }
    }

    fn insert_verified_module(
        &self,
        key: Self::Key,
        verified_code: Self::Verified,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>>
    {
        use hashbrown::hash_map::Entry::*;

        match self.module_cache.borrow_mut().entry(key) {
            Occupied(mut entry) => match version.cmp(&entry.get().version()) {
                Ordering::Less => Err(version_too_small_error!()),
                Ordering::Equal => {
                    if entry.get().code().is_verified() {
                        Ok(entry.get().clone())
                    } else {
                        let module =
                            Arc::new(ModuleCode::from_verified(verified_code, extension, version));
                        entry.insert(module.clone());
                        Ok(module)
                    }
                },
                Ordering::Greater => {
                    let module =
                        Arc::new(ModuleCode::from_verified(verified_code, extension, version));
                    entry.insert(module.clone());
                    Ok(module)
                },
            },
            Vacant(entry) => {
                let module = ModuleCode::from_verified(verified_code, extension, version);
                Ok(entry.insert(Arc::new(module)).clone())
            },
        }
    }

    fn get_module_or_build_with(
        &self,
        key: &Self::Key,
        builder: &dyn ModuleCodeBuilder<
            Key = Self::Key,
            Deserialized = Self::Deserialized,
            Verified = Self::Verified,
            Extension = Self::Extension,
            Version = Self::Version,
        >,
    ) -> VMResult<
        Option<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>>,
    > {
        use hashbrown::hash_map::Entry::*;

        Ok(match self.module_cache.borrow_mut().entry(key.clone()) {
            Occupied(entry) => Some(entry.get().clone()),
            Vacant(entry) => builder
                .build(key)?
                .map(|module| entry.insert(Arc::new(module)).clone()),
        })
    }

    fn num_modules(&self) -> usize {
        self.module_cache.borrow().len()
    }
}

/// [Sync] version of module cache, suitable for parallel execution.
pub struct SyncModuleCache<K, DC, VC, E, V> {
    module_cache: DashMap<K, CachePadded<Arc<ModuleCode<DC, VC, E, V>>>>,
}

impl<K, DC, VC, E, V> SyncModuleCache<K, DC, VC, E, V>
where
    K: Eq + Hash + Clone,
    VC: Deref<Target = Arc<DC>>,
    V: Clone + Ord,
{
    /// Returns a new empty module cache.
    pub fn empty() -> Self {
        Self {
            module_cache: DashMap::new(),
        }
    }

    /// Returns the version of the module the cache contains. Returns [None] if cache does not have
    /// the module.
    pub fn get_module_version(&self, key: &K) -> Option<V> {
        self.module_cache.get(key).map(|module| module.version())
    }

    /// Takes the modules stored in the module cache, and returns an iterator of keys and modules.
    pub fn take_modules_iter(
        &mut self,
    ) -> impl Iterator<Item = (K, Arc<ModuleCode<DC, VC, E, V>>)> {
        mem::take(&mut self.module_cache)
            .into_iter()
            .map(|(key, module)| (key, module.into_inner()))
    }
}

impl<K, DC, VC, E, V> ModuleCache for SyncModuleCache<K, DC, VC, E, V>
where
    K: Eq + Hash + Clone,
    VC: Deref<Target = Arc<DC>>,
    V: Clone + Ord,
{
    type Deserialized = DC;
    type Extension = E;
    type Key = K;
    type Verified = VC;
    type Version = V;

    fn insert_deserialized_module(
        &self,
        key: Self::Key,
        deserialized_code: Self::Deserialized,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<()> {
        use dashmap::mapref::entry::Entry::*;

        match self.module_cache.entry(key) {
            Occupied(mut entry) => match version.cmp(&entry.get().version()) {
                Ordering::Less => Err(version_too_small_error!()),
                Ordering::Equal => Ok(()),
                Ordering::Greater => {
                    let module = Arc::new(ModuleCode::from_deserialized(
                        deserialized_code,
                        extension,
                        version,
                    ));
                    entry.insert(CachePadded::new(module));
                    Ok(())
                },
            },
            Vacant(entry) => {
                let module = ModuleCode::from_deserialized(deserialized_code, extension, version);
                entry.insert(CachePadded::new(Arc::new(module)));
                Ok(())
            },
        }
    }

    fn insert_verified_module(
        &self,
        key: Self::Key,
        verified_code: Self::Verified,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>>
    {
        use dashmap::mapref::entry::Entry::*;

        match self.module_cache.entry(key) {
            Occupied(mut entry) => match version.cmp(&entry.get().version()) {
                Ordering::Less => Err(version_too_small_error!()),
                Ordering::Equal => {
                    if entry.get().code().is_verified() {
                        Ok(entry.get().deref().clone())
                    } else {
                        let module =
                            Arc::new(ModuleCode::from_verified(verified_code, extension, version));
                        entry.insert(CachePadded::new(module.clone()));
                        Ok(module)
                    }
                },
                Ordering::Greater => {
                    let module =
                        Arc::new(ModuleCode::from_verified(verified_code, extension, version));
                    entry.insert(CachePadded::new(module.clone()));
                    Ok(module)
                },
            },
            Vacant(entry) => {
                let module = ModuleCode::from_verified(verified_code, extension, version);
                Ok(entry
                    .insert(CachePadded::new(Arc::new(module)))
                    .deref()
                    .deref()
                    .clone())
            },
        }
    }

    fn get_module_or_build_with(
        &self,
        key: &Self::Key,
        builder: &dyn ModuleCodeBuilder<
            Key = Self::Key,
            Deserialized = Self::Deserialized,
            Verified = Self::Verified,
            Extension = Self::Extension,
            Version = Self::Version,
        >,
    ) -> VMResult<
        Option<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>>,
    > {
        use dashmap::mapref::entry::Entry::*;

        if let Some(module) = self.module_cache.get(key) {
            return Ok(Some(Arc::clone(module.deref())));
        }

        Ok(match self.module_cache.entry(key.clone()) {
            Occupied(entry) => Some(Arc::clone(entry.get())),
            Vacant(entry) => match builder.build(key)? {
                Some(module) => {
                    let module = Arc::new(module);
                    entry.insert(CachePadded::new(module.clone()));
                    Some(module)
                },
                None => None,
            },
        })
    }

    fn num_modules(&self) -> usize {
        self.module_cache.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::code::{MockDeserializedCode, MockVerifiedCode};
    use claims::{assert_ok, assert_some};
    use move_binary_format::errors::{Location, PartialVMError};
    use move_core_types::vm_status::StatusCode;

    struct Unreachable;

    impl ModuleCodeBuilder for Unreachable {
        type Deserialized = MockDeserializedCode;
        type Extension = ();
        type Key = usize;
        type Verified = MockVerifiedCode;
        type Version = u32;

        fn build(
            &self,
            _key: &Self::Key,
        ) -> VMResult<
            Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>,
        > {
            unreachable!("Should never be called!")
        }
    }

    struct WithSomeValue(usize);

    impl ModuleCodeBuilder for WithSomeValue {
        type Deserialized = MockDeserializedCode;
        type Extension = ();
        type Key = usize;
        type Verified = MockVerifiedCode;
        type Version = u32;

        fn build(
            &self,
            _key: &Self::Key,
        ) -> VMResult<
            Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>,
        > {
            let code = MockDeserializedCode::new(self.0);
            Ok(Some(ModuleCode::from_deserialized(code, Arc::new(()), 0)))
        }
    }

    struct WithNone;

    impl ModuleCodeBuilder for WithNone {
        type Deserialized = MockDeserializedCode;
        type Extension = ();
        type Key = usize;
        type Verified = MockVerifiedCode;
        type Version = u32;

        fn build(
            &self,
            _key: &Self::Key,
        ) -> VMResult<
            Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>,
        > {
            Ok(None)
        }
    }

    struct WithError;

    impl ModuleCodeBuilder for WithError {
        type Deserialized = MockDeserializedCode;
        type Extension = ();
        type Key = usize;
        type Verified = MockVerifiedCode;
        type Version = u32;

        fn build(
            &self,
            _key: &Self::Key,
        ) -> VMResult<
            Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension, Self::Version>>,
        > {
            Err(PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined))
        }
    }

    fn insert_deserialized_test_case(
        module_cache: &impl ModuleCache<
            Key = usize,
            Deserialized = MockDeserializedCode,
            Verified = MockVerifiedCode,
            Extension = (),
            Version = u32,
        >,
    ) {
        // New entries at version 0.
        assert_ok!(module_cache.insert_deserialized_module(
            1,
            MockDeserializedCode::new(1),
            Arc::new(()),
            0,
        ));
        assert_ok!(module_cache.insert_deserialized_module(
            2,
            MockDeserializedCode::new(2),
            Arc::new(()),
            0,
        ));

        assert_eq!(module_cache.num_modules(), 2);
        let deserialized_module_1 = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&1, &Unreachable)
        ));
        assert_eq!(deserialized_module_1.code().deserialized().value(), 1);
        let deserialized_module_2 = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&2, &Unreachable)
        ));
        assert_eq!(deserialized_module_2.code().deserialized().value(), 2);

        // Module cache already stores deserialized code at the same version.
        assert_ok!(module_cache.insert_deserialized_module(
            1,
            MockDeserializedCode::new(10),
            Arc::new(()),
            0
        ));
        assert_eq!(module_cache.num_modules(), 2);
        let deserialized_module = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&1, &Unreachable)
        ));
        assert_eq!(deserialized_module.code().deserialized().value(), 1);

        // Module cache stores deserialized code at lower version, so it should be replaced.
        assert_ok!(module_cache.insert_deserialized_module(
            1,
            MockDeserializedCode::new(100),
            Arc::new(()),
            10
        ));
        assert_eq!(module_cache.num_modules(), 2);
        let deserialized_module = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&1, &Unreachable)
        ));
        assert_eq!(deserialized_module.code().deserialized().value(), 100);

        // We already have higher-versioned deserialized code stored.
        let result = module_cache.insert_deserialized_module(
            1,
            MockDeserializedCode::new(100),
            Arc::new(()),
            5,
        );
        assert!(result.is_err());

        // Store verified module at version 10.
        assert_ok!(module_cache.insert_verified_module(
            3,
            MockVerifiedCode::new(3),
            Arc::new(()),
            10
        ));
        assert_eq!(module_cache.num_modules(), 3);

        // Lower-version cannot replace this module.
        let result = module_cache.insert_deserialized_module(
            3,
            MockDeserializedCode::new(30),
            Arc::new(()),
            0,
        );
        assert!(result.is_err());

        // Same version does not replace the stored module, so old value should be returned.
        assert_ok!(module_cache.insert_deserialized_module(
            3,
            MockDeserializedCode::new(300),
            Arc::new(()),
            10
        ));
        assert_eq!(module_cache.num_modules(), 3);
        let deserialized_module = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&3, &Unreachable)
        ));
        assert_eq!(deserialized_module.code().deserialized().value(), 3);

        // If the version is higher, we replace the module even though it was verified before.
        assert_ok!(module_cache.insert_deserialized_module(
            3,
            MockDeserializedCode::new(3000),
            Arc::new(()),
            20
        ));
        assert_eq!(module_cache.num_modules(), 3);
        let deserialized_module = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&3, &Unreachable)
        ));
        assert_eq!(deserialized_module.code().deserialized().value(), 3000);

        // Check states.
        let module_1 = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&1, &Unreachable)
        ));
        let module_2 = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&2, &Unreachable)
        ));
        let module_3 = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&3, &Unreachable)
        ));
        assert!(matches!(module_1.code(), Code::Deserialized(s) if s.value() == 100));
        assert!(matches!(module_2.code(), Code::Deserialized(s) if s.value() == 2));
        assert!(matches!(module_3.code(), Code::Deserialized(s) if s.value() == 3000));
    }

    fn insert_verified_test_case(
        module_cache: &impl ModuleCache<
            Key = usize,
            Deserialized = MockDeserializedCode,
            Verified = MockVerifiedCode,
            Extension = (),
            Version = u32,
        >,
    ) {
        // New verified entries at version 10.
        let verified_module_1 = assert_ok!(module_cache.insert_verified_module(
            1,
            MockVerifiedCode::new(1),
            Arc::new(()),
            10,
        ));
        let verified_module_2 = assert_ok!(module_cache.insert_verified_module(
            2,
            MockVerifiedCode::new(2),
            Arc::new(()),
            10,
        ));

        assert_eq!(module_cache.num_modules(), 2);
        assert!(verified_module_1.code().is_verified() && verified_module_2.code().is_verified());
        assert_eq!(verified_module_1.code().deserialized().value(), 1);
        assert_eq!(verified_module_2.code().deserialized().value(), 2);

        // Module cache already stores verified code at the same version (10), so inserting new
        // code is a noop.
        assert_ok!(module_cache.insert_deserialized_module(
            2,
            MockDeserializedCode::new(20),
            Arc::new(()),
            10
        ));
        assert_eq!(module_cache.num_modules(), 2);
        let deserialized_module = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&2, &Unreachable)
        ));
        assert_eq!(deserialized_module.code().deserialized().value(), 2);
        let verified_module = assert_ok!(module_cache.insert_verified_module(
            2,
            MockVerifiedCode::new(200),
            Arc::new(()),
            10
        ));
        assert_eq!(module_cache.num_modules(), 2);
        assert_eq!(verified_module.code().deserialized().value(), 2);

        // Module cache does not add verified or deserialized code at lower versions (0).
        let result = module_cache.insert_deserialized_module(
            1,
            MockDeserializedCode::new(10),
            Arc::new(()),
            0,
        );
        assert!(result.is_err());
        let result =
            module_cache.insert_verified_module(1, MockVerifiedCode::new(100), Arc::new(()), 0);
        assert!(result.is_err());

        // Higher versions should be inserted, whether they are verified or deserialized.
        assert_ok!(module_cache.insert_deserialized_module(
            1,
            MockDeserializedCode::new(1000),
            Arc::new(()),
            100
        ));
        assert_eq!(module_cache.num_modules(), 2);
        let deserialized_module = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&1, &Unreachable)
        ));
        assert!(!deserialized_module.code().is_verified());
        assert_eq!(deserialized_module.code().deserialized().value(), 1000);

        let verified_module = assert_ok!(module_cache.insert_verified_module(
            1,
            MockVerifiedCode::new(10_000),
            Arc::new(()),
            1000
        ));
        assert_eq!(module_cache.num_modules(), 2);
        assert!(verified_module.code().is_verified());
        assert_eq!(verified_module.code().deserialized().value(), 10_000);

        // Check states.
        let module_1 = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&1, &Unreachable)
        ));
        let module_2 = assert_some!(assert_ok!(
            module_cache.get_module_or_build_with(&2, &Unreachable)
        ));
        assert!(matches!(module_1.code(), Code::Verified(s) if s.value() == 10_000));
        assert!(matches!(module_2.code(), Code::Verified(s) if s.value() == 2));
    }

    fn get_module_or_initialize_with_test_case(
        module_cache: &impl ModuleCache<
            Key = usize,
            Deserialized = MockDeserializedCode,
            Verified = MockVerifiedCode,
            Extension = (),
            Version = u32,
        >,
    ) {
        assert_ok!(module_cache.insert_deserialized_module(
            1,
            MockDeserializedCode::new(1),
            Arc::new(()),
            0,
        ));
        assert_ok!(module_cache.insert_verified_module(
            2,
            MockVerifiedCode::new(2),
            Arc::new(()),
            0,
        ));

        // Get existing deserialized module.
        let result = module_cache.get_module_or_build_with(&1, &WithSomeValue(100));
        let module_1 = assert_some!(assert_ok!(result));
        assert!(!module_1.code().is_verified());
        assert_eq!(module_1.code().deserialized().value(), 1);

        // Get existing verified module.
        let result = module_cache.get_module_or_build_with(&2, &WithError);
        let module_2 = assert_some!(assert_ok!(result));
        assert!(module_2.code().is_verified());
        assert_eq!(module_2.code().deserialized().value(), 2);

        // Error when initializing.
        assert!(module_cache
            .get_module_or_build_with(&3, &WithError)
            .is_err());
        assert_eq!(module_cache.num_modules(), 2);

        // Module does not exist.
        let result = module_cache.get_module_or_build_with(&3, &WithNone);
        assert!(assert_ok!(result).is_none());
        assert_eq!(module_cache.num_modules(), 2);

        // Successful initialization.
        let result = module_cache.get_module_or_build_with(&3, &WithSomeValue(300));
        let module_3 = assert_some!(assert_ok!(result));
        assert!(!module_3.code().is_verified());
        assert_eq!(module_3.code().deserialized().value(), 300);
        assert_eq!(module_cache.num_modules(), 3);

        let result = module_cache.get_module_or_build_with(&3, &WithSomeValue(1000));
        let module_3 = assert_some!(assert_ok!(result));
        assert!(!module_3.code().is_verified());
        assert_eq!(module_3.code().deserialized().value(), 300);
        assert_eq!(module_cache.num_modules(), 3);
    }

    #[test]
    fn test_insert_deserialized_module() {
        insert_deserialized_test_case(&UnsyncModuleCache::empty());
        insert_deserialized_test_case(&SyncModuleCache::empty());
    }

    #[test]
    fn test_insert_verified_module() {
        insert_verified_test_case(&UnsyncModuleCache::empty());
        insert_verified_test_case(&SyncModuleCache::empty());
    }

    #[test]
    fn test_get_module_or_initialize_with() {
        get_module_or_initialize_with_test_case(&UnsyncModuleCache::empty());
        get_module_or_initialize_with_test_case(&SyncModuleCache::empty());
    }
}
