// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::natives::create_signer;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

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
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub create_address: CreateAddressGasParameters,
    pub create_signer: create_signer::CreateSignerGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "create_address",
            make_native_create_address(gas_params.create_address),
        ),
        // Despite that this is no longer present in account.move, we must keep this around for
        // replays.
        (
            "create_signer",
            create_signer::make_native_create_signer(gas_params.create_signer),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
