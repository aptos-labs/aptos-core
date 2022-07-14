// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::vm_status::StatusCode;
use better_any::{Tid, TidAble};
use move_deps::{
    move_binary_format::errors::{PartialVMResult, PartialVMError},
    move_core_types::{
        account_address::AccountAddress,
        gas_schedule::GasCost
    },
    move_vm_runtime::{
        native_functions,
        native_functions::{NativeContext, NativeFunctionTable},
    },
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        values::{Value, Reference, StructRef, IntegerValue, Struct},
        pop_arg,
    },
    move_table_extension::{TableResolver, TableHandle},
};
use smallvec::smallvec;
use std::{
    collections::{VecDeque, BTreeMap, btree_map::Entry},
    cell::RefCell, convert::TryInto
};
use tiny_keccak::{Hasher, Sha3};


/// Internal type used for aggregation, currenlty set to u128 (maximum available).
type IntegerType = u128;

/// State of the aggregator: `Data` means that aggregator stores an exact value,
/// `PositiveDelta` means that actual value is not known but must be added later.
#[derive(Clone, Copy, PartialEq)]
enum AggregatorState {
    Data,
    PositiveDelta,
}

/// Internal aggregator data structure.
struct Aggregator {
    // Identifies aggregator in registry.
    key: u128,
    // Describes state of aggregator: data or delta.
    state: AggregatorState,
    // Describes the value of data or delta.
    value: IntegerType,
    // Condition value, exceeding which aggregator overflows.
    limit: IntegerType,
}

/// If `Aggregator` overflows the `limit`.
const E_AGGREGATOR_OVERFLOW: u64 = 1600;

/// If `Aggregator` fails to resolve its value from storage.
const E_AGGREGATOR_RESOLVE_FAILURE: u64 = 1601;

/// If `Aggregator` cannot find its value when resolving from storage.
const E_AGGREGATOR_VALUE_NOT_FOUND: u64 = 1602;

impl Aggregator {
    /// Tries to add a `value` to the aggregator, aborting on exceeding the
    /// `limit`.
    fn add(&mut self, value: IntegerType) -> PartialVMResult<()> {
        match self.state {
            AggregatorState::Data | AggregatorState::PositiveDelta => {
                if value > self.limit - self.value {
                    Err(partial_error(
                        "aggregator's value overflowed",
                        E_AGGREGATOR_OVERFLOW,
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
    ) -> PartialVMResult<IntegerType> {
        // Aggregator knows what it has, return immediately.
        if self.state == AggregatorState::Data {
            return Ok(self.value);
        }

        // Otherwise, we have a delta and have to go to storage.
        let key_bytes = u128::to_be_bytes(key);
        match context
                .resolver
                .resolve_table_entry(handle, &key_bytes) {
            Err(_) => Err(partial_error(
                        "error resolving aggregator's value",
                        E_AGGREGATOR_RESOLVE_FAILURE,
                      )),
            Ok(maybe_bytes) => {
                match maybe_bytes {
                    Some(bytes) => {
                        // If the value is found, deserialize it back ensuring
                        // all bytes are preserved.
                        const VALUE_SIZE: usize = std::mem::size_of::<IntegerType>();
                        if bytes.len() < VALUE_SIZE {
                            return Err(partial_error(
                                "error resolving aggregator's value",
                                E_AGGREGATOR_RESOLVE_FAILURE,
                            ));
                        };
                        let value = IntegerType::from_be_bytes(bytes[0..VALUE_SIZE]
                            .try_into()
                            .expect("not enough bytes"));
                        
                        // Once value is reconstructed, apply delta to it and change the state.
                        self.state = AggregatorState::Data;
                        self.value += value;

                        Ok(self.value)
                    }
                    None => Err(partial_error(
                        "aggregator's value not found",
                        E_AGGREGATOR_VALUE_NOT_FOUND,
                    )),
                }
            }
        }
    }
}

/// Aggregator registry - a collection of aggregators, whose values are stored
/// in a `Table`.
struct Registry {
    // Identifies the `Table` of this registry.
    table_handle: TableHandle,
    // Table to store all agregators associated with this registry.
    table: BTreeMap<u128, Aggregator>,
}

/// Top-level struct that is used by the context to maintain the map of all
/// registries created throughout the VM session.
#[derive(Default)]
struct RegistryData {
    registries: BTreeMap<TableHandle, Registry>,
}

impl RegistryData {
    /// Returns a mutable reference to the `Registry` specified by its handle.
    /// A new registry is created if it doesn't exist in this context.
    fn get_or_create_registry_mut(
        &mut self,
        table_handle: TableHandle,
    ) -> &mut Registry {
        if let Entry::Vacant(e) = self.registries.entry(table_handle) {
            let registry = Registry {
                table_handle,
                table: BTreeMap::new(),
            };
            e.insert(registry);
        }
        self.registries.get_mut(&table_handle).unwrap()
    }
}

/// Native context that can be attached to VM NativeContextExtensions.
#[derive(Tid)]
pub struct NativeAggregatorContext<'a> {
    // Reuse table resolver since aggregator values are stored as Table values
    // internally. 
    resolver: &'a dyn TableResolver,
    txn_hash: u128,
    // All existing registries in this context, wrapped in RefCell for internal
    // mutability.
    registry_data: RefCell<RegistryData>,
}

impl<'a> NativeAggregatorContext<'a> {
    /// Creates a new instance of a native aggregator context. This must be
    /// passed into VM session.
    pub fn new(txn_hash: u128, resolver: &'a dyn TableResolver) -> Self {
        Self {
            resolver,
            txn_hash,
            registry_data: Default::default(),
        }
    }
}

// ================================= Natives =================================

/// All aggregator native functions. For more details, refer to cod in 
/// AggregatorRegistry.move.
pub fn aggregator_natives(aggregator_addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(
        aggregator_addr, 
        &[
            ("Aggregator", "add", native_add),
            ("Aggregator", "new", native_new),
            ("Aggregator", "read", native_read),
        ],
    )
}

fn native_add(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Get aggregator fields and a value to add.
    let value = pop_arg!(args, IntegerValue).value_as::<IntegerType>()?;
    let aggregator_ref = pop_arg!(args, StructRef);
    let (table_handle, key, limit) = get_aggregator_fields(&aggregator_ref)?;

    // Get the current registry. 
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut registry_data = aggregator_context.registry_data.borrow_mut();
    let registry = registry_data.get_or_create_registry_mut(table_handle);
    
    // If transaction did not initialize this aggregator, then the actual value
    // is not known, and the aggregator state must be a delta, zero-intialized.
    if !registry.table.contains_key(&key) {
        let aggregator = Aggregator {
            key,
            state: AggregatorState::PositiveDelta,
            value: 0,
            limit,
        };
        registry.table.insert(key, aggregator);
    };
    let aggregator = registry.table.get_mut(&key).unwrap();

    // Try to add a value.
    aggregator.add(value).and_then(|_| {
        // TODO: charge gas properly.
        let cost = GasCost::new(0, 0).total();
        Ok(NativeResult::ok(cost, smallvec![]))
    })
}

fn native_new(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Extract fields: `limit` of the new aggregator and a `table_handle`
    // associated with the associated registry. 
    let limit = pop_arg!(args, IntegerValue).value_as::<IntegerType>()?;
    let table_handle = get_table_handle(&pop_arg!(args, StructRef))?;

    // Get the current registry.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut registry_data = aggregator_context.registry_data.borrow_mut();
    let registry = registry_data.get_or_create_registry_mut(table_handle);
    
    // Every aggregator instance has a unique key. Here we can reuse the
    // strategy from `Table` implementation: taking hash of transaction and
    // number of aggregator instances created so far and truncating them to
    // 128 bits.
    let txn_hash_buffer = aggregator_context.txn_hash.to_be_bytes();
    let num_aggregators_buffer = registry.table.len().to_be_bytes();

    let mut sha3 = Sha3::v256();
    sha3.update(&txn_hash_buffer);
    sha3.update(&num_aggregators_buffer);

    let mut key_bytes = [0_u8; 16];
    sha3.finalize(&mut key_bytes);
    let key = u128::from_be_bytes(key_bytes);

    // If transaction initializes aggregator, then the actual value is known,
    // and the aggregator state must be Data. Also, aggregators are
    // zero-initialized.
    let aggregator = Aggregator {
        key,
        state: AggregatorState::Data,
        value: 0,
        limit,
    };
    registry.table.insert(key, aggregator);

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();

    // Return `Aggregator` Move struct to the user.
    Ok(NativeResult::ok(
        cost, 
        smallvec![
            Value::struct_(Struct::pack(vec![
                Value::u128(table_handle.0),
                Value::u128(key),
                Value::u128(limit),
            ]))
        ]
    ))
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

    // Get the current registry.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut registry_data = aggregator_context.registry_data.borrow_mut();
    let registry = registry_data.get_or_create_registry_mut(table_handle);

    // First, check if aggregator has been used during this VM session. If not,
    // create one with a zero-itiailized delta.
    if !registry.table.contains_key(&key) {
        let aggregator = Aggregator {
            key,
            state: AggregatorState::PositiveDelta,
            value: 0,
            limit,
        };
        registry.table.insert(key, aggregator);
    };
    let aggregator = registry.table.get_mut(&key).unwrap();

    // Try to read its value, possibly getting it from the storage and applying
    // delta.
    aggregator.read_value(aggregator_context, &table_handle, key).and_then(|result| {
        // TODO: charge gas properly.
        let cost = GasCost::new(0, 0).total();
        Ok(NativeResult::ok(cost, smallvec![Value::u128(result)]))
    })
}

// ================================ Utilities ================================

/// The field index of the `table` field in the `AggregatorRegistry` Move
/// struct.
const TABLE_FIELD_INDEX: usize = 0;

/// The field index of the `handle` field in the `Table` Move struct.
const TABLE_HANDLE_FIELD_INDEX: usize = 0;

/// Field indices of `registry_handle`, `key` and `limit` fields in the
/// `Aggregator` Move struct.
const REGISTRY_HANDLE_FIELD_INDEX: usize = 0;
const KEY_FIELD_INDEX: usize = 1;
const LIMIT_FIELD_INDEX: usize = 2;

/// Given a reference to `AggregatorRegistry` Move struct, returns `TableHandle`
/// (`RegistryHandle`) field value.
fn get_table_handle(aggregator_registry: &StructRef) -> PartialVMResult<TableHandle> {
    let table_ref =
        aggregator_registry
            .borrow_field(TABLE_FIELD_INDEX)?
            .value_as::<StructRef>()?;
    let handle_ref =
        table_ref
            .borrow_field(TABLE_HANDLE_FIELD_INDEX)?
            .value_as::<Reference>()?;
    handle_ref.read_ref()?.value_as::<u128>().map(TableHandle)
}

///  Given a reference to `Aggregator` Move struct and field index, returns
///  its field.
fn get_aggregator_field(aggregator: &StructRef, index: usize) -> PartialVMResult<Value> {
    let field_ref =
        aggregator
            .borrow_field(index)?
            .value_as::<Reference>()?;
    field_ref.read_ref()
}

///  Given a reference to `Aggregator` Move struct, returns a tuple of its
///  fields: (`table_handle`, `key`, `limit`).
fn get_aggregator_fields(
    aggregator: &StructRef
) -> PartialVMResult<(TableHandle, u128, IntegerType)> {
    let table_handle = get_aggregator_field(aggregator, REGISTRY_HANDLE_FIELD_INDEX)?.value_as::<u128>().map(TableHandle)?;
    let key = get_aggregator_field(aggregator, KEY_FIELD_INDEX)?.value_as::<u128>()?;
    let limit = get_aggregator_field(aggregator, LIMIT_FIELD_INDEX)?.value_as::<IntegerType>()?;
    Ok((table_handle, key, limit))
}

/// Returns a custom partial VM error with message and code.
fn partial_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}
