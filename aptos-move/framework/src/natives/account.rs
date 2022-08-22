// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas},
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

/***************************************************************************************************
 * native fun create_address
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct CreateAddressGasParameters {
    pub base: InternalGas,
}

fn native_create_address(
    gas_params: &CreateAddressGasParameters,
    _context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let cost = gas_params.base;

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

pub fn make_native_create_address(gas_params: CreateAddressGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_create_address(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * native fun create_signer
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct CreateSignerGasParameters {
    pub base: InternalGas,
}

fn native_create_signer(
    gas_params: &CreateSignerGasParameters,
    _context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let address = pop_arg!(arguments, AccountAddress);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::signer(address)],
    ))
}

pub fn make_native_create_signer(gas_params: CreateSignerGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_create_signer(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub create_address: CreateAddressGasParameters,
    pub create_signer: CreateSignerGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "create_address",
            make_native_create_address(gas_params.create_address),
        ),
        (
            "create_signer",
            make_native_create_signer(gas_params.create_signer),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
