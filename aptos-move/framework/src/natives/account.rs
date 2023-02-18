// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::{
    make_safe_native, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use crate::safely_pop_arg;
use aptos_types::on_chain_config::TimedFeatures;
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

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
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    context.charge(gas_params.base)?;

    let bytes = safely_pop_arg!(arguments, Vec<u8>);
    let address = AccountAddress::from_bytes(bytes);
    if let Ok(address) = address {
        Ok(smallvec![Value::address(address)])
    } else {
        Err(SafeNativeError::Abort {
            abort_code: super::status::NFE_UNABLE_TO_PARSE_ADDRESS,
        })
    }
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
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    context.charge(gas_params.base)?;

    let address = safely_pop_arg!(arguments, AccountAddress);
    Ok(smallvec![Value::signer(address)])
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

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "create_address",
            make_safe_native(
                gas_params.create_address,
                timed_features.clone(),
                native_create_address,
            ),
        ),
        (
            "create_signer",
            make_safe_native(
                gas_params.create_signer,
                timed_features,
                native_create_signer,
            ),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
