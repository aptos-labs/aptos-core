// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    state_store::state_key::StateKey,
    vm_status::StatusCode,
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use better_any::{Tid, TidAble};
use move_deps::{
    move_binary_format::errors::{PartialVMError, PartialVMResult},
    move_core_types::{account_address::AccountAddress, gas_schedule::GasCost},
    move_table_extension::{TableHandle, TableResolver},
    move_vm_runtime::{
        native_functions,
        native_functions::{NativeContext, NativeFunctionTable},
    },
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        pop_arg,
        values::{IntegerValue, Reference, Struct, StructRef, Value},
    },
};
use smallvec::smallvec;
use std::{
    cell::RefCell,
    collections::{btree_map::Entry, BTreeMap, VecDeque, BTreeSet},
    convert::TryInto,
};
use tiny_keccak::{Hasher, Sha3};

/// Specifies if aggregator instance is newly created (`New`), marked for
/// deletion (`Deleted`), or is currently being used (`Used`).
#[derive(Clone, Copy, PartialEq)]
enum Mark {
    New,
    Used,
    Deleted,
}

/// State of the aggregator: `Data` means that aggregator stores an exact value,
/// `PositiveDelta` means that actual value is not known but must be added later.
#[derive(Clone, Copy, PartialEq)]
enum AggregatorState {
    Data,
    PositiveDelta,
}

/// Internal aggregator data structure.
struct Aggregator {
    // Uniquely identifies aggregator in `AggregatorTable`.
    key: u128,
    // Describes state of aggregator: data or delta.
    state: AggregatorState,
    // Describes the value of data or delta.
    value: u128,
    // Postondition value, exceeding it aggregator overflows.
    limit: u128,
    // Mark to add/remove aggregators easily.
    mark: Mark,
}

/// When `Aggregator` overflows the `limit`.
const EAGGREGATOR_OVERFLOW: u64 = 1600;

impl Aggregator {
    /// Tries to add a `value` to the aggregator, aborting on exceeding the
    /// `limit`.
    fn add(&mut self, value: u128) -> PartialVMResult<()> {
        match self.state {
            AggregatorState::Data | AggregatorState::PositiveDelta => {
                if value > self.limit - self.value {
                    Err(abort_error(
                        "aggregator's value overflowed",
                        EAGGREGATOR_OVERFLOW,
                    ))
                } else {
                    self.value += value;
                    Ok(())
                }
            }
        }
    }

    /// Tries to return the value of the aggregator. If aggregator is in `Data`
    /// state, the value is returned immediately. If it is in the delta state,
    /// the value is resolved from the storage, delta is applied and the state
    /// changes to `Data`.
    fn read_value(
        &mut self,
        context: &NativeAggregatorContext,
        handle: &TableHandle,
        key: u128,
    ) -> PartialVMResult<u128> {
        // Aggregator knows what it has, return immediately.
        if self.state == AggregatorState::Data {
            return Ok(self.value);
        }

        // Otherwise, we have a delta and have to go to storage.
        let key_bytes = serialize(&key);
        match context.resolver.resolve_table_entry(handle, &key_bytes) {
            Err(_) => Err(extension_error("error resolving aggregator's value")),
            Ok(maybe_bytes) => {
                match maybe_bytes {
                    Some(bytes) => {
                        let value = deserialize(&bytes);

                        // Once value is reconstructed, apply delta to it and change the state.
                        self.state = AggregatorState::Data;
                        self.value += value;

                        Ok(self.value)
                    }
                    None => Err(extension_error("aggregator's value not found")),
                }
            }
        }
    }
}

/// Aggregator table - a top-level collection storing aggregators. It has a
/// handle and an inner table data structure to actually store the aggregators.
struct AggregatorTable {
    // Identifies `Table` Move struct of this `AggregatorTable`.
    table_handle: TableHandle,
    // Table to store all agregators associated with this `AggregatorTable`.
    table: BTreeMap<u128, Aggregator>,
}

impl std::ops::Deref for AggregatorTable {
    type Target = BTreeMap<u128, Aggregator>;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

impl std::ops::DerefMut for AggregatorTable {
    fn deref_mut(&mut self) -> &mut BTreeMap<u128, Aggregator> {
        &mut self.table
    }
}

impl AggregatorTable {
    /// Returns a possibly new aggregator instance. If transaction did not
    /// initialize this aggregator, then the actual value is not known, and the
    /// aggregator state must be a delta, zero-intialized.
    fn get_or_create_aggregator(&mut self, key: u128, limit: u128) -> &mut Aggregator {
        if !self.contains_key(&key) {
            let aggregator = Aggregator {
                key,
                state: AggregatorState::PositiveDelta,
                value: 0,
                limit,
                mark: Mark::Used,
            };
            self.insert(key, aggregator);
        }
        self.get_mut(&key).unwrap()
    }
}

/// Top-level struct that is used by the context to maintain the map of all
/// aggregator tables created throughout the VM session. In theory, only one
/// is sufficient.
#[derive(Default)]
struct AggregatorTableData {
    aggregator_tables: BTreeMap<TableHandle, AggregatorTable>,
}

impl AggregatorTableData {
    /// Returns a mutable reference to the `AggregatorTable` specified by its
    /// handle. A new aggregator table is created if it doesn't exist in this
    /// context.
    fn get_or_create_aggregator_table(
        &mut self,
        table_handle: TableHandle,
    ) -> &mut AggregatorTable {
        if let Entry::Vacant(e) = self.aggregator_tables.entry(table_handle) {
            let aggregator_table = AggregatorTable {
                table_handle,
                table: BTreeMap::new(),
            };
            e.insert(aggregator_table);
        }
        self.aggregator_tables.get_mut(&table_handle).unwrap()
    }
}

/// Native context that can be attached to VM NativeContextExtensions.
#[derive(Tid)]
pub struct NativeAggregatorContext<'a> {
    // Reuse table resolver since aggregator values are stored as Table values
    // internally.
    resolver: &'a dyn TableResolver,
    txn_hash: u128,
    // All existing aggregator tables in this context, wrapped in RefCell for
    // internal mutability.
    aggregator_table_data: RefCell<AggregatorTableData>,
}

impl<'a> NativeAggregatorContext<'a> {
    /// Creates a new instance of a native aggregator context. This must be
    /// passed into VM session.
    pub fn new(txn_hash: u128, resolver: &'a dyn TableResolver) -> Self {
        Self {
            resolver,
            txn_hash,
            aggregator_table_data: Default::default(),
        }
    }

    /// Temporary into_change_set!
    pub fn into_change_set(self) -> WriteSet {
        // let NativeAggregatorContext { registry_data, .. } = self;
        // let RegistryData {
        //     registries,
        // } = registry_data.into_inner();

        let mut write_set_mut = WriteSetMut::new(Vec::new());
        // for (table_handle, registry) in registries {
        //     let Registry {
        //         table,
        //         ..
        //     } = registry;
        //     for (key, aggregator) in table {
        //         let key_bytes = u128::to_be_bytes(key).to_vec();
        //         let state_key = StateKey::table_item(table_handle.0, key_bytes);

        //         let Aggregator {
        //             state,
        //             value,
        //             ..
        //         } = aggregator;

        //         // TODO: introduce deltas!
        //         assert!(state == AggregatorState::Data);

        //         let value_bytes = u128::to_be_bytes(value).to_vec();
        //         write_set_mut.push((state_key, WriteOp::Value(value_bytes)));
        //     }
        // }
        write_set_mut.freeze().unwrap()
    }
}

// ================================= Natives =================================

/// All aggregator native functions. For more details, refer to cod in
/// AggregatorTable.move.
pub fn aggregator_natives(aggregator_addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(
        aggregator_addr,
        &[
            ("AggregatorTable", "new_aggregator", native_new_aggregator),
            ("Aggregator", "add", native_add),
            ("Aggregator", "read", native_read),
            ("Aggregator", "remove_aggregator", native_remove_aggregator),
        ],
    )
}

fn native_new_aggregator(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Extract fields: `limit` of the new aggregator and a `table_handle`
    // associated with the associated aggregator table.
    let limit = pop_arg!(args, IntegerValue).value_as::<u128>()?;
    let table_handle = get_table_handle(&pop_arg!(args, StructRef))?;

    // Get the current aggregator table.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_table_data = aggregator_context.aggregator_table_data.borrow_mut();
    let aggregator_table = aggregator_table_data.get_or_create_aggregator_table(table_handle);

    // Every aggregator instance has a unique key. Here we can reuse the
    // strategy from `Table` implementation: taking hash of transaction and
    // number of aggregator instances created so far and truncating them to
    // 128 bits.
    let txn_hash_buffer = serialize(&aggregator_context.txn_hash);
    let num_aggregators_buffer = serialize(&(aggregator_table.len() as u128));

    let mut sha3 = Sha3::v256();
    sha3.update(&txn_hash_buffer);
    sha3.update(&num_aggregators_buffer);

    let mut key_bytes = [0_u8; 16];
    sha3.finalize(&mut key_bytes);
    let key = deserialize(&key_bytes.to_vec());

    // If transaction initializes aggregator, then the actual value is known,
    // and the aggregator state must be `Data`. Also, aggregators are
    // zero-initialized.
    let aggregator = Aggregator {
        key,
        state: AggregatorState::Data,
        value: 0,
        limit,
        mark: Mark::New,
    };
    aggregator_table.insert(key, aggregator);

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();

    // Return `Aggregator` Move struct to the user.
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::struct_(Struct::pack(vec![
            Value::u128(table_handle.0),
            Value::u128(key),
            Value::u128(limit),
        ]))],
    ))
}

fn native_add(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Get aggregator fields and a value to add.
    let value = pop_arg!(args, IntegerValue).value_as::<u128>()?;
    let aggregator_ref = pop_arg!(args, StructRef);
    let (table_handle, key, limit) = get_aggregator_fields(&aggregator_ref)?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_table_data = aggregator_context.aggregator_table_data.borrow_mut();

    let aggregator = aggregator_table_data
        .get_or_create_aggregator_table(table_handle)
        .get_or_create_aggregator(key, limit);

    aggregator.add(value).and_then(|_| {
        // TODO: charge gas properly.
        let cost = GasCost::new(0, 0).total();
        Ok(NativeResult::ok(cost, smallvec![]))
    })
}

fn native_read(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 1);
    let aggregator_ref = pop_arg!(args, StructRef);

    // Extract fields from aggregator struct reference.
    let (table_handle, key, limit) = get_aggregator_fields(&aggregator_ref)?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_table_data = aggregator_context.aggregator_table_data.borrow_mut();
    let aggregator = aggregator_table_data
        .get_or_create_aggregator_table(table_handle)
        .get_or_create_aggregator(key, limit);

    // Try to read its value, possibly getting it from the storage and applying
    // the delta.
    aggregator
        .read_value(aggregator_context, &table_handle, key)
        .and_then(|result| {
            // TODO: charge gas properly.
            let cost = GasCost::new(0, 0).total();
            Ok(NativeResult::ok(cost, smallvec![Value::u128(result)]))
        })
}

fn native_remove_aggregator(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 3);

    // Get table handle, aggregator key and its limit for removal.
    let limit = pop_arg!(args, IntegerValue).value_as::<u128>()?;
    let key = pop_arg!(args, IntegerValue).value_as::<u128>()?;
    let table_handle = pop_arg!(args, IntegerValue).value_as::<u128>().map(TableHandle)?;

    // Get aggregator table.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_table_data = aggregator_context.aggregator_table_data.borrow_mut();
    let aggregator_table = aggregator_table_data
        .get_or_create_aggregator_table(table_handle);

    if aggregator_table.contains_key(&key)
       && aggregator_table.get(&key).unwrap().mark == Mark::New {
        // Aggregator has been created in this context, hence we can simply
        // remove the entry from the table.
        aggregator_table.remove(&key);
    } else {
        // Aggregator has been created elsewhere. Mark for delition to produce
        // the right change set.
        let aggregator = aggregator_table.get_or_create_aggregator(key, limit);
        aggregator.mark = Mark::Deleted;
    }

    let cost = GasCost::new(0, 0).total();
    Ok(NativeResult::ok(cost, smallvec![]))
}

// ================================ Utilities ================================

/// The field index of the `table` field in the `AggregatorTable` Move
/// struct.
const TABLE_FIELD_INDEX: usize = 0;

/// The field index of the `handle` field in the `Table` Move struct.
const HANDLE_FIELD_INDEX: usize = 0;

/// Field indices of `table_handle`, `key` and `limit` fields in the
/// `Aggregator` Move struct.
const TABLE_HANDLE_FIELD_INDEX: usize = 0;
const KEY_FIELD_INDEX: usize = 1;
const LIMIT_FIELD_INDEX: usize = 2;

/// Given a reference to `AggregatorTable` Move struct, returns the value of
/// `table_handle` field.
fn get_table_handle(aggregator_table: &StructRef) -> PartialVMResult<TableHandle> {
    aggregator_table
        .borrow_field(TABLE_FIELD_INDEX)?
        .value_as::<StructRef>()?
        .borrow_field(HANDLE_FIELD_INDEX)?
        .value_as::<Reference>()?
        .read_ref()?
        .value_as::<u128>()
        .map(TableHandle)
}

/// Given a reference to `Aggregator` Move struct and field index, returns
/// its field specified by `index`.
fn get_aggregator_field(aggregator: &StructRef, index: usize) -> PartialVMResult<Value> {
    let field_ref = aggregator.borrow_field(index)?.value_as::<Reference>()?;
    field_ref.read_ref()
}

///  Given a reference to `Aggregator` Move struct, returns a tuple of its
///  fields: (`table_handle`, `key`, `limit`).
fn get_aggregator_fields(aggregator: &StructRef) -> PartialVMResult<(TableHandle, u128, u128)> {
    let table_handle = get_aggregator_field(aggregator, TABLE_HANDLE_FIELD_INDEX)?
        .value_as::<u128>()
        .map(TableHandle)?;
    let key = get_aggregator_field(aggregator, KEY_FIELD_INDEX)?.value_as::<u128>()?;
    let limit = get_aggregator_field(aggregator, LIMIT_FIELD_INDEX)?.value_as::<u128>()?;
    Ok((table_handle, key, limit))
}

/// Returns partial VM error on abort.
fn abort_error(message: &str, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}

/// Returns partial VM error on extension failure.
fn extension_error(message: &str) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(message.to_string())
}

/// Serializes aggregator value. The function is public so that it can be used by the executor.
pub fn serialize(value: &u128) -> Vec<u8> {
    bcs::to_bytes(value).expect("unexpected serialization error")
}

/// Deserializes aggregator value. The function is public so that it can be used by the executor.
pub fn deserialize(value_bytes: &Vec<u8>) -> u128{
    bcs::from_bytes(value_bytes).expect("unexpected deserialization error")
}
