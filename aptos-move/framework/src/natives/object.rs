// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeResult},
    safely_assert_eq, safely_pop_arg,
};
use aptos_types::on_chain_config::{Features, TimedFeatures};
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
use std::{collections::VecDeque, sync::Arc};

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
    gas_params: &ExistsAtGasParameters,
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 1);
    safely_assert_eq!(args.len(), 1);

    let type_ = ty_args.pop().unwrap();
    let address = safely_pop_arg!(args, AccountAddress);

    context.charge(gas_params.base)?;

    let (exists, num_bytes) = context.exists_at(address, &type_).map_err(|err| {
        PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(format!(
            "Failed to read resource: {:?} at {}. With error: {}",
            type_, address, err
        ))
    })?;

    if let Some(num_bytes) = num_bytes {
        context.charge(gas_params.per_item_loaded + num_bytes * gas_params.per_byte_loaded)?;
    }

    Ok(smallvec![Value::bool(exists)])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub exists_at: ExistsAtGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "exists_at",
        make_safe_native(
            gas_params.exists_at,
            timed_features,
            features,
            native_exists_at,
        ),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
