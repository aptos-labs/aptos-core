// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Block-STM-backed [`ResourceProvider`]: reads go to the shared
//! multi-version map first; on a miss the base value is deserialized
//! (BCS → flat) into its own heap and published into the map at the storage
//! version — exactly how the legacy path caches base values — so every key
//! is deserialized once per block, not once per transaction.
//!
//! Every read hands the interpreter a pointer together with the `Arc` of the
//! heap it points into (inside [`StorageRead::ExternalHeap`]). The read-write
//! set stores the read, so the pointer stays valid for the whole execution
//! even if a concurrent abort removes the map entry that also held the heap.

use crate::value::{to_mono_version, MonoValue};
use aptos_mvhashmap::{
    types::{Incarnation, MVDataError, MVDataOutput, TxnIndex},
    versioned_data::VersionedData,
};
use aptos_types::{
    state_store::{state_key::StateKey, table::TableHandle, TStateView},
    vm::module_metadata::get_metadata,
    write_set::WriteOpKind,
};
use bytes::Bytes;
use mono_move_core::{
    storage::resource_provider::{InMemoryStorageKey, ResourceProviderError},
    types::{view_name, view_type, view_type_list, InternedType, Type},
    FrameOffset, LayoutKind, LayoutProvider, ValueLayout, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap, ResourceProvider, StorageRead};
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use std::{cell::Cell, collections::BTreeMap, ptr::NonNull, sync::Arc};

/// Slack added to a base value's heap beyond the estimated flat size: covers
/// vector headers and rounding (the flat form of `n` BCS bytes is bounded by
/// a small constant factor; mirrors the replay-benchmark heuristic).
const BASE_VALUE_HEAP_SLACK: usize = 4 * 1024;
const HEAP_BYTES_PER_BLOB_BYTE: usize = 8;

/// How reads consult the multi-version map. BlockSTMv1 must not register
/// read dependencies (its estimate mechanism replaces them); BlockSTMv2 must.
#[derive(Clone, Copy)]
pub enum ReadMode {
    BlockStmV1,
    BlockStmV2 { incarnation: Incarnation },
}

/// Serves one transaction's storage reads from Block-STM state.
pub struct BlockStmResourceProvider<'a, 'guard, 'ctx, S> {
    guard: &'guard ExecutionGuard<'ctx>,
    versioned_data: &'a VersionedData<InMemoryStorageKey, MonoValue>,
    base_view: &'a S,
    txn_idx: TxnIndex,
    read_mode: ReadMode,
    /// Set when a read hit a speculative dependency: the incarnation must
    /// abort and re-execute.
    speculative_failure: Cell<bool>,
}

impl<'a, 'guard, 'ctx, S> BlockStmResourceProvider<'a, 'guard, 'ctx, S>
where
    S: TStateView<Key = StateKey>,
{
    pub fn new(
        guard: &'guard ExecutionGuard<'ctx>,
        versioned_data: &'a VersionedData<InMemoryStorageKey, MonoValue>,
        base_view: &'a S,
        txn_idx: TxnIndex,
        read_mode: ReadMode,
    ) -> Self {
        Self {
            guard,
            versioned_data,
            base_view,
            txn_idx,
            read_mode,
            speculative_failure: Cell::new(false),
        }
    }

    /// Whether any read hit a speculative dependency. Checked by the executor
    /// after the run: a true value turns the whole execution into a
    /// speculative failure.
    pub fn speculative_failure(&self) -> bool {
        self.speculative_failure.get()
    }

    /// Fetches a key from the multi-version map with the mode-appropriate
    /// read API (BlockSTMv2 registers a read dependency on the entry).
    fn fetch(
        &self,
        key: &InMemoryStorageKey,
    ) -> Result<MVDataOutput<MonoValue>, MVDataError> {
        match self.read_mode {
            ReadMode::BlockStmV1 => self.versioned_data.fetch_data_no_record(key, self.txn_idx),
            ReadMode::BlockStmV2 { incarnation } => self
                .versioned_data
                .fetch_data_and_record_dependency(key, self.txn_idx, incarnation),
        }
    }

    /// Reads a key from the base state view and publishes the materialized
    /// value (or its absence) into the map at the storage version. First
    /// writer wins on a concurrent race; the caller must re-fetch from the
    /// map afterwards — that is what registers the read dependency and
    /// observes a lower transaction's write that may have landed meanwhile.
    fn publish_base_value(&self, key: &InMemoryStorageKey) -> Result<(), ResourceProviderError> {
        // Resources may live either under their own state key or inside a
        // resource-group blob; the group (if any) is named by the struct's
        // module metadata.
        let bytes = match key {
            InMemoryStorageKey::Resource { address, ty } => {
                let tag = struct_tag_for(*ty)?;
                let state_key = StateKey::resource(address, &tag).map_err(|e| {
                    invariant(format!("failed to build a resource state key: {}", e))
                })?;
                match self.base_bytes(&state_key)? {
                    Some(bytes) => Some(bytes),
                    None => self.group_member_bytes(address, &tag)?,
                }
            },
            InMemoryStorageKey::TableItem { handle, key, .. } => {
                self.base_bytes(&StateKey::table_item(&TableHandle(handle.address()), key))?
            },
        };

        let value = match bytes {
            Some(bytes) => {
                let ty = match key {
                    InMemoryStorageKey::Resource { ty, .. } => *ty,
                    InMemoryStorageKey::TableItem { value_ty, .. } => *value_ty,
                };
                let (ptr, heap) = materialize_value(self.guard, ty, &bytes)?;
                MonoValue::Write {
                    ptr,
                    // Base values exist in storage, so a write shadowing them
                    // is a modification.
                    kind: WriteOpKind::Modification,
                    heap: Arc::new(heap),
                }
            },
            // Publish the absence too, so later transactions skip the base
            // view lookup.
            None => MonoValue::Deletion,
        };
        self.versioned_data
            .set_base_value_with(key.clone(), value, |_existing, _incoming| {});
        Ok(())
    }

    fn base_bytes(&self, state_key: &StateKey) -> Result<Option<Bytes>, ResourceProviderError> {
        self.base_view
            .get_state_value_bytes(state_key)
            .map_err(|e| invariant(format!("base state read failed for {:?}: {}", state_key, e)))
    }

    /// Reads a resource that lives inside a resource-group blob. Returns
    /// [`None`] if the struct is not a group member, the group blob does not
    /// exist, or the member is absent from it.
    // TODO(perf): cache the module deserialization / group membership per
    // provider or per block; currently paid on every base miss.
    fn group_member_bytes(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Bytes>, ResourceProviderError> {
        let module_key = StateKey::module(&tag.address, tag.module.as_ident_str());
        let Some(module_bytes) = self.base_bytes(&module_key)? else {
            return Ok(None);
        };
        let module = CompiledModule::deserialize(&module_bytes)
            .map_err(|e| invariant(format!("failed to deserialize a module: {:?}", e)))?;
        let Some(group_tag) = get_metadata(&module.metadata).and_then(|metadata| {
            metadata
                .struct_attributes
                .get(tag.name.as_ident_str().as_str())?
                .iter()
                .find_map(|attr| attr.get_resource_group_member())
        }) else {
            return Ok(None);
        };
        let Some(blob) = self.base_bytes(&StateKey::resource_group(address, &group_tag))? else {
            return Ok(None);
        };
        let mut members: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(&blob)
            .map_err(|e| invariant(format!("failed to decode a resource group blob: {}", e)))?;
        Ok(members.remove(tag).map(Bytes::from))
    }
}

impl<S> ResourceProvider for BlockStmResourceProvider<'_, '_, '_, S>
where
    S: TStateView<Key = StateKey>,
{
    fn get_resource(&self, key: &InMemoryStorageKey) -> Result<StorageRead, ResourceProviderError> {
        let fetch = match self.fetch(key) {
            Ok(output) => Ok(output),
            Err(MVDataError::Uninitialized) => {
                // Mirrors the legacy view: publish the base value, then
                // re-fetch through the map. The re-fetch registers the read
                // dependency (a fetch of a missing key registers nothing, so
                // a concurrent lower write would never invalidate this
                // transaction) and picks up such a write if it won the race.
                self.publish_base_value(key)?;
                self.fetch(key)
            },
            Err(e @ MVDataError::Dependency(_)) => Err(e),
        };
        match fetch {
            Ok(MVDataOutput::Versioned(version, value)) => {
                let version = to_mono_version(version);
                match value {
                    MonoValue::Write { ptr, heap, .. } => {
                        Ok(StorageRead::ExternalHeap { ptr, version, heap })
                    },
                    MonoValue::Deletion => Ok(StorageRead::DoesNotExist { version }),
                }
            },
            Err(MVDataError::Uninitialized) => Err(invariant(
                "a published base value must be readable".to_string(),
            )),
            Err(MVDataError::Dependency(_)) => {
                // Do not wait for the dependency; abort the incarnation and
                // let the scheduler re-execute it.
                self.speculative_failure.set(true);
                Err(invariant(
                    "read observed a speculative dependency; the incarnation must re-execute"
                        .to_string(),
                ))
            },
        }
    }
}

fn invariant(msg: String) -> ResourceProviderError {
    ResourceProviderError::InvariantViolation(msg)
}

/// Reconstructs the [`StructTag`] of an interned resource type.
fn struct_tag_for(ty: InternedType) -> Result<StructTag, ResourceProviderError> {
    match type_tag_for(ty)? {
        TypeTag::Struct(tag) => Ok(*tag),
        tag => Err(invariant(format!(
            "resource key with a non-struct type {:?}",
            tag
        ))),
    }
}

/// Reconstructs the [`TypeTag`] of an interned type by walking the type DAG.
/// Only storable value types (storage keys, event payloads) are supported.
pub(crate) fn type_tag_for(ty: InternedType) -> Result<TypeTag, ResourceProviderError> {
    // SAFETY (view_type / view_type_list / view_name / as_ref_unchecked):
    // the provider only runs during execution, under a live guard that keeps
    // the arenas alive.
    Ok(match view_type(ty) {
        Type::Bool => TypeTag::Bool,
        Type::U8 => TypeTag::U8,
        Type::U16 => TypeTag::U16,
        Type::U32 => TypeTag::U32,
        Type::U64 => TypeTag::U64,
        Type::U128 => TypeTag::U128,
        Type::U256 => TypeTag::U256,
        Type::I8 => TypeTag::I8,
        Type::I16 => TypeTag::I16,
        Type::I32 => TypeTag::I32,
        Type::I64 => TypeTag::I64,
        Type::I128 => TypeTag::I128,
        Type::I256 => TypeTag::I256,
        Type::Address => TypeTag::Address,
        Type::Signer => TypeTag::Signer,
        Type::Vector { elem } => TypeTag::Vector(Box::new(type_tag_for(*elem)?)),
        Type::Nominal {
            module_id,
            name,
            ty_args,
        } => {
            // SAFETY: see above.
            let module_id = unsafe { module_id.as_ref_unchecked() };
            let type_args = view_type_list(*ty_args)
                .iter()
                .map(|arg| type_tag_for(*arg))
                .collect::<Result<Vec<_>, _>>()?;
            TypeTag::Struct(Box::new(StructTag {
                address: *module_id.address(),
                module: identifier(view_name(module_id.name()))?,
                name: identifier(view_name(*name))?,
                type_args,
            }))
        },
        Type::ImmutRef { .. } | Type::MutRef { .. } | Type::Function { .. } => {
            return Err(invariant(
                "reference and function types cannot appear in a storage key".to_string(),
            ))
        },
        Type::TypeParam { .. } => {
            return Err(invariant(
                "unsubstituted type parameter in a storage key".to_string(),
            ))
        },
    })
}

fn identifier(name: &str) -> Result<Identifier, ResourceProviderError> {
    Identifier::new(name).map_err(|e| invariant(format!("invalid identifier {:?}: {}", name, e)))
}

/// Materializes one value of type `ty` from its BCS `blob` into a fresh heap
/// sized for it, returning the pointer to the flat object and the heap.
/// Unlike the replay-benchmark twin, every failure is an error: a missing
/// layout, a full heap or a malformed blob must never read as "the value
/// does not exist".
// TODO(cleanup): share this with replay-benchmark's `materialize_one`.
fn materialize_value(
    guard: &ExecutionGuard,
    ty: InternedType,
    blob: &[u8],
) -> Result<(NonNull<u8>, Heap), ResourceProviderError> {
    let layout = guard
        .layout_by_ty(ty)
        .ok_or_else(|| invariant("no layout published for a read value type".to_string()))?;
    let size = layout.size as usize;

    // The GC descriptor records which payload slots are heap pointers.
    // Lowering already published it for this type; `publish_struct_descriptor`
    // is idempotent and returns that one (our offsets are a fallback that is
    // ignored on the fast path).
    let mut offsets = vec![];
    collect_pointer_offsets(guard, layout, 0, &mut offsets);
    let frame_offsets = offsets.into_iter().map(FrameOffset).collect::<Vec<_>>();
    let descriptor = guard.publish_struct_descriptor(ty, layout.size, &frame_offsets);

    let mut heap = Heap::new(
        OBJECT_HEADER_SIZE + size + blob.len() * HEAP_BYTES_PER_BLOB_BYTE + BASE_VALUE_HEAP_SLACK,
    );
    let obj = heap
        .alloc_object(OBJECT_HEADER_SIZE + size, descriptor)
        .ok_or_else(|| invariant("base value heap exhausted".to_string()))?;
    // SAFETY: `obj` is a freshly reserved object with `size` payload bytes;
    // `deserialize_into` writes the flat value there and boxes any nested
    // vectors in `heap`.
    unsafe { deserialize_into(guard, &mut heap, ty, blob, obj.as_ptr()) }
        .map_err(|e| invariant(format!("failed to deserialize a base value: {}", e)))?;
    Ok((obj, heap))
}

/// Collects the byte offsets (within the payload) of 8-byte heap-pointer
/// slots, matching what MonoMove's lowering computes for an object
/// descriptor.
fn collect_pointer_offsets(
    guard: &ExecutionGuard,
    layout: &ValueLayout,
    base: u32,
    out: &mut Vec<u32>,
) {
    if layout.has_no_pointers_no_padding() {
        return;
    }
    match &layout.kind {
        LayoutKind::Vector { .. }
        | LayoutKind::Function
        | LayoutKind::Ref
        | LayoutKind::FrozenEnum { .. } => out.push(base),
        LayoutKind::Struct { fields } => {
            for field in fields.iter() {
                if let Some(sub) = guard.layout(field.id) {
                    collect_pointer_offsets(guard, sub, base + field.offset, out);
                }
            }
        },
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address => {},
    }
}
