// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Data providers serving MonoMove's module and resource reads from an Aptos
//! `StateView`, for sequential execution: tests, simulation, and replay-style
//! tools. Production execution uses the Block-STM integration's providers.
//!
//! - `StateViewModuleProvider` serves module bytes and package co-membership
//!   to the loader.
//! - `StateViewResourceProvider` serves resource and table-item reads,
//!   materializing each value into a long-lived arena on first access, and
//!   resolves resource-group placement.

use anyhow::{anyhow, Result};
use aptos_framework::natives::code::PackageRegistry;
use aptos_types::state_store::{state_key::StateKey, StateView};
use bytes::Bytes;
use fxhash::FxHashMap;
use mono_move_aptos_transaction_executor::{decode_group_members, AptosDataProvider, GroupMembers};
use mono_move_core::{
    nominal_tag,
    storage::{
        module_provider::ModuleProvider,
        resource_provider::{
            InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
        },
    },
    types::InternedType,
    ExecutionErrorKind, IntoExecutionError, LayoutProvider, VMInternalError, VMResult,
    OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap};
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, move_resource::MoveStructType,
};
use std::cell::RefCell;
use thiserror::Error;
use triomphe::Arc;

/// Default size of the provider's value arena. Occupancy is bounded by one
/// materialization per distinct key read, so this fits any realistic use.
const DEFAULT_RESOURCE_ARENA_BYTES: usize = 64 * 1024 * 1024;

/// Errors raised by these providers.
#[derive(Debug, Error)]
#[error("{0}")]
struct ProviderError(String);

impl IntoExecutionError for ProviderError {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::Placeholder
    }
}

fn provider_error(detail: String) -> VMInternalError {
    VMInternalError::new(ProviderError(detail))
}

/// Serves module bytes to the loader from a `StateView`.
pub struct StateViewModuleProvider<'s, S> {
    state_view: &'s S,
}

impl<'s, S: StateView> StateViewModuleProvider<'s, S> {
    pub fn new(state_view: &'s S) -> Self {
        Self { state_view }
    }
}

impl<S: StateView> ModuleProvider for StateViewModuleProvider<'_, S> {
    fn get_module_bytes(&self, address: &AccountAddress, name: &str) -> VMResult<Option<Bytes>> {
        let name = Identifier::new(name)
            .map_err(|e| provider_error(format!("invalid module name {name:?}: {e}")))?;
        let key = StateKey::module(address, &name);
        Ok(self
            .state_view
            .get_state_value(&key)
            .map_err(|e| provider_error(format!("module read failed: {e}")))?
            .map(|value| value.bytes().clone()))
    }

    fn deserialize_module(&self, bytes: &[u8]) -> VMResult<CompiledModule> {
        // TODO(correctness): use the on-chain deserializer config (max version, etc.).
        CompiledModule::deserialize(bytes)
            .map_err(|e| provider_error(format!("deserialize failed: {e:?}")))
    }

    fn verify_module(&self, module: &CompiledModule) -> VMResult<()> {
        // TODO(correctness): use the on-chain verifier config instead of the default.
        move_bytecode_verifier::verify_module(module)
            .map_err(|e| provider_error(format!("verification failed: {e:?}")))
    }

    // TODO(perf): fetches and BCS-deserializes the address's entire
    // `PackageRegistry` (for 0x1: every framework package) on each call,
    // uncached.
    fn get_same_package_modules(
        &self,
        address: &AccountAddress,
        module_name: &str,
    ) -> VMResult<Vec<Identifier>> {
        let identifier = |name: &str| {
            Identifier::new(name)
                .map_err(|e| provider_error(format!("invalid module name {name:?}: {e}")))
        };
        let key = StateKey::resource(address, &PackageRegistry::struct_tag())
            .map_err(|e| provider_error(format!("bad state key: {e}")))?;
        let Some(value) = self
            .state_view
            .get_state_value(&key)
            .map_err(|e| provider_error(format!("package registry read failed: {e}")))?
        else {
            return Err(provider_error(format!("no package registry at {address}")));
        };
        let registry: PackageRegistry = bcs::from_bytes(value.bytes())
            .map_err(|e| provider_error(format!("malformed package registry: {e}")))?;
        for package in &registry.packages {
            let names = package
                .modules
                .iter()
                .map(|m| identifier(&m.name))
                .collect::<VMResult<Vec<_>>>()?;
            if names.iter().any(|n| n.as_str() == module_name) {
                return Ok(names);
            }
        }
        Err(provider_error(format!(
            "module {address}::{module_name} not found in any package"
        )))
    }
}

/// Serves resource and table-item reads from a `StateView`, materializing each
/// value into a long-lived arena on first access. Also resolves and remembers
/// resource-group membership.
pub struct StateViewResourceProvider<'a, 'ctx, S> {
    guard: &'a ExecutionGuard<'ctx>,
    state_view: &'a S,
    inner: RefCell<ProviderState>,
}

/// The provider's interior-mutable state: caches and the value arena.
struct ProviderState {
    /// Bump arena holding the flat values that reads hand out pointers into.
    /// Never collected or reset; occupancy is bounded by one materialization
    /// per distinct key, thanks to the value cache.
    arena: Heap,
    /// Materialized reads, including negative ones. Sound because execution
    /// never mutates a value through a `StorageRead` pointer (mutation copies
    /// into the transaction's own heap first).
    ///
    /// Keyed by the pair of storage key and group placement. A resource's
    /// placement (own slot vs. a group container) is fixed by the reading
    /// transaction's pinned module, not by storage, so the same key can resolve
    /// to a different slot across executions that reuse this provider. Keying by
    /// the pair keeps a `#[resource_group_member]` add or remove between
    /// executions from serving the other placement's value.
    values: FxHashMap<(InMemoryStorageKey, Option<InternedType>), StorageRead>,
    /// Members of each resource group read so far, keyed by the group's state key.
    ///
    /// Here it is safe to use a non-cryptographic hasher because `StateKey` already
    /// gets hashed by its crypto digest.
    groups: FxHashMap<StateKey, Option<Arc<GroupMembers>>>,
}

impl<'a, 'ctx, S: StateView> StateViewResourceProvider<'a, 'ctx, S> {
    pub fn new(guard: &'a ExecutionGuard<'ctx>, state_view: &'a S) -> Self {
        Self::new_with_arena_size(guard, state_view, DEFAULT_RESOURCE_ARENA_BYTES)
    }

    /// Escape hatch for value arenas beyond the default's reach.
    pub fn new_with_arena_size(
        guard: &'a ExecutionGuard<'ctx>,
        state_view: &'a S,
        arena_size: usize,
    ) -> Self {
        Self {
            guard,
            state_view,
            inner: RefCell::new(ProviderState {
                arena: Heap::new(arena_size),
                values: FxHashMap::default(),
                groups: FxHashMap::default(),
            }),
        }
    }

    /// The stored BCS bytes for a key, looked up among the group's members
    /// for group-member keys. `None` = does not exist.
    fn fetch_bytes(
        &self,
        key: &InMemoryStorageKey,
        group: Option<InternedType>,
    ) -> Result<Option<Bytes>> {
        match group {
            None => {
                let state_key = key.as_state_key()?;
                Ok(self
                    .state_view
                    .get_state_value(&state_key)
                    .map_err(|e| anyhow!("state read failed: {e}"))?
                    .map(|value| value.bytes().clone()))
            },
            Some(group_ty) => {
                let tag = nominal_tag(group_ty)?;
                let group_key = StateKey::resource_group(&key.address(), &tag);
                Ok(self
                    .group_members(&group_key)?
                    .and_then(|members| members.get(&key.value_ty()).cloned()))
            },
        }
    }
}

impl<S: StateView> ResourceProvider for StateViewResourceProvider<'_, '_, S> {
    /// Cached per (key, group placement), so re-reads across executions reuse
    /// the already materialized value only when the placement also matches.
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        let cache_key = (key.clone(), group);
        if let Some(read) = self.inner.borrow().values.get(&cache_key) {
            return Ok(*read);
        }
        let internal = |detail: String| ResourceProviderError::InvariantViolation(detail);
        let Some(blob) = self
            .fetch_bytes(key, group)
            .map_err(|e| internal(e.to_string()))?
        else {
            self.inner
                .borrow_mut()
                .values
                .insert(cache_key, StorageRead::DoesNotExist);
            return Ok(StorageRead::DoesNotExist);
        };

        // Materialize the value (BCS → flat) into the provider's arena.
        let ty = key.value_ty();
        let layout = self
            .guard
            .layout_by_ty(ty)
            .ok_or_else(|| internal(format!("no layout for the value at {:?}", key.address())))?;
        // Lowering publishes a descriptor for every resource a global-storage
        // instruction reaches, so the read that got here has one already.
        let descriptor = self
            .guard
            .struct_descriptor(ty)
            .ok_or_else(|| internal(format!("no GC descriptor for {:?}", key.address())))?;

        let mut inner = self.inner.borrow_mut();
        let obj = inner
            .arena
            .alloc_object(OBJECT_HEADER_SIZE + layout.size as usize, descriptor)
            .ok_or_else(|| internal("resource arena is full".to_string()))?;
        // SAFETY: `obj` is a freshly reserved object sized for the value's
        // layout; `deserialize_into` writes the flat value there.
        unsafe { deserialize_into(self.guard, &mut inner.arena, ty, &blob, obj.as_ptr()) }
            .map_err(|e| internal(format!("stored value failed to deserialize: {e}")))?;
        let read = StorageRead::ExternalHeap {
            ptr: obj,
            version: 0,
        };
        inner.values.insert(cache_key, read);
        Ok(read)
    }
}

impl<S: StateView> AptosDataProvider for StateViewResourceProvider<'_, '_, S> {
    /// Loaded from the state view on first access, caching absence as well so a
    /// missing group is read at most once.
    fn group_members(&self, group_key: &StateKey) -> Result<Option<Arc<GroupMembers>>> {
        if let Some(members) = self.inner.borrow().groups.get(group_key) {
            return Ok(members.clone());
        }
        let members = match self
            .state_view
            .get_state_value(group_key)
            .map_err(|e| anyhow!("group read failed: {e}"))?
        {
            Some(value) => Some(Arc::new(decode_group_members(value.bytes(), self.guard)?)),
            None => None,
        };
        self.inner
            .borrow_mut()
            .groups
            .insert(group_key.clone(), members.clone());
        Ok(members)
    }
}
