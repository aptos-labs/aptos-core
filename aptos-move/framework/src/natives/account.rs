// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::{account_address::AccountAddress, gas_schedule::GasCost},
    move_vm_runtime::native_functions::NativeContext,
    move_vm_types::{
        gas_schedule::NativeCostIndex,
        loaded_data::runtime_types::Type,
        natives::function::{native_gas, NativeResult},
        pop_arg,
        values::Value,
    },
};
use smallvec::smallvec;
use std::collections::VecDeque;

pub fn native_create_address(
    _context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let cost = GasCost::new(super::cost::APTOS_CREATE_ADDRESS, 1).total();
    let bytes = pop_arg!(arguments, Vec<u8>);
    let address = AccountAddress::from_bytes(bytes);
    if let Ok(address) = address {
        Ok(NativeResult::ok(cost, smallvec![Value::address(address)]))
    } else {
        Ok(NativeResult::err(
            cost,
            super::status::NFE_UNABLE_TO_PARSE_ADDRESS,
        ))
    }
}

pub fn native_create_signer(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let address = pop_arg!(arguments, AccountAddress);
    let cost = native_gas(context.cost_table(), NativeCostIndex::CREATE_SIGNER, 0);
    Ok(NativeResult::ok(cost, smallvec![Value::signer(address)]))
}
