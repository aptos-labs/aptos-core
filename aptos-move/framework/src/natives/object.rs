// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeResult,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, InternalGasPerByte},
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::PartialVMError, values::Value,
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native exists_at<T>
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Clone, Debug)]
pub struct ExistsAtGasParameters {
    pub base: InternalGas,
    pub per_byte_loaded: InternalGasPerByte,
    pub per_item_loaded: InternalGas,
}

fn native_exists_at(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 1);
    safely_assert_eq!(args.len(), 1);

    let type_ = ty_args.pop().unwrap();
    let address = safely_pop_arg!(args, AccountAddress);

    context.charge(OBJECT_EXISTS_AT_BASE)?;

    let (exists, num_bytes) = context.exists_at(address, &type_).map_err(|err| {
        PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
            "Failed to read resource: {:?} at {}. With error: {}",
            type_, address, err
        ))
    })?;

    if let Some(num_bytes) = num_bytes {
        context.charge(
            OBJECT_EXISTS_AT_PER_ITEM_LOADED + OBJECT_EXISTS_AT_PER_BYTE_LOADED * num_bytes,
        )?;
    }

    Ok(smallvec![Value::bool(exists)])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("exists_at", native_exists_at as RawSafeNative)];

    builder.make_named_natives(natives)
}
