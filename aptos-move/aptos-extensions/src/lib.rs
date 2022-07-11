// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use aptos_types::write_set::WriteOp;
use move_deps::{
    move_binary_format::errors::{PartialVMResult, PartialVMError},
    move_core_types::{
        account_address::AccountAddress,
        gas_schedule::GasCost,
        vm_status::StatusCode, value::MoveTypeLayout,
    },
    move_vm_runtime::{
        native_functions,
        native_functions::{NativeContext, NativeFunctionTable},
    },
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        pop_arg,
        values::{IntegerValue, Reference, StructRef, Value},
    },
};
use smallvec::smallvec;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use tiny_keccak::{Hasher, Sha3};

/// Specifies operations on aggregatable values: they can be added, subtracted
/// (both checking for over and underflow), and serialized. 
trait AggregatableValue: Sized + PartialOrd {
    fn add(&self, v: &Self) -> Option<Self>;

    fn sub(&self, v: &Self) -> Option<Self>;

    fn serialize(&self, layout: &MoveTypeLayout) -> Option<Vec<u8>>;
}

macro_rules! aggregatable_value_impl {
    ($t:ty, $method:ident) => {
        impl AggregatableValue for $t {
            #[inline]
            fn add(&self, v: &$t) -> Option<$t> {
                <$t>::checked_add(*self, *v)
            }

            #[inline]
            fn sub(&self, v: &$t) -> Option<$t> {
                <$t>::checked_sub(*self, *v)
            }

            #[inline]
            fn serialize(&self, layout: &MoveTypeLayout) -> Option<Vec<u8>> {
                Value::$method(*self).simple_serialize(layout)
            }
        }
    }
}

aggregatable_value_impl!(u8, u8);
aggregatable_value_impl!(u64, u64);
aggregatable_value_impl!(u128, u128);

/// State of aggregator. `Data` means that the aggregator's value is known,
/// `PositiveDelta` means that the actual value is not known, and all updates
/// must be accumulated together as deltas.
enum AggregatorState {
    Data,
    PositiveDelta,
}

/// Error code for aggregator's value overflow (both for data or delta).
const E_AGGREGATOR_OVERFLOW: u64 = 1500;

struct Aggregator<T: AggregatableValue> {
    state: AggregatorState,
    layout: MoveTypeLayout,
    value: T,
}

impl<T> Aggregator<T>
where 
    T: AggregatableValue
{
    /// Creates a new aggregator with given state and value.
    fn new(state: AggregatorState, layout: MoveTypeLayout, value: T) -> Self {
        Aggregator {
            state,
            layout,
            value,
        }
    }

    /// Adds `other_value` to currently aggregated value, aborting on overflow.
    fn add(&mut self, other_value: &T) -> PartialVMResult<()>{
        match self.state {
            AggregatorState::Data | AggregatorState::PositiveDelta => {
                let result = self.value.add(other_value);
                if result.is_some() {
                    self.value = result.unwrap();
                    Ok(())
                } else {
                    Err(partial_error(
                        "aggregator's value overflowed",
                        E_AGGREGATOR_OVERFLOW,
                    ))
                }

            }
        }
    }

    /// Converts aggregator into a write operation.
    fn into_write_op(self) -> PartialVMResult<WriteOp> {
        unimplemented!()
    }
}

/// Top-level wrapper to allow aggregators of different types.
enum AggregatorWrapper {
    AggregatorU8(Aggregator<u8>),
    AggregatorU64(Aggregator<u64>),
    AggregatorU128(Aggregator<u128>),
}

impl AggregatorWrapper {
    fn new(
        state: AggregatorState,
        layout: MoveTypeLayout,
        ty: &Type,
    ) -> Self {
        match ty {
            Type::U8 => AggregatorWrapper::AggregatorU8(Aggregator::new(state, layout, 0)),
            Type::U64 => AggregatorWrapper::AggregatorU64(Aggregator::new(state, layout, 0)),
            Type::U128 => AggregatorWrapper::AggregatorU128(Aggregator::new(state, layout, 0)),
            _ => unreachable!("aggregator uses only integer types"),
        }
    }

    fn add(&mut self, other: IntegerValue) -> PartialVMResult<()> {
        match (self, other) {
            (AggregatorWrapper::AggregatorU8(a), IntegerValue::U8(v)) => a.add(&v),
            (AggregatorWrapper::AggregatorU64(a), IntegerValue::U64(v)) => a.add(&v),
            (AggregatorWrapper::AggregatorU128(a), IntegerValue::U128(v)) => a.add(&v),
            _ => unreachable!("aggregator operates on a single type"),
        }
    }

    fn into_write_op(self) -> PartialVMResult<WriteOp> {
        match self {
            AggregatorWrapper::AggregatorU8(a) => a.into_write_op(),
            AggregatorWrapper::AggregatorU64(a) => a.into_write_op(),
            AggregatorWrapper::AggregatorU128(a) => a.into_write_op(),
        }
    }
}

/// Every aggregator instance is uniquely specified by 128-bit handle.
type AggregatorHandle = u128;
const HANDLE_FIELD_IDX: usize = 0;

/// Native context that can be passed to VM extensions.
#[derive(Tid)]
pub struct NativeAggregatorContext {
    txn_hash: u128,
    aggregators: RefCell<BTreeMap<AggregatorHandle, AggregatorWrapper>>,
}

impl NativeAggregatorContext {
    /// Creates new context.
    pub fn new(txn_hash: u128) -> Self {
        Self {
            // TODO: add resolver once we suppord read and need to go to
            // global storage!
            txn_hash,
            aggregators: RefCell::new(BTreeMap::new()),
        }
    }
}

// ================================= Natives =================================

/// All aggregator native functions. For more details, refer to Aggregator.move
/// code.
pub fn aggregator_natives(aggregator_addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(
        aggregator_addr, 
        &[
            ("Aggregator", "is_integral", native_is_integral),
            ("Aggregator", "new_handle", native_new_handle),
            ("Aggregator", "add", native_add),
        ],
    )
}

fn native_is_integral(
    _context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(ty_args.len() == 1);
    assert!(args.is_empty());

    let result = match ty_args[0] {
        Type::U8 | Type::U64 | Type::U128 => true,
        _ => false,
    };

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();

    Ok(NativeResult::ok(cost, smallvec![Value::bool(result)]))
}

fn native_new_handle(
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(ty_args.len() == 1);
    assert!(args.is_empty());

    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregators = aggregator_context.aggregators.borrow_mut();

    // Every aggregator instance has a unique handle. Here we can reuse the
    // strategy from Table implementation: taking hash of transaction and
    // number of aggregator instances created so far and truncating them to
    // 128 bits.
    let txn_hash_buffer = aggregator_context.txn_hash.to_be_bytes();
    let num_aggregators_buffer = aggregators.len().to_be_bytes();

    let mut sha3 = Sha3::v256();
    sha3.update(&txn_hash_buffer);
    sha3.update(&num_aggregators_buffer);

    let mut handle_bytes = [0_u8; 16];
    sha3.finalize(&mut handle_bytes);
    let handle = u128::from_be_bytes(handle_bytes);

    // If transaction initializes aggregator, then the actual value is known,
    // and Data(0) is produced.
    let int_ty = ty_args.pop().unwrap();
    let layout = context.type_to_type_layout(&int_ty).unwrap().unwrap();
    aggregators.insert(handle, AggregatorWrapper::new(AggregatorState::Data, layout, &int_ty));

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();

    Ok(NativeResult::ok(cost, smallvec![Value::u128(handle)]))
}

fn native_add(
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(ty_args.len() == 1);
    assert!(args.len() == 2);

    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregators = aggregator_context.aggregators.borrow_mut();

    let value = pop_arg!(args, IntegerValue);
    let handle = get_handle(&pop_arg!(args, StructRef))?;

    // If aggregator has not been initialized in this context, create one
    // with unkown value, i.e. with Delta(0).
    if !aggregators.contains_key(&handle) {
        let int_ty = ty_args.pop().unwrap();
        let layout = context.type_to_type_layout(&int_ty).unwrap().unwrap();
        aggregators.insert(handle, AggregatorWrapper::new(AggregatorState::PositiveDelta, layout, &int_ty));
    }

    // Add a value to aggregator, and check for errors. 
    let aggregator = aggregators.get_mut(&handle).unwrap();
    if let Err(err) = aggregator.add(value) {
        return Err(err);
    }

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();

    Ok(NativeResult::ok(cost, smallvec![]))
}

/// Returns 128-bit handle of Move aggregator struct reference.
fn get_handle(aggregator: &StructRef) -> PartialVMResult<AggregatorHandle> {
    let field_ref =
        aggregator
            .borrow_field(HANDLE_FIELD_IDX)?
            .value_as::<Reference>()?;
    field_ref.read_ref()?.value_as::<AggregatorHandle>()
}

fn partial_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}

// ================================== Tests ==================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_aggregator_data_overflow() {
        let int_ty = Type::U8;
        let mut a = AggregatorWrapper::new(AggregatorState::Data, MoveTypeLayout::U8, &int_ty);
        _ = a.add(IntegerValue::U8(200));
        let status = a.add(IntegerValue::U8(200));

        if status.is_err() {
            panic!("overflow")
        }
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_aggregator_delta_overflow() {
        let int_ty = Type::U8;
        let mut a = AggregatorWrapper::new(AggregatorState::PositiveDelta, MoveTypeLayout::U8, &int_ty);
        _ = a.add(IntegerValue::U8(200));
        let status = a.add(IntegerValue::U8(200));

        if status.is_err() {
            panic!("overflow")
        }
    }
}
