// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use move_deps::{
    move_binary_format::errors::{PartialVMResult, PartialVMError},
    move_core_types::{
        account_address::AccountAddress,
        gas_schedule::GasCost,
        vm_status::StatusCode,
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

trait CheckedAdd: Sized {
    fn checked_add(&self, v: &Self) -> Option<Self>;
}

impl CheckedAdd for u8 {
    #[inline]
    fn checked_add(&self, v: &u8) -> Option<u8> {
        u8::checked_add(*self, *v)
    }
}

impl CheckedAdd for u64 {
    #[inline]
    fn checked_add(&self, v: &u64) -> Option<u64> {
        u64::checked_add(*self, *v)
    }
}

impl CheckedAdd for u128 {
    #[inline]
    fn checked_add(&self, v: &u128) -> Option<u128> {
        u128::checked_add(*self, *v)
    }
}

/// Error code for overflow.
const E_OVERFLOW: u64 = 1500;

/// Value of a single aggregator instance. `Data(val)` means that the value is
/// known and equals to `val`, `PositiveDelta(val)` means that the actual value
/// is not known and `val` should be added. `NegativeDelta(val)` is similar but
/// `val` should be subtracted instead.
enum AggregatorValue<T: CheckedAdd + PartialOrd> {
    Data(T),
    NegativeDelta(T),
    PositiveDelta(T),
}

impl<T> AggregatorValue<T>
where
    T: CheckedAdd + PartialOrd
{
    /// Returns a new aggrerated value or error on overflow.
    fn checked_add(&mut self, v: &T) -> PartialVMResult<()> {
        *self = match self {
            AggregatorValue::Data(u) => {
                let result = u.checked_add(v);
                if result.is_some() {
                    AggregatorValue::Data(result.unwrap())
                } else {
                    return Err(partial_error(
                        "aggregator data overflowed",
                        E_OVERFLOW,
                    ));
                }
            }
            AggregatorValue::PositiveDelta(u) => {
                let result = u.checked_add(v);
                if result.is_some() {
                    AggregatorValue::PositiveDelta(result.unwrap())
                } else {
                    return Err(partial_error(
                        "aggregator delta overflowed",
                        E_OVERFLOW,
                    ));
                }
            }
            _ => unimplemented!() 
        };
        Ok(())
    }
}

/// Typed aggregator instance.
enum Aggregator {
    AggregatorU8(AggregatorValue<u8>),
    AggregatorU64(AggregatorValue<u64>),
    AggregatorU128(AggregatorValue<u128>),
}

impl Aggregator {
    /// Creates aggregator with a known value, initialized to 0.
    fn new_known(ty: Type) -> Self {
        match ty {
            Type::U8 => Aggregator::AggregatorU8(AggregatorValue::Data(0)),
            Type::U64 => Aggregator::AggregatorU64(AggregatorValue::Data(0)),
            Type::U128 => Aggregator::AggregatorU128(AggregatorValue::Data(0)),
            _ => unreachable!(),
        }
    }

    /// Creates aggregator with a unknown value.
    fn new_unknown(ty: Type) -> Self {
        match ty {
            Type::U8 => Aggregator::AggregatorU8(AggregatorValue::PositiveDelta(0)),
            Type::U64 => Aggregator::AggregatorU64(AggregatorValue::PositiveDelta(0)),
            Type::U128 => Aggregator::AggregatorU128(AggregatorValue::PositiveDelta(0)),
            _ => unreachable!(),
        }
    }

    /// Adds a value to aggregator.
    fn add(&mut self, value: IntegerValue) -> PartialVMResult<()> {
        match (self, value) {
            (Aggregator::AggregatorU8(a), IntegerValue::U8(v)) => a.checked_add(&v),
            (Aggregator::AggregatorU64(a), IntegerValue::U64(v)) => a.checked_add(&v),
            (Aggregator::AggregatorU128(a), IntegerValue::U128(v)) => a.checked_add(&v),
            _ => unreachable!()
        }
    }
}

/// Every aggregator instance is uniquely specified by 128-bit handle.
type AggregatorHandle = u128;
const HANDLE_FIELD_IDX: usize = 0;

/// Native context of aggregator that can be passed to VM extensions.
/// Aggregators are wrapped in RefCell for internal mutability.
#[derive(Tid)]
pub struct NativeAggregatorContext {
    txn_hash: u128,
    aggregators: RefCell<BTreeMap<AggregatorHandle, Aggregator>>,
}

impl NativeAggregatorContext {
    /// Creates new aggregator context.
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

    // Every Aggregator instance has a unique handle. Here we can reuse the
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
    aggregators.insert(handle, Aggregator::new_known(int_ty));

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
        aggregators.insert(handle, Aggregator::new_unknown(int_ty));
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
        let mut a = Aggregator::new_known(Type::U8);
        _ = a.add(IntegerValue::U8(200));
        let status = a.add(IntegerValue::U8(200));

        if status.is_err() {
            panic!("overflow")
        }
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_aggregator_delta_overflow() {
        let mut a = Aggregator::new_unknown(Type::U8);
        _ = a.add(IntegerValue::U8(200));
        let status = a.add(IntegerValue::U8(200));

        if status.is_err() {
            panic!("overflow")
        }
    }

    #[test]
    fn test_aggregator_can_add() {
        let mut a_unkwon = Aggregator::new_unknown(Type::U8);
        _ = a_unkwon.add(IntegerValue::U8(1));
        let status = a_unkwon.add(IntegerValue::U8(2));
        assert!(status.is_ok());

        // TODO: what would be a better way to test this?
        if let Aggregator::AggregatorU8(AggregatorValue::PositiveDelta(v)) = a_unkwon {
            assert!(v == 3)
        }
    }
}
