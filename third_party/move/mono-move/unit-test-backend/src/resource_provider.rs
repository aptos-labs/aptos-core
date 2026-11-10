// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Serves resources to MonoMove from a lookup of their BCS bytes.
//!
//! Resources are decoded from BCS into flat values in the provider's arena
//! on first access. Each storage read includes a pointer and pins the arena
//! segment it is in.
//! Materialization is lazy because a type's layout and GC descriptor are only
//! published once the function that accesses it has been lowered.

use bytes::Bytes;
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
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use std::{cell::RefCell, collections::HashMap, ptr::NonNull, sync::Arc};

/// Serves the resources `lookup` finds, looking each up by the struct tag of
/// its interned type and materializing it on first access. Table items are
/// never served.
pub struct LazyResourceProvider<'guard, 'ctx, L> {
    guard: &'guard ExecutionGuard<'ctx>,
    lookup: L,
    materialized: RefCell<Materialized>,
}

struct Materialized {
    /// Holds decoded values and is never reset. Each storage read pins the
    /// segment its value is in, keeping it alive while the value is in use.
    arena: SegmentedArena,
    cache: HashMap<InMemoryStorageKey, (NonNull<u8>, Arc<dyn ReadPin>)>,
}

impl<'guard, 'ctx, L> LazyResourceProvider<'guard, 'ctx, L>
where
    L: Fn(AccountAddress, StructTag) -> Result<Option<Bytes>, ResourceProviderError>,
{
    /// `lookup` returns a resource's BCS bytes, or `None` if it does not
    /// exist.
    pub fn new(guard: &'guard ExecutionGuard<'ctx>, lookup: L) -> Self {
        Self {
            guard,
            lookup,
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

impl<L> ResourceProvider for LazyResourceProvider<'_, '_, L>
where
    L: Fn(AccountAddress, StructTag) -> Result<Option<Bytes>, ResourceProviderError>,
{
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
                match (self.lookup)(*address, tag)? {
                    Some(blob) => self.materialize(key, *ty, &blob),
                    None => Ok(StorageRead::DoesNotExist { version: None }),
                }
            },
            // TODO(testing): serve table items through the lookup, so the
            // transactional session can keep tables across tasks once it
            // commits table writes.
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
