// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! A crate which extends Move by tables.
//!
//! See [`Table.move`](../sources/Table.move) for language use.
//! See [`README.md`](../README.md) for integration into an adapter.

use aptos_gas_schedule::gas_params::natives::table::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use better_any::{Tid, TidAble};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress, effects::Op, gas_algebra::NumBytes, identifier::Identifier,
    value::MoveTypeLayout, vm_status::StatusCode,
};
// ===========================================================================================
// Public Data Structures and Constants
pub use move_table_extension::{TableHandle, TableInfo, TableResolver};
use move_vm_runtime::native_functions::{LoaderContext, NativeFunctionTable};
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
    new_tables: BTreeMap<TableHandle, TableInfo>,
    removed_tables: BTreeSet<TableHandle>,
    tables: BTreeMap<TableHandle, Table>,
}

/// A structure containing information about the layout of a value stored in a table. Needed in
/// order to replace delayed fields.
struct LayoutInfo {
    layout: TriompheArc<MoveTypeLayout>,
    contains_delayed_fields: bool,
}

/// A structure representing a single table.
struct Table {
    handle: TableHandle,
    key_layout: MoveTypeLayout,
    value_layout_info: LayoutInfo,
    content: BTreeMap<Vec<u8>, GlobalValue>,
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

    /// Computes the change set from a NativeTableContext.
    pub fn into_change_set(
        self,
        function_value_extension: &impl FunctionValueExtension,
    ) -> PartialVMResult<TableChangeSet> {
        let NativeTableContext { table_data, .. } = self;
        let TableData {
            new_tables,
            removed_tables,
            tables,
        } = table_data.into_inner();
        let mut changes = BTreeMap::new();
        for (handle, table) in tables {
            let Table {
                value_layout_info,
                content,
                ..
            } = table;
            let mut entries = BTreeMap::new();
            for (key, gv) in content {
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
            layout: TriompheArc::new(layout),
            contains_delayed_fields,
        })
    }
}

impl Table {
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
                (entry.insert(gv), Some(loaded))
            },
            Entry::Occupied(entry) => (entry.into_mut(), None),
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

    let table_context = context.extensions().get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    // Take the transaction hash provided by the environment, combine it with the # of tables
    // produced so far, sha256 this to produce a unique handle. Given the txn hash
    // is unique, this should create a unique and deterministic global id.
    let mut digest = Sha3_256::new();
    let table_len = table_data.new_tables.len() as u32; // cast usize to u32 to ensure same length
    Digest::update(&mut digest, table_context.txn_hash);
    Digest::update(&mut digest, table_len.to_be_bytes());
    let bytes = digest.finalize().to_vec();
    let handle = AccountAddress::from_bytes(&bytes[0..AccountAddress::LENGTH])
        .map_err(|_| partial_extension_error("Unable to create table handle"))?;
    let key_type = context.type_to_type_tag(&ty_args[0])?;
    let value_type = context.type_to_type_tag(&ty_args[1])?;
    assert!(table_data
        .new_tables
        .insert(TableHandle(handle), TableInfo::new(key_type, value_type))
        .is_none());

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

    let val = args.pop_back().unwrap();
    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table =
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    let function_value_extension = loader_context.function_value_extension();
    let key_bytes = serialize_key(&function_value_extension, &table.key_layout, &key)?;
    let key_cost = ADD_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) =
        table.get_or_create_global_value(&function_value_extension, table_context, key_bytes)?;
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

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table =
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    let function_value_extension = loader_context.function_value_extension();
    let key_bytes = serialize_key(&function_value_extension, &table.key_layout, &key)?;
    let key_cost = BORROW_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) =
        table.get_or_create_global_value(&function_value_extension, table_context, key_bytes)?;
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

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table =
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    let function_value_extension = loader_context.function_value_extension();
    let key_bytes = serialize_key(&function_value_extension, &table.key_layout, &key)?;
    let key_cost = CONTAINS_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) =
        table.get_or_create_global_value(&function_value_extension, table_context, key_bytes)?;
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

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table =
        table_data.get_or_create_table(&mut loader_context, handle, &ty_args[0], &ty_args[2])?;

    let function_value_extension = loader_context.function_value_extension();
    let key_bytes = serialize_key(&function_value_extension, &table.key_layout, &key)?;
    let key_cost = REMOVE_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) =
        table.get_or_create_global_value(&function_value_extension, table_context, key_bytes)?;
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
