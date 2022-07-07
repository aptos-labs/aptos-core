// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use tiny_keccak::{Hasher, Sha3};

use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::{
        account_address::AccountAddress,
        gas_schedule::GasCost,
    },
    move_vm_runtime::{
        native_functions,
        native_functions::{NativeContext, NativeFunctionTable},
    },
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        values::{Value, IntegerValue},
    },
};
use smallvec::smallvec;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};

enum AggregatorValue {
    Data(IntegerValue),
    Delta(IntegerValue),
}

type AggregatorHandle = u128;

#[derive(Tid)]
pub struct NativeAggregatorContext {
    txn_hash: u128,
    handles: RefCell<BTreeMap<AggregatorHandle, AggregatorValue>>,
}

impl NativeAggregatorContext {
    pub fn new(txn_hash: u128) -> Self {
        Self {
            txn_hash,
            handles: RefCell::new(BTreeMap::new()),
        }
    }
}

pub fn aggregator_natives(aggregator_addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(
        aggregator_addr, 
        &[
            ("Aggregator", "is_integral", native_is_integral),
            ("Aggregator", "new_handle", native_new_handle),
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
        Type::U8 => true,
        Type::U64 => true,
        Type::U128 => true,
        _ => false
    };

    // Type check should not be charged for gas.
    let cost = GasCost::new(0, 0).total();
    Ok(NativeResult::ok(cost, smallvec![Value::bool(result)]))
}

fn native_new_handle(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(ty_args.len() == 1);
    assert!(args.is_empty());

    let context = context.extensions().get::<NativeAggregatorContext>();
    let mut handles = context.handles.borrow_mut();

    // Every Aggregator instance has a unique handle. Here we can reuse the
    // strategy from Table implementation: taking hash of transaction and
    // number of aggregator instances created so far and truncating
    let txn_hash_buffer = context.txn_hash.to_be_bytes();
    let num_handles_buffer = handles.len().to_be_bytes();

    let mut sha3 = Sha3::v256();
    sha3.update(&txn_hash_buffer);
    sha3.update(&num_handles_buffer);

    let mut handle_bytes = [0_u8; 16];
    sha3.finalize(&mut handle_bytes);
    let handle = u128::from_be_bytes(handle_bytes);

    // Aggregator is initialized as delta of 0.
    let zero = match ty_args[0] {
        Type::U8 => IntegerValue::U8(0),
        Type::U64 => IntegerValue::U64(0),
        Type::U128 => IntegerValue::U128(0),
        _ => unreachable!()
    };
    handles.insert(handle, AggregatorValue::Delta(zero));

    // TODO: set the cost accordingly!
    let cost = GasCost::new(0, 0).total();
    Ok(NativeResult::ok(cost, smallvec![Value::u128(handle)]))
}
