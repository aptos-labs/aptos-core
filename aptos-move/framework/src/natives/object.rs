// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress, gas_algebra::InternalGas, vm_status::StatusCode,
};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

/***************************************************************************************************
 * native exists_at<T>
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Clone, Debug)]
pub struct ExistsAtGasParameters {
    pub base_cost: InternalGas,
}

fn native_exists_at(
    gas_params: &ExistsAtGasParameters,
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(ty_args.len() == 1);
    assert!(args.len() == 1);

    let type_ = ty_args.pop().unwrap();
    let address = pop_arg!(args, AccountAddress);

    let exists = context.exists_at(address, &type_).map_err(|err| {
        PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
            "Failed to read resource: {:?} at {}. With error: {}",
            type_, address, err
        ))
    })?;

    Ok(NativeResult::ok(gas_params.base_cost, smallvec![
        Value::bool(exists)
    ]))
}

pub fn make_native_exists_at(gas_params: ExistsAtGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| native_exists_at(&gas_params, context, ty_args, args))
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub exists_at: ExistsAtGasParameters,
}

impl GasParameters {
    pub fn new(exists_at_base: InternalGas) -> Self {
        Self {
            exists_at: ExistsAtGasParameters {
                base_cost: exists_at_base,
            },
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [("exists_at", make_native_exists_at(gas_params.exists_at))];

    crate::natives::helpers::make_module_natives(natives)
}
