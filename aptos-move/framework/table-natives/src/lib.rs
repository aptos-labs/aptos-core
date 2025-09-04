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
    write_set::{WriteOp, WriteOpSize},
};
use aptos_vm_types::{
    abstract_write_op::{AbstractResourceWriteOp, InPlaceDelayedFieldChangeOp},
    change_set::WriteOpInfo,
    resolver::ExecutorView,
    storage::{change_set_configs::ChangeSetSizeTracker, space_pricing::ChargeAndRefund},
};
use better_any::{Tid, TidAble};
use bytes::Bytes;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
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
    global_values::{
        GlobalValueState, VersionController, VersionedGlobalValue, VersionedOnceCell,
    },
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
    tables: BTreeMap<TableHandle, Table>,
    new_tables_counter: u32,
    legacy_new_tables: BTreeMap<TableHandle, TableInfo>,
    legacy_removed_tables: BTreeSet<TableHandle>,
}

/// A structure containing information about the layout of a value stored in a table. Needed in
/// order to replace delayed fields.
struct LayoutInfo {
    layout: Arc<MoveTypeLayout>,
    contains_delayed_fields: bool,
}

enum TableItem {
    Legacy(GlobalValue),
    New {
        gv: VersionedGlobalValue,
        metadata_and_size: VersionedOnceCell<(StateValueMetadata, WriteOpSize, u64)>,
    },
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

    pub fn materialize(
        &self,
        executor_view: &dyn ExecutorView,
        function_extension: &impl FunctionValueExtension,
        new_slot_metadata: &Option<StateValueMetadata>,
        delayed_field_ids: &HashSet<DelayedFieldID>,
    ) -> PartialVMResult<()> {
        let mut table_data = self.table_data.borrow_mut();
        let current_version = table_data.vc.current_version();

        for (handle, table) in table_data.tables.iter_mut() {
            for (key, table_item) in table.content.iter_mut() {
                let (inner, metadata_and_size) = match table_item {
                    TableItem::Legacy(_) => unreachable!("New flow only!"),
                    TableItem::New {
                        gv,
                        metadata_and_size,
                    } => (gv, metadata_and_size),
                };
                let need_compute = metadata_and_size
                    .needs_derived_recomputation(&mut inner.versions.borrow_mut(), current_version);
                if !need_compute {
                    continue;
                }

                let state_key = StateKey::table_item(&AptosTableHandle(handle.0), key);
                let mut inner = inner.versions.borrow_mut();
                let v = inner.latest(current_version).unwrap(); // we have just checked.

                let state_value_metadata =
                    executor_view.get_resource_state_value_metadata(&state_key)?;
                let previous_size = executor_view.get_resource_state_value_size(&state_key)?;

                let (state_value_metadata, write_op_size) = match v {
                    GlobalValueState::None => {
                        continue;
                    },
                    GlobalValueState::Read(_) => {
                        if table
                            .value_layout_info
                            .layout_if_contains_delayed_fields()
                            .is_some()
                        {
                            // TODO: cache metadata on read, cache previous size on read.
                            if let Some((metadata, write_len)) = executor_view
                                .get_read_needing_exchange(&state_key, delayed_field_ids)?
                            {
                                (metadata, WriteOpSize::Modification { write_len })
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    },
                    GlobalValueState::Creation(v) => {
                        if state_value_metadata.is_some() {
                            return Err(PartialVMError::new(
                                StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                            ));
                        }

                        let meta = match new_slot_metadata {
                            None => StateValueMetadata::none(),
                            Some(metadata) => metadata.clone(),
                        };

                        let write_len =
                            serialize_value_v2(function_extension, &table.value_layout_info, v)?
                                .len() as u64;
                        (meta, WriteOpSize::Creation { write_len })
                    },
                    GlobalValueState::Modification(v) => {
                        let state_value_metadata = state_value_metadata.ok_or_else(|| {
                            PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                        })?;
                        let write_len =
                            serialize_value_v2(function_extension, &table.value_layout_info, v)?
                                .len() as u64;
                        (state_value_metadata, WriteOpSize::Modification {
                            write_len,
                        })
                    },
                    GlobalValueState::Deletion => {
                        let state_value_metadata = state_value_metadata.ok_or_else(|| {
                            PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                        })?;
                        (state_value_metadata, WriteOpSize::Deletion)
                    },
                };
                metadata_and_size.set(
                    (state_value_metadata, write_op_size, previous_size),
                    current_version,
                )?;
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
            let metadata_and_size = match table_item {
                TableItem::Legacy(_) => unreachable!(),
                TableItem::New {
                    metadata_and_size, ..
                } => metadata_and_size,
            };

            if let Some((metadata_mut, op_size, prev_size)) =
                metadata_and_size.get_mut(current_version)
            {
                if let Some(pricing) = change_set_size_tracker.disk_pricing {
                    let ChargeAndRefund { charge, refund } = pricing.charge_refund_write_op(
                        change_set_size_tracker.txn_gas_params.unwrap(),
                        WriteOpInfo {
                            key: &state_key,
                            op_size: *op_size,
                            prev_size: *prev_size,
                            metadata_mut,
                        },
                    );
                    change_set_size_tracker.write_fee += charge;
                    change_set_size_tracker.total_refund += refund;
                }

                change_set_size_tracker.record_write_op(&state_key, *op_size)?;
                gas_meter.charge_io_gas_for_write(&state_key, op_size)?;
            }
        }
        Ok(())
    }

    pub fn take_writes(
        &mut self,
        executor_view: &dyn ExecutorView,
        function_extension: &impl FunctionValueExtension,
        delayed_field_ids: &HashSet<DelayedFieldID>,
    ) -> VMResult<BTreeMap<StateKey, AbstractResourceWriteOp>> {
        let mut changes = BTreeMap::new();
        let mut table_data = self.table_data.borrow_mut();

        let current_version = table_data.vc.current_version();
        for (handle, table) in table_data.tables.iter_mut() {
            for (key, table_item) in table.content.iter_mut() {
                let (inner, metadata_and_size) = match table_item {
                    TableItem::Legacy(_) => unreachable!("New flow only!"),
                    TableItem::New {
                        gv,
                        metadata_and_size,
                    } => (gv, metadata_and_size),
                };

                let state_key = StateKey::table_item(&AptosTableHandle(handle.0), key);
                let mut vv = inner.versions.borrow_mut();
                match vv.take_latest(current_version) {
                    None | Some(GlobalValueState::None) => {
                        continue;
                    },
                    Some(GlobalValueState::Read(_)) => {
                        if let Some(layout) =
                            table.value_layout_info.layout_if_contains_delayed_fields()
                        {
                            // TODO: cache metadata on read, cache previous size on read.
                            if let Some((metadata, materialized_size)) = executor_view
                                .get_read_needing_exchange(&state_key, delayed_field_ids)?
                            {
                                let change = InPlaceDelayedFieldChangeOp {
                                    // TODO:
                                    //   Do we need to compare with layout that was captured?
                                    layout,
                                    materialized_size,
                                    metadata,
                                };
                                let op = AbstractResourceWriteOp::InPlaceDelayedFieldChange(change);
                                changes.insert(state_key, op);
                            }
                        }
                    },
                    Some(GlobalValueState::Creation(v)) => {
                        let metadata = metadata_and_size
                            .take_latest(current_version)
                            .expect("TODO: invariant violation: no refund")
                            .0;
                        let bytes =
                            serialize_value_v2(function_extension, &table.value_layout_info, &v)
                                .map_err(|err| err.finish(Location::Undefined))?;
                        let write_op = WriteOp::creation(bytes, metadata);
                        let op = AbstractResourceWriteOp::from_resource_write_with_maybe_layout(
                            write_op,
                            table.value_layout_info.layout_if_contains_delayed_fields(),
                        );
                        changes.insert(state_key, op);
                    },
                    Some(GlobalValueState::Modification(v)) => {
                        let metadata = match metadata_and_size.take_latest(current_version) {
                            Some((m, _, _)) => m,
                            None => executor_view
                                .get_resource_state_value_metadata(&state_key)
                                .map_err(|err| err.finish(Location::Undefined))?
                                .ok_or_else(|| {
                                    PartialVMError::new(
                                        StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                                    )
                                    .finish(Location::Undefined)
                                })?,
                        };
                        let bytes =
                            serialize_value_v2(function_extension, &table.value_layout_info, &v)
                                .map_err(|err| err.finish(Location::Undefined))?;
                        let write_op = WriteOp::modification(bytes, metadata);
                        let op = AbstractResourceWriteOp::from_resource_write_with_maybe_layout(
                            write_op,
                            table.value_layout_info.layout_if_contains_delayed_fields(),
                        );
                        changes.insert(state_key, op);
                    },
                    Some(GlobalValueState::Deletion) => {
                        let metadata = match metadata_and_size.take_latest(current_version) {
                            Some((m, _, _)) => m,
                            None => executor_view
                                .get_resource_state_value_metadata(&state_key)
                                .map_err(|err| err.finish(Location::Undefined))?
                                .ok_or_else(|| {
                                    PartialVMError::new(
                                        StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                                    )
                                    .finish(Location::Undefined)
                                })?,
                        };
                        let write_op = WriteOp::deletion(metadata);
                        let op = AbstractResourceWriteOp::from_resource_write_with_maybe_layout(
                            write_op,
                            table.value_layout_info.layout_if_contains_delayed_fields(),
                        );
                        changes.insert(state_key, op);
                    },
                }
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
            legacy_new_tables: new_tables,
            legacy_removed_tables: removed_tables,
            tables,
            ..
        } = table_data.into_inner();
        let mut changes = BTreeMap::new();
        for (handle, table) in tables {
            let Table {
                value_layout_info,
                content,
                ..
            } = table;
            let mut entries = BTreeMap::new();
            for (key, item) in content {
                let gv = match item {
                    TableItem::Legacy(gv) => gv,
                    TableItem::New { .. } => unreachable!("Only used for legacy flow"),
                };
                let op = match gv.into_effect() {
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
    fn initialize_slot(
        &mut self,
        function_value_extension: &dyn FunctionValueExtension,
        table_context: &NativeTableContext,
        key: Vec<u8>,
        current_version: u32,
    ) -> PartialVMResult<(&mut VersionedGlobalValue, Option<Option<NumBytes>>)> {
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
                        GlobalValueState::Read(val),
                        Some(NumBytes::new(val_bytes.len() as u64)),
                    )
                },
                None => (GlobalValueState::None, None),
            })
        };

        let requires_initialization = match self.content.get_mut(&key) {
            None => true,
            Some(TableItem::Legacy(_)) => unreachable!(),
            Some(TableItem::New { gv, .. }) => gv.check_if_initialized(current_version),
        };

        if requires_initialization {
            let (sv, bytes_loaded) = init_table_item()?;
            let gv = match self.content.entry(key) {
                Entry::Vacant(entry) => {
                    let table_item = TableItem::New {
                        gv: VersionedGlobalValue::new(sv, current_version),
                        metadata_and_size: VersionedOnceCell::empty(),
                    };
                    match entry.insert(table_item) {
                        TableItem::Legacy(_) => unreachable!(),
                        TableItem::New { gv, .. } => gv,
                    }
                },
                Entry::Occupied(entry) => {
                    let gv = match entry.into_mut() {
                        TableItem::Legacy(_) => unreachable!(),
                        TableItem::New { gv, .. } => gv,
                    };
                    gv.set(sv, current_version)?;
                    gv
                },
            };
            return Ok((gv, Some(bytes_loaded)));
        }

        let gv = match self.content.get_mut(&key).expect("Table item exists") {
            TableItem::Legacy(_) => unreachable!(),
            TableItem::New { gv, .. } => gv,
        };
        Ok((gv, None))
    }

    fn get_or_create_global_value(
        &mut self,
        function_value_extension: &dyn FunctionValueExtension,
        table_context: &NativeTableContext,
        key: Vec<u8>,
    ) -> PartialVMResult<(&mut GlobalValue, Option<Option<NumBytes>>)> {
        Ok(match self.content.entry(key) {
            Entry::Vacant(entry) => {
                // If there is an identifier mapping, we need to pass layout to
                // ensure it gets recorded.
                let data = table_context
                    .resolver
                    .resolve_table_entry_bytes_with_layout(
                        &self.handle,
                        entry.key(),
                        if self.value_layout_info.contains_delayed_fields {
                            Some(&self.value_layout_info.layout)
                        } else {
                            None
                        },
                    )?;

                let (gv, loaded) = match data {
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
                let item = entry.insert(TableItem::Legacy(gv));
                match item {
                    TableItem::Legacy(gv) => (gv, Some(loaded)),
                    TableItem::New { .. } => unreachable!(),
                }
            },
            Entry::Occupied(entry) => match entry.into_mut() {
                TableItem::Legacy(gv) => (gv, None),
                TableItem::New { .. } => unreachable!(),
            },
        })
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

    let use_new_flow = context.get_feature_flags().is_aptos_vm_v2_enabled();
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

    table_data.new_tables_counter += 1;
    if !use_new_flow {
        let key_type = context.type_to_type_tag(&ty_args[0])?;
        let value_type = context.type_to_type_tag(&ty_args[1])?;
        assert!(table_data
            .legacy_new_tables
            .insert(TableHandle(handle), TableInfo::new(key_type, value_type))
            .is_none());
    }

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

    let use_new_flow = context.get_feature_flags().is_aptos_vm_v2_enabled();
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

    let (res, mem_usage, loaded) = if use_new_flow {
        let (gv, loaded) = table.initialize_slot(
            &function_value_extension,
            table_context,
            key_bytes,
            current_version,
        )?;
        // Note: there is no mem usage because global is none/deleted.
        (gv.move_to(current_version, val), None, loaded)
    } else {
        let (gv, loaded) = table.get_or_create_global_value(
            &function_value_extension,
            table_context,
            key_bytes,
        )?;
        let mem_usage = gv
            .view()
            .map(|val| {
                abs_val_gas_params
                    .abstract_heap_size(&val, gas_feature_version)
                    .map(u64::from)
            })
            .transpose()?;
        (gv.move_to(val).map_err(|(err, _)| err), mem_usage, loaded)
    };

    let res = match res {
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

    let use_new_flow = context.get_feature_flags().is_aptos_vm_v2_enabled();
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

    let (res, mem_usage, loaded) = if use_new_flow {
        let (gv, loaded) = table.initialize_slot(
            &function_value_extension,
            table_context,
            key_bytes,
            current_version,
        )?;
        let res = gv.borrow_global(current_version);
        let mem_usage = res
            .as_ref()
            .ok()
            .map(|val| {
                abs_val_gas_params
                    .abstract_heap_size(&val, gas_feature_version)
                    .map(u64::from)
            })
            .transpose()?;
        (res, mem_usage, loaded)
    } else {
        let (gv, loaded) = table.get_or_create_global_value(
            &function_value_extension,
            table_context,
            key_bytes,
        )?;
        let mem_usage = gv
            .view()
            .map(|val| {
                abs_val_gas_params
                    .abstract_heap_size(&val, gas_feature_version)
                    .map(u64::from)
            })
            .transpose()?;
        (gv.borrow_global(), mem_usage, loaded)
    };

    let res = match res {
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

    let use_new_flow = context.get_feature_flags().is_aptos_vm_v2_enabled();
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

    let (exists, mem_usage, loaded) = if use_new_flow {
        let (gv, loaded) = table.initialize_slot(
            &function_value_extension,
            table_context,
            key_bytes,
            current_version,
        )?;
        let res = gv.exists(current_version);
        let mem_usage = res
            .as_ref()
            .ok()
            .map(|val| {
                abs_val_gas_params
                    .abstract_heap_size(&val, gas_feature_version)
                    .map(u64::from)
            })
            .transpose()?;
        (res?, mem_usage, loaded)
    } else {
        let (gv, loaded) = table.get_or_create_global_value(
            &function_value_extension,
            table_context,
            key_bytes,
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
        (exists, mem_usage, loaded)
    };

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

    let use_new_flow = context.get_feature_flags().is_aptos_vm_v2_enabled();
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

    let (res, mem_usage, loaded) = if use_new_flow {
        let (gv, loaded) = table.initialize_slot(
            &function_value_extension,
            table_context,
            key_bytes,
            current_version,
        )?;
        let res = gv.move_from(current_version);
        let mem_usage = res
            .as_ref()
            .ok()
            .map(|val| {
                abs_val_gas_params
                    .abstract_heap_size(&val, gas_feature_version)
                    .map(u64::from)
            })
            .transpose()?;
        (res, mem_usage, loaded)
    } else {
        let (gv, loaded) = table.get_or_create_global_value(
            &function_value_extension,
            table_context,
            key_bytes,
        )?;
        let mem_usage = gv
            .view()
            .map(|val| {
                abs_val_gas_params
                    .abstract_heap_size(&val, gas_feature_version)
                    .map(u64::from)
            })
            .transpose()?;
        (gv.move_from(), mem_usage, loaded)
    };

    let res = match res {
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

    let use_new_flow = context.get_feature_flags().is_aptos_vm_v2_enabled();
    let (extensions, mut loader_context) = context.extensions_with_loader_context();
    let table_context = extensions.get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;
    if !use_new_flow {
        // TODO: do we need one line like below?
        assert!(table_data.legacy_removed_tables.insert(handle));
        // TODO: Can the following line be removed?
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;
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
