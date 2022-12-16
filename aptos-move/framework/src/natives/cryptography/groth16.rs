// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module is cool.

use crate::natives::cryptography::curves;
use crate::natives::util::make_native_from_func;
use bellman::domain::Scalar;
use bellman::groth16::{prepare_verifying_key, Proof, VerifyingKey};
use better_any::{Tid, TidAble};
use bls12_381::Bls12;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::{CachedStructIndex, Type};
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::{Reference, Struct, VMValueCast, VectorRef};
use move_vm_types::values::{StructRef, Value};
use smallvec::smallvec;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
}

fn verify_proof_internal(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let pid = pop_arg!(args, u8);
    let public_input_handles = pop_arg!(args, Vec<u8>);
    let proof_c_handle = pop_arg!(args, u8);
    let proof_b_handle = pop_arg!(args, u8);
    let proof_a_handle = pop_arg!(args, u8);
    let gamma_abc_g1_handles = pop_arg!(args, Vec<u8>);
    let vk_alpha_g1_handle = pop_arg!(args, u8);
    let vk_beta_g2_handle = pop_arg!(args, u8);
    let vk_gamma_g2_handle = pop_arg!(args, u8);
    let vk_delta_g2_handle = pop_arg!(args, u8);
    let accepted: bool = match pid {
        curves::PID_BLS12_381 => {
            //todo
            true
        }
        _ => todo!(),
    };

    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::bool(accepted)],
    ))
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![(
        "verify_proof_internal",
        make_native_from_func(gas_params.clone(), verify_proof_internal),
    )]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![]);

    crate::natives::helpers::make_module_natives(natives)
}
