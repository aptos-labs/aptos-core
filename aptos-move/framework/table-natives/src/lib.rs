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
    vm::versioning::{VersionController, VersionedSlot},
    write_set::WriteOp,
};
use aptos_vm_types::{
    abstract_write_op::{AbstractResourceWriteOp, InPlaceDelayedFieldChangeOp},
    change_set::WriteOpInfo,
    resolver::ExecutorView,
    storage::{change_set_configs::ChangeSetSizeTracker, space_pricing::ChargeAndRefund},
};
use better_any::{Tid, TidAble};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress, effects::Op, gas_algebra::NumBytes, identifier::Identifier,
    value::MoveTypeLayout, vm_status::StatusCode,
};
// ===========================================================================================
// Public Data Structures and Constants
pub use move_table_extension::{TableHandle, TableInfo, TableResolver};
use move_vm_runtime::{
    native_extensions::VersionControlledNativeExtension,
    native_functions::{LoaderContext, NativeFunctionTable},
};
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    loaded_data::runtime_types::Type,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::{GlobalValue, Reference, StructRef, Value},
};
use sha3::{Digest, Sha3_256};
use smallvec::{smallvec, SmallVec};
use std::{
    cell::RefCell,
    collections::{btree_map::Entry, BTreeMap, BTreeSet, HashSet, VecDeque},
    mem::drop,
    sync::Arc,
};

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
    // TODO: add versioning here or remove.
    new_tables: BTreeMap<TableHandle, TableInfo>,
    new_tables_counter: u32,
    // TODO: add versioning here or remove.
    removed_tables: BTreeSet<TableHandle>,
    tables: BTreeMap<TableHandle, Table>,
}

/// A structure containing information about the layout of a value stored in a table. Needed in
/// order to replace delayed fields.
struct LayoutInfo {
    layout: Arc<MoveTypeLayout>,
    contains_delayed_fields: bool,
}

#[derive(Debug, Clone)]
struct MaterializedGlobalValue {
    op: AbstractResourceWriteOp,
    previous_size: u64,
}

struct TableItem {
    gv: VersionedSlot<GlobalValue>,
    materialization: VersionedSlot<MaterializedGlobalValue>,
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
    pub entries: BTreeMap<Vec<u8>, Op<(Bytes, Option<Arc<MoveTypeLayout>>)>>,
}

// =========================================================================================
// Implementation of Native Table Context

impl<'a> VersionControlledNativeExtension for NativeTableContext<'a> {
    fn undo(&mut self) {
        self.table_data.borrow_mut().vc.undo();
    }

    fn save(&mut self) {
        self.table_data.borrow_mut().vc.save();
    }

    fn update(&mut self, txn_hash: &[u8; 32], _script_hash: &[u8]) {
        self.txn_hash = *txn_hash;
        self.table_data.borrow_mut().new_tables_counter = 0;
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

    fn handle_delayed_fields_materialization(
        &self,
        value_layout_info: &LayoutInfo,
        state_key: &StateKey,
        executor_view: &dyn ExecutorView,
        delayed_field_ids: &HashSet<DelayedFieldID>,
    ) -> PartialVMResult<Option<MaterializedGlobalValue>> {
        if let Some(layout) = value_layout_info.layout_if_contains_delayed_fields() {
            // TODO: cache metadata on read, cache previous size on read.
            if let Some((metadata, materialized_size)) =
                executor_view.get_read_needing_exchange(state_key, delayed_field_ids)?
            {
                let change = InPlaceDelayedFieldChangeOp {
                    // TODO:
                    //   Do we need to compare with layout that was captured?
                    layout,
                    materialized_size,
                    metadata,
                };
                let op = AbstractResourceWriteOp::InPlaceDelayedFieldChange(change);
                return Ok(Some(MaterializedGlobalValue {
                    op,
                    // Previous size is the same as materialized size.
                    previous_size: materialized_size,
                }));
            }
        }
        Ok(None)
    }

    fn build_write_op(
        &self,
        function_extension: &impl FunctionValueExtension,
        op: Op<&Value>,
        layout_info: &LayoutInfo,
        state_value_metadata: Option<StateValueMetadata>,
        new_slot_metadata: &Option<StateValueMetadata>,
    ) -> PartialVMResult<AbstractResourceWriteOp> {
        let write_op = match op {
            Op::New(value) => {
                let bytes = serialize_value_v2(function_extension, layout_info, value)?;
                if state_value_metadata.is_some() {
                    return Err(PartialVMError::new(
                        StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                    ));
                }

                match new_slot_metadata {
                    None => WriteOp::legacy_creation(bytes),
                    Some(metadata) => WriteOp::creation(bytes, metadata.clone()),
                }
            },
            Op::Modify(value) => {
                let bytes = serialize_value_v2(function_extension, layout_info, value)?;
                let state_value_metadata = state_value_metadata.ok_or_else(|| {
                    PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                })?;
                WriteOp::modification(bytes, state_value_metadata)
            },
            Op::Delete => {
                let state_value_metadata = state_value_metadata.ok_or_else(|| {
                    PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                })?;
                WriteOp::deletion(state_value_metadata)
            },
        };
        Ok(
            AbstractResourceWriteOp::from_resource_write_with_maybe_layout(
                write_op,
                layout_info.layout_if_contains_delayed_fields(),
            ),
        )
    }

    pub fn materialize(
        &self,
        executor_view: &dyn ExecutorView,
        function_extension: &impl FunctionValueExtension,
        new_slot_metadata: &Option<StateValueMetadata>,
        delayed_field_ids: &HashSet<DelayedFieldID>,
        inherit_metadata: bool,
    ) -> PartialVMResult<()> {
        let mut table_data = self.table_data.borrow_mut();
        let current_version = table_data.vc.current_version();

        for (handle, table) in table_data.tables.iter_mut() {
            for (key, table_item) in table.content.iter_mut() {
                let (gv, gv_version, gv_incarnation, old_materialization) = match table_item
                    .gv
                    .needs_derived_recomputation(&mut table_item.materialization, current_version)
                {
                    Some(result) => result,
                    None => continue, // No materialization needed
                };

                let state_key = StateKey::table_item(&AptosTableHandle(handle.0), key);
                let op = match gv.effect() {
                    Some(op) => op,
                    None => {
                        if let Some(materialization) = self.handle_delayed_fields_materialization(
                            &table.value_layout_info,
                            &state_key,
                            executor_view,
                            delayed_field_ids,
                        )? {
                            table_item.materialization.set(
                                materialization,
                                gv_version,
                                gv_incarnation,
                            )?;
                        }
                        continue;
                    },
                };

                let state_value_metadata = match old_materialization {
                    Some(old_materialization) if inherit_metadata => {
                        Some(old_materialization.op.metadata().clone())
                    },
                    Some(_) | None => {
                        executor_view.get_resource_state_value_metadata(&state_key)?
                    },
                };
                let previous_size = executor_view.get_resource_state_value_size(&state_key)?;

                let op = self.build_write_op(
                    function_extension,
                    op,
                    &table.value_layout_info,
                    state_value_metadata,
                    new_slot_metadata,
                )?;

                let materialization = MaterializedGlobalValue { op, previous_size };
                table_item
                    .materialization
                    .set(materialization, gv_version, gv_incarnation)?;
            }
        }
        Ok(())
    }

    pub fn charge_write_ops(
        &self,
        change_set_size_tracker: &mut ChangeSetSizeTracker,
        gas_meter: &mut impl AptosGasMeter,
    ) -> VMResult<()> {
        // TODO: materialize here?

        let mut table_data = self.table_data.borrow_mut();

        let current_version = table_data.vc.current_version();
        for (handle, key, table_item) in table_data.tables.iter_mut().flat_map(|(handle, table)| {
            table
                .content
                .iter_mut()
                .map(move |(key, table_item)| (handle, key, table_item))
        }) {
            // TODO: cache state keys.
            let state_key = StateKey::table_item(&AptosTableHandle(handle.0), key);
            if let Some(materialized_value) = table_item
                .materialization
                .latest_mut_sync_for_read(current_version)
            {
                let MaterializedGlobalValue { op, previous_size } = materialized_value;

                if let Some(pricing) = change_set_size_tracker.disk_pricing {
                    let ChargeAndRefund { charge, refund } = pricing.charge_refund_write_op(
                        change_set_size_tracker.txn_gas_params.unwrap(),
                        WriteOpInfo {
                            key: &state_key,
                            op_size: op.materialized_size(),
                            prev_size: *previous_size,
                            metadata_mut: op.metadata_mut(),
                        },
                    );
                    change_set_size_tracker.write_fee += charge;
                    change_set_size_tracker.total_refund += refund;
                }

                change_set_size_tracker.record_write_op(&state_key, op.materialized_size())?;
                gas_meter.charge_io_gas_for_write(&state_key, &op.materialized_size())?;
            }
        }
        Ok(())
    }

    pub fn take_writes(&mut self) -> VMResult<BTreeMap<StateKey, AbstractResourceWriteOp>> {
        // TODO: Materialize here?

        let mut changes = BTreeMap::new();
        let mut table_data = self.table_data.borrow_mut();

        let current_version = table_data.vc.current_version();
        for (handle, key, table_item) in table_data.tables.iter_mut().flat_map(|(handle, table)| {
            table
                .content
                .iter_mut()
                .map(move |(key, table_item)| (handle, key, table_item))
        }) {
            // TODO: cache state keys.
            let state_key = StateKey::table_item(&AptosTableHandle(handle.0), key);
            if let Some(materialization) = table_item.materialization.take_latest(current_version) {
                let MaterializedGlobalValue { op, .. } = materialization;
                changes.insert(state_key, op);
            }
        }
        Ok(changes)
    }

    /// Computes the change set from a NativeTableContext.
    pub fn into_change_set(
        self,
        function_value_extension: &impl FunctionValueExtension,
    ) -> PartialVMResult<TableChangeSet> {
        let NativeTableContext { table_data, .. } = self;
        let TableData {
            vc,
            new_tables,
            removed_tables,
            tables,
            ..
        } = table_data.into_inner();

        let current_version = vc.current_version();
        let mut changes = BTreeMap::new();

        for (handle, table) in tables {
            let Table {
                value_layout_info,
                content,
                ..
            } = table;
            let mut entries = BTreeMap::new();
            for (key, mut slot) in content {
                let op = match slot
                    .gv
                    .take_latest(current_version)
                    .and_then(|gv| gv.into_effect())
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
            layout: Arc::new(layout),
            contains_delayed_fields,
        })
    }

    fn layout_if_contains_delayed_fields(&self) -> Option<Arc<MoveTypeLayout>> {
        self.contains_delayed_fields.then(|| self.layout.clone())
    }

    fn layout_ref_if_contains_delayed_fields(&self) -> Option<&MoveTypeLayout> {
        self.contains_delayed_fields.then(|| self.layout.as_ref())
    }
}

impl Table {
    fn get_or_create_global_value(
        &mut self,
        function_value_extension: &dyn FunctionValueExtension,
        table_context: &NativeTableContext,
        key: Vec<u8>,
        current_version: u32,
    ) -> PartialVMResult<(&mut GlobalValue, Option<Option<NumBytes>>)> {
        let init_table_item = || -> PartialVMResult<_> {
            // If there are delayed fields, we need to pass layout when resolving bytes from
            // storage.
            let layout_if_contains_delayed_fields = self
                .value_layout_info
                .layout_ref_if_contains_delayed_fields();
            let data = table_context
                .resolver
                .resolve_table_entry_bytes_with_layout(
                    &self.handle,
                    &key,
                    layout_if_contains_delayed_fields,
                )?;

            Ok(match data {
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
            })
        };

        let requires_initialization = match self.content.get_mut(&key) {
            None => true,
            Some(slot) => slot.gv.latest(current_version).is_none(),
        };

        if requires_initialization {
            let (gv, bytes_loaded) = init_table_item()?;
            let gv = match self.content.entry(key) {
                Entry::Vacant(entry) => {
                    let table_item = TableItem {
                        gv: VersionedSlot::new(gv, current_version),
                        materialization: VersionedSlot::empty(),
                    };
                    entry
                        .insert(table_item)
                        .gv
                        .latest_mut(current_version)?
                        .expect("Valur must exist at current version")
                },
                Entry::Occupied(entry) => entry.into_mut().gv.set(gv, current_version, 0)?,
            };
            return Ok((gv, Some(bytes_loaded)));
        }

        let gv = self
            .content
            .get_mut(&key)
            .expect("Table item exists")
            .gv
            .latest_mut(current_version)?
            .expect("Table item contains a value");
        Ok((gv, None))
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
                ("borrow_box_mut", native_borrow_box),
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
    assert!(table_data
        .new_tables
        .insert(TableHandle(handle), TableInfo::new(key_type, value_type))
        .is_none());
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

    let (gv, loaded) = table.get_or_create_global_value(
        &function_value_extension,
        table_context,
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

    let (gv, loaded) = table.get_or_create_global_value(
        &function_value_extension,
        table_context,
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

    let (gv, loaded) = table.get_or_create_global_value(
        &function_value_extension,
        table_context,
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
    let exists = Value::bool(gv.exists()?);

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

    let (gv, loaded) = table.get_or_create_global_value(
        &function_value_extension,
        table_context,
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

    let (extensions, mut loader_context) = context.extensions_with_loader_context();
    let table_context = extensions.get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;
    // TODO: Can the following line be removed?
    table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    assert!(table_data.removed_tables.insert(handle));

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
) -> PartialVMResult<(Bytes, Option<Arc<MoveTypeLayout>>)> {
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

fn serialize_value_v2(
    function_value_extension: &dyn FunctionValueExtension,
    layout_info: &LayoutInfo,
    val: &Value,
) -> PartialVMResult<Bytes> {
    let max_value_nest_depth = function_value_extension.max_value_nest_depth();
    let mut ctx = ValueSerDeContext::new(max_value_nest_depth)
        .with_func_args_deserialization(function_value_extension);
    if layout_info.contains_delayed_fields {
        ctx = ctx.with_delayed_fields_serde();
    }

    ctx.serialize(val, layout_info.layout.as_ref())?
        .map(Bytes::from)
        .ok_or_else(|| partial_extension_error("cannot serialize table value"))
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
