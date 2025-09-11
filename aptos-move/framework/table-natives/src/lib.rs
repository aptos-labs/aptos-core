// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! A crate which extends Move by tables.
//!
//! See [`Table.move`](../sources/Table.move) for language use.
//! See [`README.md`](../README.md) for integration into an adapter.

use aptos_gas_meter::AptosGasMeter;
use aptos_gas_schedule::gas_params::natives::table::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_value::StateValueMetadata,
        table::TableHandle as AptosTableHandle,
    },
    vm::versioning::{CurrentVersion, VersionController, VersionedSlot, VersionedSlotDerivedData},
    write_set::WriteOpSize,
};
use aptos_vm_types::{
    abstract_write_op::{AbstractResourceWriteOp, InPlaceDelayedFieldChangeOp},
    change_set::WriteOpInfo,
    storage::change_set_configs::ChangeSetSizeAndRefundTracker,
    write_info_builder::WriteOpInfoBuilder,
};
use better_any::{Tid, TidAble};
use bytes::Bytes;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress, effects::Op, gas_algebra::NumBytes, identifier::Identifier,
    value::MoveTypeLayout, vm_status::StatusCode,
};
pub use move_table_extension::{TableHandle, TableInfo, TableResolver};
use move_vm_runtime::{
    native_extensions::NativeExtensionSession,
    native_functions::{LoaderContext, NativeFunctionTable},
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::{GlobalValue, Reference, StructRef, Value},
};
use sha3::{Digest, Sha3_256};
use smallvec::{smallvec, SmallVec};
use std::{
    cell::RefCell,
    collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque},
    mem::drop,
};
use triomphe::Arc as TriompheArc;

/// The native table context extension. This needs to be attached to the NativeContextExtensions
/// value which is passed into session functions, so its accessible from natives of this
/// extension.
#[derive(Tid)]
pub struct NativeTableContext<'a> {
    resolver: &'a dyn TableResolver,
    txn_hash: [u8; 32],
    table_data: RefCell<TableData>,
}

// See stdlib/Error.move
const _ECATEGORY_INVALID_STATE: u8 = 0;
const ECATEGORY_INVALID_ARGUMENT: u8 = 7;

const ALREADY_EXISTS: u64 = (100 << 8) + ECATEGORY_INVALID_ARGUMENT as u64;
const NOT_FOUND: u64 = (101 << 8) + ECATEGORY_INVALID_ARGUMENT as u64;
// Move side raises this
const _NOT_EMPTY: u64 = (102 << 8) + _ECATEGORY_INVALID_STATE as u64;

// ===========================================================================================
// Private Data Structures and Constants

/// A structure representing mutable data of the NativeTableContext. This is in a RefCell
/// of the overall context so we can mutate while still accessing the overall context.
#[derive(Default)]
struct TableData {
    vc: VersionController,
    tables: BTreeMap<TableHandle, Table>,
    new_tables_counter: u32,
    // Below structures only support legacy, non-continuous session.
    legacy_new_tables: BTreeMap<TableHandle, TableInfo>,
    legacy_removed_tables: BTreeSet<TableHandle>,
}

/// A structure containing information about the layout of a value stored in a table. Needed in
/// order to replace delayed fields.
struct LayoutInfo {
    layout: TriompheArc<MoveTypeLayout>,
    contains_delayed_fields: bool,
}

/// An item in a table. For continuous session: a versioned global value. For legacy session model
/// a single base version is kept at all times as the context is never saved or rolled back.
struct TableItem {
    /// Different version of the global storage slot.
    gv: VersionedSlot<GlobalValue>,
    /// Previous size (before any transaction side effects), in bytes.
    prev_size: u64,
    /// Metadata and sizes corresponding to different versions. Computed exactly once.
    metadata_and_size: VersionedSlotDerivedData<(StateValueMetadata, WriteOpSize)>,
}

/// A structure representing a single table.
struct Table {
    handle: TableHandle,
    key_layout: MoveTypeLayout,
    value_layout_info: LayoutInfo,
    content: BTreeMap<Vec<u8>, TableItem>,
}

/// The field index of the `handle` field in the `Table` Move struct.
const HANDLE_FIELD_INDEX: usize = 0;

/// A table change set.
#[derive(Default)]
pub struct TableChangeSet {
    pub new_tables: BTreeMap<TableHandle, TableInfo>,
    pub removed_tables: BTreeSet<TableHandle>,
    pub changes: BTreeMap<TableHandle, TableChange>,
}

/// A change of a single table.
pub struct TableChange {
    pub entries: BTreeMap<Vec<u8>, Op<(Bytes, Option<TriompheArc<MoveTypeLayout>>)>>,
}

// =========================================================================================
// Implementation of Native Table Context

impl<'a> NativeExtensionSession for NativeTableContext<'a> {
    fn abort(&mut self) {
        self.table_data.borrow_mut().vc.undo();
    }

    fn finish(&mut self) {
        self.table_data.borrow_mut().vc.save();
    }

    fn start(&mut self, txn_hash: &[u8; 32], _script_hash: &[u8], _session_counter: u8) {
        self.txn_hash = *txn_hash;

        let mut table_data = self.table_data.borrow_mut();
        table_data.new_tables_counter = 0;
        assert!(table_data.legacy_new_tables.is_empty());
        assert!(table_data.legacy_removed_tables.is_empty());
        // Note: tables are persisted on reset for continuous session.
    }
}

impl<'a> NativeTableContext<'a> {
    /// Create a new instance of a native table context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new(txn_hash: [u8; 32], resolver: &'a dyn TableResolver) -> Self {
        Self {
            resolver,
            txn_hash,
            table_data: Default::default(),
        }
    }

    /// Computes the change set from a NativeTableContext. Used for legacy non-continuous session
    /// only.
    pub fn legacy_into_change_set(
        self,
        function_value_extension: &impl FunctionValueExtension,
    ) -> PartialVMResult<TableChangeSet> {
        let NativeTableContext { table_data, .. } = self;
        let TableData {
            vc,
            legacy_new_tables: new_tables,
            legacy_removed_tables: removed_tables,
            tables,
            ..
        } = table_data.into_inner();

        let version = vc.current_version();
        let mut changes = BTreeMap::new();
        for (handle, table) in tables {
            let Table {
                value_layout_info,
                content,
                ..
            } = table;
            let mut entries = BTreeMap::new();
            for (key, mut table_item) in content {
                let op = match table_item
                    .gv
                    .take(version)
                    .expect("Global value always exist in non-continuous session")
                    .into_effect()
                {
                    Some(op) => op,
                    None => continue,
                };

                match op {
                    Op::New(val) => {
                        entries.insert(
                            key,
                            Op::New(serialize_value(
                                function_value_extension,
                                &value_layout_info,
                                &val,
                            )?),
                        );
                    },
                    Op::Modify(val) => {
                        entries.insert(
                            key,
                            Op::Modify(serialize_value(
                                function_value_extension,
                                &value_layout_info,
                                &val,
                            )?),
                        );
                    },
                    Op::Delete => {
                        entries.insert(key, Op::Delete);
                    },
                }
            }
            if !entries.is_empty() {
                changes.insert(handle, TableChange { entries });
            }
        }
        Ok(TableChangeSet {
            new_tables,
            removed_tables,
            changes,
        })
    }

    /// Extracts latest version of writes from the extension. For writes for which the metadata was
    /// not yet computed, recomputes it (not allowing any creations).
    ///
    /// Note: used only for continuous session.
    pub fn take_write_ops(
        &self,
        write_op_info_builder: &impl WriteOpInfoBuilder,
    ) -> PartialVMResult<BTreeMap<StateKey, AbstractResourceWriteOp>> {
        let mut changes = BTreeMap::new();
        let mut table_data = self.table_data.borrow_mut();

        let current_version = table_data.vc.current_version();
        for (handle, table) in table_data.tables.iter_mut() {
            for (key, table_item) in table.content.iter_mut() {
                let state_key = StateKey::table_item(&AptosTableHandle(handle.0), key);
                let maybe_computed = table_item.metadata_and_size.take(current_version);
                let (metadata, op_size) = match maybe_computed {
                    Some(res) => res,
                    None => {
                        if !table_item.compute_metadata_and_size_once(
                            write_op_info_builder,
                            &state_key,
                            &table.value_layout_info,
                            current_version,
                            true,
                        )? {
                            continue;
                        }

                        table_item
                            .metadata_and_size
                            .take(current_version)
                            .ok_or_else(|| {
                                PartialVMError::new_invariant_violation(
                                    "Metadata and size must be computed",
                                )
                            })?
                    },
                };

                let layout = table.value_layout_info.layout.clone();
                let write_op = match table_item
                    .gv
                    .get(current_version)
                    .expect("No-ops are already skipped during metadata computation")
                    .effect()
                {
                    Some(op) => write_op_info_builder.get_resource_write_op(
                        op,
                        layout,
                        table.value_layout_info.contains_delayed_fields,
                        metadata,
                    )?,
                    None => {
                        // This means there is a delayed field change (otherwise, we should have
                        // continued the loop early when computing metadata).
                        let materialized_size = match op_size {
                            WriteOpSize::Modification { write_len } => write_len,
                            WriteOpSize::Creation { .. } | WriteOpSize::Deletion => {
                                return Err(PartialVMError::new_invariant_violation(
                                    "In-place delayed field changes are always modifications",
                                ));
                            },
                        };
                        AbstractResourceWriteOp::InPlaceDelayedFieldChange(
                            InPlaceDelayedFieldChangeOp {
                                layout,
                                materialized_size,
                                metadata,
                            },
                        )
                    },
                };

                changes.insert(state_key, write_op);
            }
        }

        Ok(changes)
    }

    /// Charges gas for latest version of writes from the extension, computing their metadata and
    /// refund.
    ///
    /// Note: used only for continuous session.
    pub fn charge_write_ops(
        &self,
        write_op_info_builder: &impl WriteOpInfoBuilder,
        change_set_size_tracker: &mut ChangeSetSizeAndRefundTracker,
        gas_meter: &mut impl AptosGasMeter,
    ) -> VMResult<()> {
        let mut table_data = self.table_data.borrow_mut();

        let current_version = table_data.vc.current_version();
        for (handle, table) in table_data.tables.iter_mut() {
            for (key, table_item) in table.content.iter_mut() {
                let state_key = StateKey::table_item(&AptosTableHandle(handle.0), key);
                if !table_item
                    .compute_metadata_and_size_once(
                        write_op_info_builder,
                        &state_key,
                        &table.value_layout_info,
                        current_version,
                        false,
                    )
                    .map_err(|err| err.finish(Location::Undefined))?
                {
                    continue;
                }

                let (metadata_mut, size) = table_item
                    .metadata_and_size
                    .get(current_version)
                    .ok_or_else(|| {
                        PartialVMError::new_invariant_violation(
                            "Metadata and size must be computed",
                        )
                        .finish(Location::Undefined)
                    })?;

                let op_info = WriteOpInfo {
                    key: &state_key,
                    op_size: *size,
                    prev_size: table_item.prev_size,
                    metadata_mut,
                };

                change_set_size_tracker.count_write_op(&state_key, *size)?;
                change_set_size_tracker.record_storage_fee_and_refund_write_op(op_info)?;
                gas_meter.charge_io_gas_for_write(&state_key, size)?;
            }
        }
        Ok(())
    }
}

impl TableData {
    /// Gets or creates a new table in the TableData. This initializes information about
    /// the table, like the type layout for keys and values.
    fn get_or_create_table(
        &mut self,
        loader_context: &mut LoaderContext,
        handle: TableHandle,
        key_ty: &Type,
        value_ty: &Type,
    ) -> PartialVMResult<&mut Table> {
        Ok(match self.tables.entry(handle) {
            Entry::Vacant(e) => {
                let key_layout = loader_context
                    .type_to_type_layout_with_delayed_fields(key_ty)?
                    .unpack()
                    .0;
                let value_layout_info = LayoutInfo::from_value_ty(loader_context, value_ty)?;
                let table = Table {
                    handle,
                    key_layout,
                    value_layout_info,
                    content: Default::default(),
                };
                e.insert(table)
            },
            Entry::Occupied(e) => e.into_mut(),
        })
    }
}

impl LayoutInfo {
    fn from_value_ty(loader_context: &mut LoaderContext, value_ty: &Type) -> PartialVMResult<Self> {
        let (layout, contains_delayed_fields) = loader_context
            .type_to_type_layout_with_delayed_fields(value_ty)?
            .unpack();
        Ok(Self {
            layout: TriompheArc::new(layout),
            contains_delayed_fields,
        })
    }

    fn layout_ref_if_contains_delayed_fields(&self) -> Option<&MoveTypeLayout> {
        self.contains_delayed_fields.then(|| self.layout.as_ref())
    }
}

impl Table {
    /// If the entry corresponding to te key is vacant, or occupied but without any versions,
    /// returns true.
    fn table_item_needs_initialization(&mut self, key: &[u8], version: CurrentVersion) -> bool {
        self.content
            .get_mut(key)
            .map(|table_item| table_item.gv.check_empty(version))
            .unwrap_or(true)
    }

    /// Initializes the entry to the global value fetched from the on-chain, and returns a mutable
    /// reference to the value as well as number of loaded bytes.
    ///
    /// Even though a mutable reference is returned, there is never any copy-on-write as the value
    /// has just been inserted under the current working version.
    fn initialize_table_item(
        &mut self,
        function_value_extension: &dyn FunctionValueExtension,
        resolver: &dyn TableResolver,
        key: &[u8],
        version: CurrentVersion,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)> {
        // If there are delayed fields, we need to pass layout when resolving bytes from
        // storage.
        let layout_if_contains_delayed_fields = self
            .value_layout_info
            .layout_ref_if_contains_delayed_fields();
        let data = resolver.resolve_table_entry_bytes_with_layout(
            &self.handle,
            key,
            layout_if_contains_delayed_fields,
        )?;
        let prev_size = data.as_ref().map(|bytes| bytes.len() as u64).unwrap_or(0);

        let (gv, bytes_loaded) = match data {
            Some(val_bytes) => {
                let val = deserialize_value(
                    function_value_extension,
                    &val_bytes,
                    &self.value_layout_info,
                )?;
                (
                    GlobalValue::cached(val)?,
                    Some(NumBytes::new(val_bytes.len() as u64)),
                )
            },
            None => (GlobalValue::none(), None),
        };

        let gv = self
            .content
            .entry(key.to_vec())
            .or_insert_with(|| TableItem::new(prev_size))
            .gv
            .set_empty(gv, version);
        Ok((gv, bytes_loaded))
    }

    /// Returns an immutable reference to the global value.
    fn get_or_create_table_item(
        &mut self,
        function_value_extension: &dyn FunctionValueExtension,
        resolver: &dyn TableResolver,
        key: Vec<u8>,
        version: CurrentVersion,
    ) -> PartialVMResult<(&GlobalValue, Option<Option<NumBytes>>)> {
        let needs_initialization = self.table_item_needs_initialization(&key, version);
        if needs_initialization {
            let (gv, bytes_loaded) =
                self.initialize_table_item(function_value_extension, resolver, &key, version)?;
            Ok((gv, Some(bytes_loaded)))
        } else {
            let gv = self
                .content
                .get_mut(&key)
                .expect("Table item exists")
                .gv
                .get(version)
                .expect("Global value slot contains a value");
            Ok((gv, None))
        }
    }

    /// Returns a mutable reference to the global value. Global value is copied into the next
    /// version if its current version was previously saved.
    fn get_or_create_table_item_mut(
        &mut self,
        function_value_extension: &dyn FunctionValueExtension,
        resolver: &dyn TableResolver,
        key: Vec<u8>,
        version: CurrentVersion,
    ) -> PartialVMResult<(&mut GlobalValue, Option<Option<NumBytes>>)> {
        let needs_initialization = self.table_item_needs_initialization(&key, version);
        if needs_initialization {
            let (gv, bytes_loaded) =
                self.initialize_table_item(function_value_extension, resolver, &key, version)?;
            Ok((gv, Some(bytes_loaded)))
        } else {
            let gv = self
                .content
                .get_mut(&key)
                .expect("Table item exists")
                .gv
                .get_mut(version)?
                .expect("Global value slot contains a value");
            Ok((gv, None))
        }
    }
}

impl TableItem {
    fn new(prev_size: u64) -> Self {
        Self {
            prev_size,
            gv: VersionedSlot::default(),
            metadata_and_size: VersionedSlotDerivedData::default(),
        }
    }

    /// Computes metadata and size for the value (with version being at most the current version)
    /// and sets them at the current version.
    ///
    ///   - Returns true if the metadata has been computed.
    ///   - Returns false if there was no need to compute it (i.e., no global state changes).
    ///   - An error is returned on double-inserting metadata.
    fn compute_metadata_and_size_once(
        &mut self,
        write_op_info_builder: &impl WriteOpInfoBuilder,
        key: &StateKey,
        layout_info: &LayoutInfo,
        version: CurrentVersion,
        assert_no_creation: bool,
    ) -> PartialVMResult<bool> {
        let gv = match self.gv.get(version) {
            Some(gv) => gv,
            None => {
                // No-op.
                return Ok(false);
            },
        };

        let op = match gv.effect() {
            Some(op) => op,
            None => {
                // If there are any delayed fields in the layout, it is possible that this read is
                // a modification. We check this is the case, and if so, also store metadata and
                // size.
                if layout_info.contains_delayed_fields {
                    if let Some(res) = write_op_info_builder
                        .get_resource_metadata_and_size_for_read_with_delayed_fields(key)?
                    {
                        self.metadata_and_size.set_once(version, res)?;
                        return Ok(true);
                    }
                }
                return Ok(false);
            },
        };

        // Otherwise, this is a change to the global state - get metadata and size and set them.
        let res = write_op_info_builder.get_resource_metadata_and_size(
            key,
            op,
            layout_info.layout.as_ref(),
            layout_info.contains_delayed_fields,
            assert_no_creation,
        )?;
        self.metadata_and_size.set_once(version, res)?;
        Ok(true)
    }
}

// =========================================================================================
// Native Function Implementations

/// Returns all natives for tables.
pub fn table_natives(
    table_addr: AccountAddress,
    builder: &mut SafeNativeBuilder,
) -> NativeFunctionTable {
    builder.with_incremental_gas_charging(false, |builder| {
        builder
            .make_named_natives([
                ("new_table_handle", native_new_table_handle as RawSafeNative),
                ("add_box", native_add_box),
                ("borrow_box", native_borrow_box),
                ("borrow_box_mut", native_borrow_box_mut),
                ("remove_box", native_remove_box),
                ("contains_box", native_contains_box),
                ("destroy_empty_box", native_destroy_empty_box),
                ("drop_unchecked_box", native_drop_unchecked_box),
            ])
            .map(|(func_name, func)| {
                (
                    table_addr,
                    Identifier::new("table").unwrap(),
                    Identifier::new(func_name).unwrap(),
                    func,
                )
            })
            .collect()
    })
}

fn charge_load_cost(
    context: &mut SafeNativeContext,
    loaded: Option<Option<NumBytes>>,
) -> SafeNativeResult<()> {
    context.charge(COMMON_LOAD_BASE_LEGACY)?;

    match loaded {
        Some(Some(num_bytes)) => {
            let num_bytes = if context.gas_feature_version() >= 12 {
                // Round up bytes to whole pages
                // TODO(gas): make PAGE_SIZE configurable
                const PAGE_SIZE: u64 = 4096;

                let loaded_u64: u64 = num_bytes.into();
                let r = loaded_u64 % PAGE_SIZE;
                let rounded_up = loaded_u64 + if r == 0 { 0 } else { PAGE_SIZE - r };

                NumBytes::new(rounded_up)
            } else {
                num_bytes
            };
            context.charge(COMMON_LOAD_BASE_NEW + COMMON_LOAD_PER_BYTE * num_bytes)
        },
        Some(None) => context.charge(COMMON_LOAD_BASE_NEW + COMMON_LOAD_FAILURE),
        None => Ok(()),
    }
}

fn native_new_table_handle(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 2);
    assert!(args.is_empty());

    context.charge(NEW_TABLE_HANDLE_BASE)?;

    let is_legacy_session_used = !context.get_feature_flags().is_aptos_vm_v2_enabled();
    let table_context = context.extensions().get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    // Take the transaction hash provided by the environment, combine it with the # of tables
    // produced so far, sha256 this to produce a unique handle. Given the txn hash
    // is unique, this should create a unique and deterministic global id.
    let mut digest = Sha3_256::new();
    Digest::update(&mut digest, table_context.txn_hash);
    Digest::update(&mut digest, table_data.new_tables_counter.to_be_bytes());
    let bytes = digest.finalize().to_vec();
    let handle = AccountAddress::from_bytes(&bytes[0..AccountAddress::LENGTH])
        .map_err(|_| partial_extension_error("Unable to create table handle"))?;
    let key_type = context.type_to_type_tag(&ty_args[0])?;
    let value_type = context.type_to_type_tag(&ty_args[1])?;
    if is_legacy_session_used {
        assert!(table_data
            .legacy_new_tables
            .insert(TableHandle(handle), TableInfo::new(key_type, value_type))
            .is_none());
    }
    table_data.new_tables_counter += 1;

    Ok(smallvec![Value::address(handle)])
}

fn native_add_box(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 3);
    assert_eq!(args.len(), 3);

    context.charge(ADD_BOX_BASE)?;

    let (extensions, mut loader_context, abs_val_gas_params, gas_feature_version) =
        context.extensions_with_loader_context_and_gas_params();
    let table_context = extensions.get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();
    let current_version = table_data.vc.current_version();

    let val = args.pop_back().unwrap();
    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table =
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    let function_value_extension = loader_context.function_value_extension();
    let key_bytes = serialize_key(&function_value_extension, &table.key_layout, &key)?;
    let key_cost = ADD_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) = table.get_or_create_table_item_mut(
        &function_value_extension,
        table_context.resolver,
        key_bytes,
        current_version,
    )?;
    let mem_usage = gv
        .view()
        .map(|val| {
            abs_val_gas_params
                .abstract_heap_size(&val, gas_feature_version)
                .map(u64::from)
        })
        .transpose()?;

    let res = match gv.move_to(val) {
        Ok(_) => Ok(smallvec![]),
        Err(_) => Err(SafeNativeError::Abort {
            abort_code: ALREADY_EXISTS,
        }),
    };

    drop(table_data);

    // TODO(Gas): Figure out a way to charge this earlier.
    context.charge(key_cost)?;
    if let Some(amount) = mem_usage {
        context.use_heap_memory(amount)?;
    }
    charge_load_cost(context, loaded)?;

    res
}

fn native_borrow_box(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    native_borrow_box_impl::<false>(context, ty_args, args)
}

fn native_borrow_box_mut(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    native_borrow_box_impl::<true>(context, ty_args, args)
}

fn native_borrow_box_impl<const IS_MUT: bool>(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 3);
    assert_eq!(args.len(), 2);

    context.charge(BORROW_BOX_BASE)?;

    let (extensions, mut loader_context, abs_val_gas_params, gas_feature_version) =
        context.extensions_with_loader_context_and_gas_params();
    let table_context = extensions.get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();
    let current_version = table_data.vc.current_version();

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table =
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    let function_value_extension = loader_context.function_value_extension();
    let key_bytes = serialize_key(&function_value_extension, &table.key_layout, &key)?;
    let key_cost = BORROW_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded): (&GlobalValue, _) = if IS_MUT {
        let (gv, loaded) = table.get_or_create_table_item_mut(
            &function_value_extension,
            table_context.resolver,
            key_bytes,
            current_version,
        )?;
        (gv, loaded)
    } else {
        table.get_or_create_table_item(
            &function_value_extension,
            table_context.resolver,
            key_bytes,
            current_version,
        )?
    };
    let mem_usage = gv
        .view()
        .map(|val| {
            abs_val_gas_params
                .abstract_heap_size(&val, gas_feature_version)
                .map(u64::from)
        })
        .transpose()?;

    let res = match gv.borrow_global() {
        Ok(ref_val) => Ok(smallvec![ref_val]),
        Err(_) => Err(SafeNativeError::Abort {
            abort_code: NOT_FOUND,
        }),
    };

    drop(table_data);

    // TODO(Gas): Figure out a way to charge this earlier.
    context.charge(key_cost)?;
    if let Some(amount) = mem_usage {
        context.use_heap_memory(amount)?;
    }
    charge_load_cost(context, loaded)?;

    res
}

fn native_contains_box(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 3);
    assert_eq!(args.len(), 2);

    context.charge(CONTAINS_BOX_BASE)?;

    let (extensions, mut loader_context, abs_val_gas_params, gas_feature_version) =
        context.extensions_with_loader_context_and_gas_params();
    let table_context = extensions.get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();
    let current_version = table_data.vc.current_version();

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table =
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    let function_value_extension = loader_context.function_value_extension();
    let key_bytes = serialize_key(&function_value_extension, &table.key_layout, &key)?;
    let key_cost = CONTAINS_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) = table.get_or_create_table_item(
        &function_value_extension,
        table_context.resolver,
        key_bytes,
        current_version,
    )?;
    let mem_usage = gv
        .view()
        .map(|val| {
            abs_val_gas_params
                .abstract_heap_size(&val, gas_feature_version)
                .map(u64::from)
        })
        .transpose()?;
    let exists = Value::bool(gv.exists());

    drop(table_data);

    // TODO(Gas): Figure out a way to charge this earlier.
    context.charge(key_cost)?;
    if let Some(amount) = mem_usage {
        context.use_heap_memory(amount)?;
    }
    charge_load_cost(context, loaded)?;

    Ok(smallvec![exists])
}

fn native_remove_box(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 3);
    assert_eq!(args.len(), 2);

    context.charge(REMOVE_BOX_BASE)?;

    let (extensions, mut loader_context, abs_val_gas_params, gas_feature_version) =
        context.extensions_with_loader_context_and_gas_params();
    let table_context = extensions.get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();
    let current_version = table_data.vc.current_version();

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table =
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    let function_value_extension = loader_context.function_value_extension();
    let key_bytes = serialize_key(&function_value_extension, &table.key_layout, &key)?;
    let key_cost = REMOVE_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) = table.get_or_create_table_item_mut(
        &function_value_extension,
        table_context.resolver,
        key_bytes,
        current_version,
    )?;
    let mem_usage = gv
        .view()
        .map(|val| {
            abs_val_gas_params
                .abstract_heap_size(&val, gas_feature_version)
                .map(u64::from)
        })
        .transpose()?;

    let res = match gv.move_from() {
        Ok(val) => Ok(smallvec![val]),
        Err(_) => Err(SafeNativeError::Abort {
            abort_code: NOT_FOUND,
        }),
    };

    drop(table_data);

    // TODO(Gas): Figure out a way to charge this earlier.
    context.charge(key_cost)?;
    if let Some(amount) = mem_usage {
        context.use_heap_memory(amount)?;
    }
    charge_load_cost(context, loaded)?;

    res
}

fn native_destroy_empty_box(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 3);
    assert_eq!(args.len(), 1);

    context.charge(DESTROY_EMPTY_BOX_BASE)?;

    let is_legacy_session_used = !context.get_feature_flags().is_aptos_vm_v2_enabled();
    let (extensions, mut loader_context) = context.extensions_with_loader_context();
    let table_context = extensions.get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;
    // TODO: Can the following line be removed?
    table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    if is_legacy_session_used {
        assert!(table_data.legacy_removed_tables.insert(handle));
    }

    Ok(smallvec![])
}

fn native_drop_unchecked_box(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 3);
    assert_eq!(args.len(), 1);

    context.charge(DROP_UNCHECKED_BOX_BASE)?;

    Ok(smallvec![])
}

// =========================================================================================
// Helpers

fn get_table_handle(table: &StructRef) -> PartialVMResult<TableHandle> {
    let handle = table
        .borrow_field(HANDLE_FIELD_INDEX)?
        .value_as::<Reference>()?
        .read_ref()?
        .value_as::<AccountAddress>()?;
    Ok(TableHandle(handle))
}

fn serialize_key(
    function_value_extension: &dyn FunctionValueExtension,
    layout: &MoveTypeLayout,
    key: &Value,
) -> PartialVMResult<Vec<u8>> {
    ValueSerDeContext::new(function_value_extension.max_value_nest_depth())
        .with_func_args_deserialization(function_value_extension)
        .serialize(key, layout)?
        .ok_or_else(|| partial_extension_error("cannot serialize table key"))
}

fn serialize_value(
    function_value_extension: &dyn FunctionValueExtension,
    layout_info: &LayoutInfo,
    val: &Value,
) -> PartialVMResult<(Bytes, Option<TriompheArc<MoveTypeLayout>>)> {
    let max_value_nest_depth = function_value_extension.max_value_nest_depth();
    let serialization_result = if layout_info.contains_delayed_fields {
        // Value contains delayed fields, so we should be able to serialize it.
        ValueSerDeContext::new(max_value_nest_depth)
            .with_delayed_fields_serde()
            .with_func_args_deserialization(function_value_extension)
            .serialize(val, layout_info.layout.as_ref())?
            .map(|bytes| (bytes.into(), Some(layout_info.layout.clone())))
    } else {
        // No delayed fields, make sure serialization fails if there are any
        // native values.
        ValueSerDeContext::new(max_value_nest_depth)
            .with_func_args_deserialization(function_value_extension)
            .serialize(val, layout_info.layout.as_ref())?
            .map(|bytes| (bytes.into(), None))
    };
    serialization_result.ok_or_else(|| partial_extension_error("cannot serialize table value"))
}

fn deserialize_value(
    function_value_extension: &dyn FunctionValueExtension,
    bytes: &[u8],
    layout_info: &LayoutInfo,
) -> PartialVMResult<Value> {
    let layout = layout_info.layout.as_ref();
    let deserialization_result = if layout_info.contains_delayed_fields {
        ValueSerDeContext::new(function_value_extension.max_value_nest_depth())
            .with_func_args_deserialization(function_value_extension)
            .with_delayed_fields_serde()
            .deserialize(bytes, layout)
    } else {
        ValueSerDeContext::new(function_value_extension.max_value_nest_depth())
            .with_func_args_deserialization(function_value_extension)
            .deserialize(bytes, layout)
    };
    deserialization_result.ok_or_else(|| partial_extension_error("cannot deserialize table value"))
}

fn partial_extension_error(msg: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::value::MoveStructLayout;
    use move_vm_types::values::{AbstractFunction, SerializedFunctionData};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct MockItem {
        value: u64,
    }

    #[derive(Default)]
    struct MockState {
        data: BTreeMap<(TableHandle, Vec<u8>), Bytes>,
    }

    impl MockState {
        fn insert_value(&mut self, key: Vec<u8>, value: u64) {
            self.data.insert(
                (TableHandle(AccountAddress::ONE), key),
                bcs::to_bytes(&MockItem { value }).unwrap().into(),
            );
        }
    }

    impl TableResolver for MockState {
        fn resolve_table_entry_bytes_with_layout(
            &self,
            handle: &TableHandle,
            key: &[u8],
            _layout: Option<&MoveTypeLayout>,
        ) -> PartialVMResult<Option<Bytes>> {
            Ok(self.data.get(&(*handle, key.to_vec())).cloned())
        }
    }

    fn new_empty_table_for_test() -> Table {
        Table {
            // Handle and key layout are irrelevant.
            handle: TableHandle(AccountAddress::ONE),
            key_layout: MoveTypeLayout::U8,
            value_layout_info: LayoutInfo {
                // Values are structs with a single u64 field, so we make sure layout matches.
                layout: Arc::new(MoveTypeLayout::Struct(MoveStructLayout::Runtime(vec![
                    MoveTypeLayout::U64,
                ]))),
                contains_delayed_fields: false,
            },
            content: BTreeMap::new(),
        }
    }

    struct MockFunctionValueExtension;

    impl FunctionValueExtension for MockFunctionValueExtension {
        fn create_from_serialization_data(
            &self,
            _data: SerializedFunctionData,
        ) -> PartialVMResult<Box<dyn AbstractFunction>> {
            unimplemented!("Irrelevant for the test")
        }

        fn get_serialization_data(
            &self,
            _fun: &dyn AbstractFunction,
        ) -> PartialVMResult<SerializedFunctionData> {
            unimplemented!("Irrelevant for the test")
        }

        fn max_value_nest_depth(&self) -> Option<u64> {
            None
        }
    }

    #[test]
    fn test_extension_is_reset_correctly() {
        let state = MockState::default();
        let mut context = NativeTableContext::new([0u8; 32], &state);
        context.table_data.borrow_mut().new_tables_counter = 10;

        context.reset(&[1u8; 32], &[], 10);

        let NativeTableContext {
            resolver: _,
            txn_hash,
            table_data,
        } = context;
        let TableData {
            vc: _,
            tables: _,
            new_tables_counter,
            legacy_new_tables: _,
            legacy_removed_tables: _,
        } = table_data.into_inner();
        assert_eq!(txn_hash, [1u8; 32]);
        assert_eq!(new_tables_counter, 0);
    }

    #[test]
    fn test_table_item_needs_initialization() {
        let mut vc = VersionController::new();
        vc.save();
        let mut table = new_empty_table_for_test();

        // Empty table should need initialization.
        assert!(table.table_item_needs_initialization(b"key1", vc.current_version()));

        let gv = GlobalValue::none();
        table
            .content
            .entry(b"key1".to_vec())
            .or_insert_with(|| TableItem::new(0))
            .gv
            .set_empty(gv, vc.current_version());

        // Should no longer need initialization for existing key.
        assert!(!table.table_item_needs_initialization(b"key1", vc.current_version()));
        assert!(table.table_item_needs_initialization(b"key2", vc.current_version()));

        // Back to version 1, we need to re-initialize now.
        vc.undo();
        assert!(table.table_item_needs_initialization(b"key1", vc.current_version()));
    }

    #[test]
    fn test_table_item_versioning() {
        let mut state = MockState::default();
        state.insert_value(b"key1".to_vec(), 100);

        let mut vc = VersionController::new();
        let mut table = new_empty_table_for_test();

        let (gv, bytes_loaded) = table
            .get_or_create_table_item(
                &MockFunctionValueExtension,
                &state,
                b"key1".to_vec(),
                vc.current_version(),
            )
            .unwrap();

        assert!(bytes_loaded.is_some());
        assert!(gv.exists().unwrap());

        // Save and advance to version 2.
        vc.save();

        // Mutable access - should trigger CoW.
        let (gv, bytes_loaded) = table
            .get_or_create_table_item_mut(
                &MockFunctionValueExtension,
                &state,
                b"key1".to_vec(),
                vc.current_version(),
            )
            .unwrap();
        assert!(bytes_loaded.is_none());

        // Delete the global value for version 2.
        gv.move_from().unwrap();
        assert!(!gv.exists().unwrap());
        let versions = table
            .content
            .get(b"key1".as_slice())
            .unwrap()
            .gv
            .to_versions_vec();
        assert_eq!(versions, vec![1, 2]);

        // Undo back to version 1
        vc.undo();

        let (gv, bytes_loaded) = table
            .get_or_create_table_item(
                &MockFunctionValueExtension,
                &state,
                b"key1".to_vec(),
                vc.current_version(),
            )
            .unwrap();
        assert!(bytes_loaded.is_none());
        assert!(gv.exists().unwrap());

        let versions = table
            .content
            .get(b"key1".as_slice())
            .unwrap()
            .gv
            .to_versions_vec();
        assert_eq!(versions, vec![1]);
    }
}
