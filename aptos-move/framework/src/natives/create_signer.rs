// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeResult},
    safely_pop_arg,
};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, sync::Arc};

/***************************************************************************************************
 * native fun create_signer
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
pub(crate) fn native_create_signer(
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
pub struct CreateSignerGasParameters {
    pub base: InternalGas,
}

pub fn make_all(
    gas_param: CreateSignerGasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "create_signer",
        make_safe_native(gas_param, timed_features, features, native_create_signer),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
