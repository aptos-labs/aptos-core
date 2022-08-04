// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::vm_status::sub_status::NFE_BCS_SERIALIZATION_FAILURE,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        pop_arg,
        values::{values_impl::Reference, Value},
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, hash::Hasher, sync::Arc};

/***************************************************************************************************
 * native fun sip_hash
 *
 *   gas cost: base_cost + unit_cost * data_length
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct SipHashGasParameters {
    pub base_cost: u64,
    pub unit_cost: u64,
}

/// Serialize the MoveValue with BCS and then feed the bytes into SipHasher. This is not
/// cryptographically secure.
fn native_sip_hash(
    _gas_params: &SipHashGasParameters,
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    // TODO(Gas): proper gas metering

    let ref_to_val = pop_arg!(args, Reference);
    let arg_type = ty_args.pop().unwrap();

    // delegate to the BCS serialization for `Value`
    let serialized_value_opt = match context.type_to_type_layout(&arg_type)? {
        None => None,
        Some(layout) => ref_to_val.read_ref()?.simple_serialize(&layout),
    };
    let serialized_value = match serialized_value_opt {
        None => {
            return Ok(NativeResult::err(0, NFE_BCS_SERIALIZATION_FAILURE));
        }
        Some(serialized_value) => serialized_value,
    };

    // SipHash of the serialized bytes
    let mut hasher = siphasher::sip::SipHasher::new();
    hasher.write(&serialized_value);
    let hash = hasher.finish();

    Ok(NativeResult::ok(0, smallvec![Value::u64(hash)]))
}

pub fn make_native_sip_hash(gas_params: SipHashGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| native_sip_hash(&gas_params, context, ty_args, args))
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub sip_hash: SipHashGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [("sip_hash", make_native_sip_hash(gas_params.sip_hash))];

    crate::natives::helpers::make_module_natives(natives)
}
