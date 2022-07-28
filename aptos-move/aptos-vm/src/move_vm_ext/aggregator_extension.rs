// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    delta_change_set::{addition, deserialize, serialize, subtraction, DeltaOp},
    vm_status::StatusCode,
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
    collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque},
};
use tiny_keccak::{Hasher, Sha3};

/// State of an aggregator: `Data` means that aggregator stores an exact value,
/// `PositiveDelta` means that actual value is not known but must be added later
#[derive(Clone, Copy, PartialEq)]
pub enum AggregatorState {
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
}

/// When `Aggregator` overflows the `limit`.
const EAGGREGATOR_OVERFLOW: u64 = 1600;

/// When `Aggregator`'s underflows.
const EAGGREGATOR_UNDERFLOW: u64 = 1601;

impl Aggregator {
    /// Implements logic for adding to an aggregator.
    fn add(&mut self, value: u128) -> PartialVMResult<()> {
        // At this point, aggregator holds a positive delta or knows the value.
        // Hence, we can add, of course checking overflow condition.
        match addition(self.value, value, self.limit) {
            Some(result) => {
                self.value = result;
                Ok(())
            }
            None => Err(abort_error(
                format!("aggregator's value overflowed when adding {}", value),
                EAGGREGATOR_OVERFLOW,
            )),
        }
    }

    /// Implements logic for subtracting from an aggregator.
    fn sub(&mut self, value: u128) -> PartialVMResult<()> {
        // First, we check if the value can be subtracted in theory, that is
        // if `value <= limit`. If not, we can immediately abort.
        if value > self.limit {
            return Err(abort_error(
                format!("aggregator's value underflowed when subtracting more than aggregator can hold ({} > limit)", value),
                EAGGREGATOR_UNDERFLOW,
            ));
        }

        // We can subtract. Now consider each possible state of aggregator
        // separately.
        match self.state {
            AggregatorState::Data => {
                // Aggregator knows the value, therefore we can subtract
                // checking we don't drop below zero.
                match subtraction(self.value, value) {
                    Some(result) => {
                        self.value = result;
                        Ok(())
                    }
                    None => Err(abort_error(
                        format!(
                            "aggregator's value underflowed when subtracting {} from {}",
                            value, self.value
                        ),
                        EAGGREGATOR_UNDERFLOW,
                    )),
                }
            }
            // Since `sub` is a barrier, we never encounter delta during
            // subtraction. This will change in future.
            // TODO: implement this properly once `sub` stops being a barrier.
            AggregatorState::PositiveDelta => {
                unreachable!("subtraction always materializes the value")
            }
        }
    }

    /// Materializes the value of the aggregator, changing its state.
    fn materialize(
        &mut self,
        context: &NativeAggregatorContext,
        handle: &TableHandle,
    ) -> PartialVMResult<()> {
        // If aggregator has already been materialized, return immediately.
        if self.state == AggregatorState::Data {
            return Ok(());
        }

        // Otherwise, we have a delta and have to go to storage.
        let key_bytes = serialize(&self.key);
        match context.resolver.resolve_table_entry(handle, &key_bytes) {
            Err(_) => Err(extension_error("error resolving aggregator's value")),
            Ok(maybe_bytes) => {
                match maybe_bytes {
                    Some(bytes) => {
                        let base = deserialize(&bytes);

                        // At this point, we fetched the "true" value of the
                        // aggregator from the previously executed transaction.
                        match self.state {
                            AggregatorState::PositiveDelta => {
                                match addition(base, self.value, self.limit) {
                                    Some(result) => {
                                        self.value = result;
                                        self.state = AggregatorState::Data;
                                        Ok(())
                                    }
                                    None => Err(abort_error(
                                        "aggregator's value overflowed when materializing the delta",
                                        EAGGREGATOR_OVERFLOW,
                                    )),
                                }
                            }
                            AggregatorState::Data => {
                                unreachable!("aggregator's value is already materialized")
                            }
                        }
                    }
                    None => Err(extension_error("aggregator's value not found")),
                }
            }
        }
    }
}

/// Aggregator table - a top-level collection storing aggregators. It holds an
/// inner table data structure to actually store the aggregators.
struct AggregatorTable {
    // All aggregators created with this table.
    new_aggregators: BTreeSet<u128>,
    // All aggregators removed from this table.
    removed_aggregators: BTreeSet<u128>,
    // Table to store all agregators associated with this `AggregatorTable`.
    table: BTreeMap<u128, Aggregator>,
}

impl AggregatorTable {
    /// Returns a possibly new aggregator instance. If transaction did not
    /// initialize this aggregator, then the actual value is not known, and the
    /// aggregator state must be a delta, zero-intialized.
    fn get_or_create_aggregator(&mut self, key: u128, limit: u128) -> &mut Aggregator {
        if !self.table.contains_key(&key) {
            let aggregator = Aggregator {
                key,
                state: AggregatorState::PositiveDelta,
                value: 0,
                limit,
            };
            self.table.insert(key, aggregator);
        }
        self.table.get_mut(&key).unwrap()
    }

    fn num_aggregators(&self) -> u128 {
        self.table.len() as u128
    }

    /// Creates and inserts into table a new Aggregator with given `key` and
    /// `limit`. Since this transaction initializes aggregator, then the actual
    /// value is known, and the aggregator state must be `Data`. Also,
    /// aggregators are zero-initialized.
    fn create_new_aggregator(&mut self, key: u128, limit: u128) {
        let aggregator = Aggregator {
            key,
            state: AggregatorState::Data,
            value: 0,
            limit,
        };
        self.table.insert(key, aggregator);
        self.new_aggregators.insert(key);
    }

    /// Removes an aggregator.
    fn remove_aggregator(&mut self, key: u128) {
        // Aggregator no longer in use during this context, so remove it
        // from the table if it was there.
        self.table.remove(&key);

        if self.new_aggregators.contains(&key) {
            // Aggregator has been created by the same transaction. Therefore,
            // no side-effects.
            self.new_aggregators.remove(&key);
        } else {
            // Otherwise, aggregator has been created somewhere else.
            self.removed_aggregators.insert(key);
        }
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
                new_aggregators: BTreeSet::new(),
                removed_aggregators: BTreeSet::new(),
                table: BTreeMap::new(),
            };
            e.insert(aggregator_table);
        }
        self.aggregator_tables.get_mut(&table_handle).unwrap()
    }
}

/// Enum to wrap a single aggregator change.
#[derive(Debug)]
pub enum AggregatorChange {
    Write(u128),
    Delta(DeltaOp),
}

/// Contains all changes during this VM session. The change set must be
/// traversed by VM and converted to appropriate write ops.

pub struct AggregatorChangeSet {
    pub changes: BTreeMap<TableHandle, BTreeMap<u128, Option<AggregatorChange>>>,
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

    /// Returns all changes made during this VM session.
    pub fn into_change_set(self) -> AggregatorChangeSet {
        let NativeAggregatorContext {
            aggregator_table_data,
            ..
        } = self;
        let AggregatorTableData { aggregator_tables } = aggregator_table_data.into_inner();

        let mut changes = BTreeMap::new();
        for (table_handle, aggregator_table) in aggregator_tables {
            let mut change = BTreeMap::new();
            let AggregatorTable {
                removed_aggregators,
                table,
                ..
            } = aggregator_table;

            // Add changes that update or create aggregators.
            for (key, aggregator) in table {
                let Aggregator {
                    state,
                    value,
                    limit,
                    ..
                } = aggregator;

                let op = match state {
                    AggregatorState::Data => AggregatorChange::Write(value),
                    AggregatorState::PositiveDelta => {
                        let delta_op = DeltaOp::Addition { value, limit };
                        AggregatorChange::Delta(delta_op)
                    }
                };
                change.insert(key, Some(op));
            }

            // Add changes that delete aggregators.
            for key in removed_aggregators {
                change.insert(key, None);
            }

            changes.insert(table_handle, change);
        }

        AggregatorChangeSet { changes }
    }
}

// ================================= Natives =================================

/// All aggregator native functions. For more details, refer to code in
/// `aggregator_table.move`.
pub fn aggregator_natives(aggregator_addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(
        aggregator_addr,
        &[
            ("aggregator", "add", native_add),
            ("aggregator", "read", native_read),
            ("aggregator", "remove_aggregator", native_remove_aggregator),
            ("aggregator", "sub", native_sub),
            ("aggregator_table", "new_aggregator", native_new_aggregator),
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
    // strategy from `table` implementation: taking hash of transaction and
    // number of aggregator instances created so far and truncating them to
    // 128 bits.
    let txn_hash_buffer = serialize(&aggregator_context.txn_hash);
    let num_aggregators_buffer = serialize(&aggregator_table.num_aggregators());

    let mut sha3 = Sha3::v256();
    sha3.update(&txn_hash_buffer);
    sha3.update(&num_aggregators_buffer);

    let mut key_bytes = [0_u8; 16];
    sha3.finalize(&mut key_bytes);
    let key = deserialize(&key_bytes.to_vec());

    aggregator_table.create_new_aggregator(key, limit);

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();

    // Return `Aggregator` Move struct to the user so that we can add/subtract.
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

    // Get aggregator fields and a value to subtract.
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

    // First, materialize the value.
    aggregator
        .materialize(aggregator_context, &table_handle)
        .and_then(|_| {
            // TODO: charge gas properly.
            let cost = GasCost::new(0, 0).total();

            // Value has been materialized, return it.
            Ok(NativeResult::ok(
                cost,
                smallvec![Value::u128(aggregator.value)],
            ))
        })
}

fn native_sub(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Get aggregator fields and a value to subtract.
    let value = pop_arg!(args, IntegerValue).value_as::<u128>()?;
    let aggregator_ref = pop_arg!(args, StructRef);
    let (table_handle, key, limit) = get_aggregator_fields(&aggregator_ref)?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_table_data = aggregator_context.aggregator_table_data.borrow_mut();

    let aggregator = aggregator_table_data
        .get_or_create_aggregator_table(table_handle)
        .get_or_create_aggregator(key, limit);

    // For V1 Aggregator, subtraction is a barrier, and materializes the value
    // first.
    aggregator
        .materialize(aggregator_context, &table_handle)
        .and_then(|_| {
            aggregator.sub(value).and_then(|_| {
                // TODO: charge gas properly.
                let cost = GasCost::new(0, 0).total();
                Ok(NativeResult::ok(cost, smallvec![]))
            })
        })
}

fn native_remove_aggregator(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Get table handle and aggregator key for removal.
    let key = pop_arg!(args, IntegerValue).value_as::<u128>()?;
    let table_handle = pop_arg!(args, IntegerValue)
        .value_as::<u128>()
        .map(TableHandle)?;

    // Get aggregator table.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_table_data = aggregator_context.aggregator_table_data.borrow_mut();
    let aggregator_table = aggregator_table_data.get_or_create_aggregator_table(table_handle);

    aggregator_table.remove_aggregator(key);

    // TODO: charge gas properly.
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
fn abort_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}

/// Returns partial VM error on extension failure.
fn extension_error(message: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(message.to_string())
}

// ================================= Tests =================================

mod test {
    use super::*;
    use claim::{assert_matches, assert_none};
    use move_deps::move_vm_test_utils::BlankStorage;
    use once_cell::sync::Lazy;

    static DUMMY_RESOLVER: Lazy<BlankStorage> = Lazy::new(|| BlankStorage);
    static DUMMY_HANDLE: Lazy<TableHandle> = Lazy::new(|| TableHandle(0));

    fn set_up(context: &NativeAggregatorContext) {
        let mut aggregator_table_data = context.aggregator_table_data.borrow_mut();
        let table = aggregator_table_data.get_or_create_aggregator_table(*DUMMY_HANDLE);

        // Aggregators with data.
        table.create_new_aggregator(0, 1000);
        table.create_new_aggregator(1, 1000);
        table.create_new_aggregator(2, 1000);

        // Aggregators with delta.
        table.get_or_create_aggregator(3, 1000);
        table.get_or_create_aggregator(4, 1000);
        table.get_or_create_aggregator(5, 10);

        // Different cases of agregator removal.
        table.remove_aggregator(0);
        table.remove_aggregator(3);
        table.remove_aggregator(6);
    }

    #[test]
    fn test_into_change_set() {
        let context = NativeAggregatorContext::new(0, &*DUMMY_RESOLVER);
        set_up(&context);

        let AggregatorChangeSet { changes } = context.into_change_set();

        let change = changes.get(&*DUMMY_HANDLE).unwrap();

        assert!(!change.contains_key(&0));

        assert_matches!(change.get(&1).unwrap(), Some(AggregatorChange::Write(0)));
        assert_matches!(change.get(&2).unwrap(), Some(AggregatorChange::Write(0)));

        assert_none!(change.get(&3).unwrap());

        assert_matches!(
            change.get(&4).unwrap(),
            Some(AggregatorChange::Delta(DeltaOp::Addition {
                value: 0,
                limit: 1000
            }))
        );
        assert_matches!(
            change.get(&5).unwrap(),
            Some(AggregatorChange::Delta(DeltaOp::Addition {
                value: 0,
                limit: 10
            }))
        );

        assert_none!(change.get(&6).unwrap());
    }
}
