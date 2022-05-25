// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::{
        gas_schedule::{GasAlgebra, GasCost},
        vm_status::sub_status::NFE_BCS_SERIALIZATION_FAILURE,
    },
    move_vm_runtime::native_functions::NativeContext,
    move_vm_types::{
        gas_schedule::NativeCostIndex,
        loaded_data::runtime_types::Type,
        natives::function::{native_gas, NativeResult},
        pop_arg,
        values::{values_impl::Reference, Value},
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, hash::Hasher};

/// Serialize the MoveValue with BCS and then feed the bytes into SipHasher. This is not
/// cryptographically secure.
pub fn native_sip_hash(
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    let ref_to_val = pop_arg!(args, Reference);

    let arg_type = ty_args.pop().unwrap();
    // delegate to the BCS serialization for `Value`
    let serialized_value_opt = match context.type_to_type_layout(&arg_type)? {
        None => None,
        Some(layout) => ref_to_val.read_ref()?.simple_serialize(&layout),
    };
    let serialized_value = match serialized_value_opt {
        None => {
            let cost = native_gas(context.cost_table(), NativeCostIndex::BCS_TO_BYTES, 1);
            return Ok(NativeResult::err(cost, NFE_BCS_SERIALIZATION_FAILURE));
        }
        Some(serialized_value) => serialized_value,
    };

    // cost is proportional to the size of the serialized value
    let bcs_cost = native_gas(
        context.cost_table(),
        NativeCostIndex::BCS_TO_BYTES,
        serialized_value.len(),
    );
    // SipHash of the serialized bytes
    let mut hasher = siphasher::sip::SipHasher::new();
    hasher.write(&serialized_value);
    let hash = hasher.finish();

    let cost = bcs_cost.add(GasCost::new(super::cost::APTOS_SIP_HASH, 1).total());

    Ok(NativeResult::ok(cost, smallvec![Value::u64(hash)]))
}
