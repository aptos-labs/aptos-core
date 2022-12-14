// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module is cool.

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

fn new_verifying_key_from_bytes(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let bytes = pop_arg!(arguments, Vec<u8>);
    let vk = VerifyingKey::read(bytes.as_slice()).unwrap();
    let handle = _context
        .extensions_mut()
        .get_mut::<BellmanContext>()
        .add_vk(vk);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u64(handle as u64)],
    ))
}

fn new_proof_from_bytes(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let bytes = pop_arg!(arguments, Vec<u8>);
    let proof = Proof::read(bytes.as_slice()).unwrap();
    let handle = _context
        .extensions_mut()
        .get_mut::<BellmanContext>()
        .add_proof(proof);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u64(handle as u64)],
    ))
}

fn new_scalar_from_bytes(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let bytes = pop_arg!(arguments, Vec<u8>);
    let bytes_2 = <[u8; 32]>::try_from(bytes).unwrap();
    let scalar = bls12_381::Scalar::from_bytes(&bytes_2).unwrap();
    let handle = _context
        .extensions_mut()
        .get_mut::<BellmanContext>()
        .add_scalar(scalar);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u64(handle as u64)],
    ))
}

// fn ref_vector_u8_to_bytes(r: VectorRef) -> Vec<u8> {
//     let byte_count = r.len(&Type::U8).unwrap().value_as::<u64>().unwrap() as usize;
//     let mut bytes = Vec::with_capacity(byte_count);
//     for i in 0..byte_count {
//         let y = r.borrow_elem(i, &Type::U8).unwrap();
//         let z = y.value_as::<Reference>().unwrap();
//         let w = z.read_ref().unwrap().value_as::<u8>().unwrap();
//         bytes.push(w);
//     }
//     bytes
// }

#[derive(Tid)]
pub struct BellmanContext {
    proof_store: Vec<Proof<Bls12>>,
    vk_store: Vec<VerifyingKey<Bls12>>,
    scalar_store: Vec<bls12_381::Scalar>,
}

impl BellmanContext {
    pub fn add_scalar(&mut self, scalar: bls12_381::Scalar) -> usize {
        let ret = self.scalar_store.len();
        self.scalar_store.push(scalar);
        ret
    }

    pub fn borrow_scalar(&self, handle: usize) -> &bls12_381::Scalar {
        self.scalar_store.get(handle).unwrap()
    }

    pub fn add_proof(&mut self, proof: Proof<Bls12>) -> usize {
        let ret = self.proof_store.len();
        self.proof_store.push(proof);
        ret
    }

    pub fn borrow_proof(&self, handle: usize) -> &Proof<Bls12> {
        self.proof_store.get(handle).unwrap()
    }

    pub fn add_vk(&mut self, vk: VerifyingKey<Bls12>) -> usize {
        let ret = self.proof_store.len();
        self.vk_store.push(vk);
        ret
    }

    pub fn borrow_vk(&self, handle: usize) -> &VerifyingKey<Bls12> {
        self.vk_store.get(handle).unwrap()
    }
}

impl BellmanContext {
    pub fn new() -> Self {
        Self {
            proof_store: vec![],
            vk_store: vec![],
            scalar_store: vec![],
        }
    }
}

fn get_handle(r: StructRef) -> usize {
    r.borrow_field(0).unwrap().value_as::<u64>().unwrap() as usize
}

fn verify_proof(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut _arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let bellman_ctxt = _context.extensions().get::<BellmanContext>();
    let proof_handle = pop_arg!(_arguments, u64) as usize;
    let proof = bellman_ctxt.borrow_proof(proof_handle);

    let public_inputs_arg = pop_arg!(_arguments, Vec<u8>);
    let public_inputs: Vec<bls12_381::Scalar> = public_inputs_arg
        .into_iter()
        .map(|v| bellman_ctxt.borrow_scalar(v as usize).clone())
        .collect();

    let vk_handle = pop_arg!(_arguments, u64) as usize;
    let vk = bellman_ctxt.borrow_vk(vk_handle);
    let pvk = prepare_verifying_key(vk);

    let accepted = match bellman::groth16::verify_proof::<bls12_381::Bls12>(
        &pvk,
        proof,
        public_inputs.as_slice(),
    ) {
        Ok(()) => true,
        _ => false,
    };

    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::bool(accepted)],
    ))
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "new_verifying_key_from_bytes_internal",
            make_native_from_func(gas_params.clone(), new_verifying_key_from_bytes),
        ),
        (
            "new_proof_from_bytes_internal",
            make_native_from_func(gas_params.clone(), new_proof_from_bytes),
        ),
        (
            "new_scalar_from_bytes_internal",
            make_native_from_func(gas_params.clone(), new_scalar_from_bytes),
        ),
        (
            "verify_proof_internal",
            make_native_from_func(gas_params.clone(), verify_proof),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![]);

    crate::natives::helpers::make_module_natives(natives)
}
