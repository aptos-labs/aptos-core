// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The in-memory [`ResourceProvider`] shared by the MonoMove test harnesses.
//!
//! Resources are held as BCS bytes in V1's [`InMemoryStorage`] and
//! materialized (BCS -> flat) into an arena the provider owns on first access,
//! then served as a pointer that pins the arena segment it is in. Materialization is lazy
//! because a type's layout and GC descriptor are only published once the
//! function that accesses it has been lowered.

use aptos_types::on_chain_config::{Features, OnChainConfig};
use mono_move_core::{
    nominal_tag,
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
    ReadPin,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::SegmentedArena;
use move_core_types::account_address::AccountAddress;
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::resolver::ResourceResolver;
use std::{cell::RefCell, collections::HashMap, ptr::NonNull, sync::Arc};

/// Serves the resources of V1's [`InMemoryStorage`] to MonoMove, looking each
/// up by the struct tag of its interned type and materializing it on first
/// access.
pub(crate) struct InMemoryResourceProvider<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    storage: &'guard InMemoryStorage,
    materialized: RefCell<Materialized>,
}

struct Materialized {
    /// Holds the flat representation of materialized values. Each read pins
    /// the segment its value is in; never reset, so every interpreter reading
    /// a value keeps it alive.
    arena: SegmentedArena,
    cache: HashMap<InMemoryStorageKey, (NonNull<u8>, Arc<dyn ReadPin>)>,
}

impl<'guard, 'ctx> InMemoryResourceProvider<'guard, 'ctx> {
    /// Creates a provider over `storage`.
    pub(crate) fn new(
        guard: &'guard ExecutionGuard<'ctx>,
        storage: &'guard InMemoryStorage,
    ) -> Self {
        Self {
            guard,
            storage,
            materialized: RefCell::new(Materialized {
                arena: SegmentedArena::new(),
                cache: HashMap::new(),
            }),
        }
    }

    /// The value already materialized for `key`, if any.
    fn cached(&self, key: &InMemoryStorageKey) -> Option<StorageRead> {
        self.materialized
            .borrow()
            .cache
            .get(key)
            .map(|(ptr, pin)| StorageRead::ExternalHeap {
                ptr: *ptr,
                version: None,
                pin: pin.clone(),
            })
    }

    /// Materializes `blob` as a value of type `ty` and caches it under `key`.
    fn materialize(
        &self,
        key: &InMemoryStorageKey,
        ty: InternedType,
        blob: &[u8],
    ) -> Result<StorageRead, ResourceProviderError> {
        let mut materialized = self.materialized.borrow_mut();
        let (ptr, pin) = materialized.arena.deserialize(self.guard, ty, blob)?;
        materialized.cache.insert(key.clone(), (ptr, pin.clone()));
        Ok(StorageRead::ExternalHeap {
            ptr,
            version: None,
            pin,
        })
    }
}

impl ResourceProvider for InMemoryResourceProvider<'_, '_> {
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        _group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        if let Some(read) = self.cached(key) {
            return Ok(read);
        }
        match key {
            InMemoryStorageKey::Resource { address, ty } => {
                let tag = nominal_tag(*ty)
                    .map_err(|err| ResourceProviderError::InvariantViolation(format!("{err:#}")))?;
                let (blob, _) = self
                    .storage
                    .get_resource_bytes_with_metadata_and_layout(address, &tag, &[], None)
                    .map_err(|err| ResourceProviderError::InvariantViolation(err.to_string()))?;
                match blob {
                    Some(blob) => self.materialize(key, *ty, &blob),
                    None => Ok(StorageRead::DoesNotExist { version: None }),
                }
            },
            // TODO(testing): serve table items from the storage's tables once the
            // session commits table writes, so transactional sources can keep tables
            // across tasks.
            InMemoryStorageKey::TableItem { .. } => Ok(StorageRead::DoesNotExist { version: None }),
            // TODO(testing): members are served by their own key and the group is
            // ignored. Serve them from the group's slot so transactional sources
            // cover resource-group placement.
            InMemoryStorageKey::ResourceGroup { .. } => {
                Ok(StorageRead::DoesNotExist { version: None })
            },
        }
    }
}

/// Returns storage with default features for testing enabled.
pub(crate) fn storage_with_default_features_for_test() -> InMemoryStorage {
    let mut storage = InMemoryStorage::new();
    storage.publish_or_overwrite_resource(
        AccountAddress::ONE,
        Features::struct_tag(),
        features_blob(),
    );
    storage
}

/// BCS bytes of the `Features` resource the test harnesses seed at `0x1`.
pub(crate) fn features_blob() -> Vec<u8> {
    bcs::to_bytes(&Features::default_for_tests()).expect("Features serializes")
}
