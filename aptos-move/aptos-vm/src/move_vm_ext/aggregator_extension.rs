// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::vm_status::StatusCode;
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
    collections::{btree_map::Entry, BTreeMap, VecDeque},
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

/// State of an aggregator: `Data` means that aggregator stores an exact value,
/// `PositiveDelta` means that actual value is not known but must be added later,
/// `NegativeDelta` means that the value is also not known but the value should
/// be subtracted instead.
#[derive(Clone, Copy, PartialEq)]
pub enum AggregatorState {
    Data,
    PositiveDelta,
    NegativeDelta,
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

/// When `Aggregator`'s value goes below zero.
const EAGGREGATOR_UNDERFLOW: u64 = 1601;

impl Aggregator {
    /// Implements logic for adding to an aggregator.
    fn add(
        &mut self,
        context: &NativeAggregatorContext,
        handle: &TableHandle,
        value: u128,
    ) -> PartialVMResult<()> {
        // First, check if the current value is a negative delta. In this case,
        // we must materialize the value. For example, assume we hold X-10, with
        // X being the "true" value. When add(5) comes, the result is X-10+5 =
        // X-5, which is undefined for X < 6!
        if self.state == AggregatorState::NegativeDelta {
            if let Err(e) = self.materialize(context, handle) {
                return Err(e);
            }
        }

        // At this point, aggregator holds a positive delta or knows the value.
        // Hence, we can add, of course checking overflow condition.
        checked_add(self.value, value, self.limit).and_then(|result| {
            self.value = result;
            Ok(())
        })
    }

    /// Implements logic for subtracting from an aggregator.
    fn sub(
        &mut self,
        context: &NativeAggregatorContext,
        handle: &TableHandle,
        value: u128,
    ) -> PartialVMResult<()> {
        // First, we check if the value can be subtracted in theory, that is
        // if `value <= limit`. If not, we can immediately abort.
        if value > self.limit {
            return Err(abort_error(
                "aggregator's value underflowed",
                EAGGREGATOR_UNDERFLOW,
            ));
        }

        // We can subtract. Now consider each possible state of aggregator
        // separately.
        match self.state {
            AggregatorState::Data => {
                // Aggregator knows the value, therefore we can subtract
                // checking we don't drop below zero.
                checked_sub(self.value, value).and_then(|result| {
                    self.value = result;
                    Ok(())
                })
            }
            AggregatorState::NegativeDelta => {
                // If aggregator holds a negative delta, subtraction is
                // commutative because of monotonicity. Indeed, if we hold
                // X-10 and sub(2) comes, the result is X-10-2 = X-12 which
                // aborts if X-10 should have aborted.
                // Note that here we reuse `checeked_add`. the reason is that
                // since we never can have `result > limit`, we also can never
                // subtract more that `limit`.
                checked_add(self.value, value, self.limit).and_then(|result| {
                    self.value = result;
                    Ok(())
                })
            }
            AggregatorState::PositiveDelta => {
                // If aggregator holds a positive delta, we need to consider
                // three cases. First, the delta can be zero itself, and hence
                // it is allowed to become negative. (again, monotonicty means
                // commutativity). If this is the case, change to negative delta
                // of `value`.
                // Note: we must protect over edge case of X+0-0 keeping it to
                // be X+0.
                if self.value == 0 && value != 0 {
                    self.state = AggregatorState::NegativeDelta;
                    self.value = value;
                    return Ok(());
                }

                // Alternatively, we try to subtract the value from the delta, and
                // as long as we stay above zero it's ok. (e.g. going from X+10 to
                // X+2 preserves correctness).
                match checked_sub(self.value, value) {
                    Ok(result) => {
                        self.value = result;
                        Ok(())
                    }
                    Err(_) => {
                        // Subtraction from delta failed and we dropped below
                        // zero. Unlucky! We must materialize the value to
                        // check if the "true" value is large enough for doing
                        // subtraction. If not - abort.
                        self.materialize(context, handle).and_then(|_|{
                            checked_sub(self.value, value).and_then(|result| {
                                self.value = result;
                                Ok(())
                            })
                        })
                    }
                }
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
                        let true_value = deserialize(&bytes);

                        // At this point, we fetched the "true" value of the
                        // aggregator from the previously executed transaction.
                        let result = match self.state {
                            AggregatorState::PositiveDelta => {
                                // If we hold a positive delta, apply it to the
                                // "true" value, aborting on overflow. Also change
                                // the state since the value is now known.
                                checked_add(true_value, self.value, self.limit)
                            }
                            AggregatorState::NegativeDelta => {
                                // Otherwise, we must be holding a negative
                                // delta. Check if applying it to the "true"
                                // value does not cause underflow.
                                checked_sub(true_value, self.value)
                            }
                            AggregatorState::Data => unreachable!("aggregator's value is already materialized"),
                        };

                        // If no errors occurred after applying deltas, update
                        // the value and the state.
                        result.and_then(|result| {
                            self.value = result;
                            self.state = AggregatorState::Data;
                            Ok(())
                        })
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
    // Table to store all agregators associated with this `AggregatorTable`.
    inner: BTreeMap<u128, Aggregator>,
}

impl std::ops::Deref for AggregatorTable {
    type Target = BTreeMap<u128, Aggregator>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for AggregatorTable {
    fn deref_mut(&mut self) -> &mut BTreeMap<u128, Aggregator> {
        &mut self.inner
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
                inner: BTreeMap::new(),
            };
            e.insert(aggregator_table);
        }
        self.aggregator_tables.get_mut(&table_handle).unwrap()
    }
}

/// Contains all changes during this VM session. The change set must be
/// traversed by VM and converted to appropriate write ops.
pub struct AggregatorChangeSet {
    pub changes: BTreeMap<TableHandle, BTreeMap<Vec<u8>, Option<(AggregatorState, Vec<u8>)>>>,
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
        let NativeAggregatorContext { aggregator_table_data, .. } = self;
        let AggregatorTableData {
            aggregator_tables,
        } = aggregator_table_data.into_inner();

        let mut changes = BTreeMap::new();
        for (table_handle, aggregator_table) in aggregator_tables {

            let mut change = BTreeMap::new();
            let AggregatorTable {
                inner,
            } = aggregator_table;

            for (key, aggregator) in inner {
                let key_bytes = serialize(&key);

                let Aggregator {
                    state,
                    value,
                    mark,
                    ..
                } = aggregator;

                let content = match mark {
                    Mark::Deleted => None,
                    _ => {
                        let value_bytes = serialize(&value);
                        Some((state, value_bytes))
                    }
                };

                change.insert(key_bytes, content);
            }
            changes.insert(table_handle, change);
        }

        AggregatorChangeSet {
            changes,
        }
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
            ("Aggregator", "sub", native_sub),
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

    aggregator.add(aggregator_context, &table_handle, value).and_then(|_| {
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
    aggregator.materialize(aggregator_context, &table_handle).and_then(|_| {
        // TODO: charge gas properly.
        let cost = GasCost::new(0, 0).total();

        // Value has been materialized, return it.
        Ok(NativeResult::ok(cost, smallvec![Value::u128(aggregator.value)]))
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

    aggregator.sub(aggregator_context, &table_handle, value).and_then(|_| {
        // TODO: charge gas properly.
        let cost = GasCost::new(0, 0).total();
        Ok(NativeResult::ok(cost, smallvec![]))
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
fn abort_error(message: &str, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}

/// Returns partial VM error on extension failure.
fn extension_error(message: &str) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
        .with_message(message.to_string())
}

/// Serializes aggregator value. The function is public so that it can be used
/// by the executor.
pub fn serialize(value: &u128) -> Vec<u8> {
    bcs::to_bytes(value).expect("unexpected serialization error")
}

/// Deserializes aggregator value. The function is public so that it can be
/// used by the executor.
pub fn deserialize(value_bytes: &Vec<u8>) -> u128{
    bcs::from_bytes(value_bytes).expect("unexpected deserialization error")
}

/// Returns `base` + `value` or error if the result is greater than `limit`.
/// Function is public so that it can be used by executor.
pub fn checked_add(base: u128, value: u128, limit: u128) -> PartialVMResult<u128> {
    if value > limit - base {
        Err(abort_error(
            "aggregator's value overflowed",
            EAGGREGATOR_OVERFLOW,
        ))
    } else {
        Ok(base + value)
    }
}

/// Returns `base` - `value` or error if the result is smaller than zero.
/// Function is public so that it can be used by executor.
pub fn checked_sub(base: u128, value: u128) -> PartialVMResult<u128> {
    if value > base {
        Err(abort_error(
            "aggregator's value underflowed",
            EAGGREGATOR_UNDERFLOW,
        ))
    } else {
        Ok(base - value)
    }
}
