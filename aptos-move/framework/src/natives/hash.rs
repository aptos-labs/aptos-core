// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes},
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
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
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

/// Feed thes bytes into SipHasher. This is not cryptographically secure.
fn native_sip_hash(
    gas_params: &SipHashGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);

    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);

    // SipHash of the serialized bytes
    let mut hasher = siphasher::sip::SipHasher::new();
    hasher.write(&bytes);
    let hash = hasher.finish();

    Ok(NativeResult::ok(cost, smallvec![Value::u64(hash)]))
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
