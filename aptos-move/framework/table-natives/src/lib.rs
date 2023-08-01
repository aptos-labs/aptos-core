// Copyright © Aptos Foundation

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
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress, effects::Op, gas_algebra::NumBytes, identifier::Identifier,
    value::MoveTypeLayout, vm_status::StatusCode,
};
// ===========================================================================================
// Public Data Structures and Constants
pub use move_table_extension::{
    TableChange, TableChangeSet, TableHandle, TableInfo, TableResolver,
};
use move_vm_runtime::native_functions::NativeFunctionTable;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{GlobalValue, Reference, StructRef, Value},
};
use sha3::{Digest, Sha3_256};
use smallvec::{smallvec, SmallVec};
use std::{
    cell::RefCell,
    collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque},
    mem::drop,
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
    new_tables: BTreeMap<TableHandle, TableInfo>,
    removed_tables: BTreeSet<TableHandle>,
    tables: BTreeMap<TableHandle, Table>,
}

/// A structure representing a single table.
struct Table {
    handle: TableHandle,
    key_layout: MoveTypeLayout,
    value_layout: MoveTypeLayout,
    content: BTreeMap<Vec<u8>, GlobalValue>,
}

/// The field index of the `handle` field in the `Table` Move struct.
const HANDLE_FIELD_INDEX: usize = 0;

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
    pub fn into_change_set(self) -> PartialVMResult<TableChangeSet> {
        let NativeTableContext { table_data, .. } = self;
        let TableData {
            new_tables,
            removed_tables,
            tables,
        } = table_data.into_inner();
        let mut changes = BTreeMap::new();
        for (handle, table) in tables {
            let Table {
                value_layout,
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
                        let bytes = serialize(&value_layout, &val)?;
                        entries.insert(key, Op::New(bytes));
                    },
                    Op::Modify(val) => {
                        let bytes = serialize(&value_layout, &val)?;
                        entries.insert(key, Op::Modify(bytes));
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
        context: &SafeNativeContext,
        handle: TableHandle,
        key_ty: &Type,
        value_ty: &Type,
    ) -> PartialVMResult<&mut Table> {
        Ok(match self.tables.entry(handle) {
            Entry::Vacant(e) => {
                let key_layout = context.type_to_type_layout(key_ty)?;
                let value_layout = context.type_to_type_layout(value_ty)?;
                let table = Table {
                    handle,
                    key_layout,
                    value_layout,
                    content: Default::default(),
                };
                e.insert(table)
            },
            Entry::Occupied(e) => e.into_mut(),
        })
    }
}

impl Table {
    fn get_or_create_global_value(
        &mut self,
        context: &NativeTableContext,
        key: Vec<u8>,
    ) -> PartialVMResult<(&mut GlobalValue, Option<Option<NumBytes>>)> {
        Ok(match self.content.entry(key) {
            Entry::Vacant(entry) => {
                let (gv, loaded) = match context
                    .resolver
                    .resolve_table_entry(&self.handle, entry.key())
                    .map_err(|err| {
                        partial_extension_error(format!("remote table resolver failure: {}", err))
                    })? {
                    Some(val_bytes) => {
                        let val = deserialize(&self.value_layout, &val_bytes)?;
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

    let table_context = context.extensions().get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    let val = args.pop_back().unwrap();
    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table = table_data.get_or_create_table(context, handle, &ty_args[0], &ty_args[2])?;

    let key_bytes = serialize(&table.key_layout, &key)?;
    let key_cost = ADD_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) = table.get_or_create_global_value(table_context, key_bytes)?;

    let res = match gv.move_to(val) {
        Ok(_) => Ok(smallvec![]),
        Err(_) => Err(SafeNativeError::Abort {
            abort_code: ALREADY_EXISTS,
        }),
    };

    drop(table_data);

    // TODO(Gas): Figure out a way to charge this earlier.
    context.charge(key_cost)?;
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

    let table_context = context.extensions().get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table = table_data.get_or_create_table(context, handle, &ty_args[0], &ty_args[2])?;

    let key_bytes = serialize(&table.key_layout, &key)?;
    let key_cost = BORROW_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) = table.get_or_create_global_value(table_context, key_bytes)?;

    let res = match gv.borrow_global() {
        Ok(ref_val) => Ok(smallvec![ref_val]),
        Err(_) => Err(SafeNativeError::Abort {
            abort_code: NOT_FOUND,
        }),
    };

    drop(table_data);

    // TODO(Gas): Figure out a way to charge this earlier.
    context.charge(key_cost)?;
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

    let table_context = context.extensions().get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table = table_data.get_or_create_table(context, handle, &ty_args[0], &ty_args[2])?;

    let key_bytes = serialize(&table.key_layout, &key)?;
    let key_cost = CONTAINS_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) = table.get_or_create_global_value(table_context, key_bytes)?;
    let exists = Value::bool(gv.exists()?);

    drop(table_data);

    // TODO(Gas): Figure out a way to charge this earlier.
    context.charge(key_cost)?;
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

    let table_context = context.extensions().get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    let key = args.pop_back().unwrap();
    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;

    let table = table_data.get_or_create_table(context, handle, &ty_args[0], &ty_args[2])?;

    let key_bytes = serialize(&table.key_layout, &key)?;
    let key_cost = REMOVE_BOX_PER_BYTE_SERIALIZED * NumBytes::new(key_bytes.len() as u64);

    let (gv, loaded) = table.get_or_create_global_value(table_context, key_bytes)?;
    let res = match gv.move_from() {
        Ok(val) => Ok(smallvec![val]),
        Err(_) => Err(SafeNativeError::Abort {
            abort_code: NOT_FOUND,
        }),
    };

    drop(table_data);

    // TODO(Gas): Figure out a way to charge this earlier.
    context.charge(key_cost)?;
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

    let table_context = context.extensions().get::<NativeTableContext>();
    let mut table_data = table_context.table_data.borrow_mut();

    let handle = get_table_handle(&safely_pop_arg!(args, StructRef))?;
    // TODO: Can the following line be removed?
    table_data.get_or_create_table(context, handle, &ty_args[0], &ty_args[2])?;

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

fn serialize(layout: &MoveTypeLayout, val: &Value) -> PartialVMResult<Vec<u8>> {
    val.simple_serialize(layout)
        .ok_or_else(|| partial_extension_error("cannot serialize table key or value"))
}

fn deserialize(layout: &MoveTypeLayout, bytes: &[u8]) -> PartialVMResult<Value> {
    Value::simple_deserialize(bytes, layout)
        .ok_or_else(|| partial_extension_error("cannot deserialize table key or value"))
}

fn partial_extension_error(msg: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(msg.to_string())
}
