// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};

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
        values::Value,
    },
};
use smallvec::smallvec;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};

enum AggregatorValue {}

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
