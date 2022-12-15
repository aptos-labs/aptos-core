// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::groth16::BellmanContext;
use crate::natives::util::{make_native_from_func, make_test_only_native_from_func};
use aptos_crypto::bls12381::PrivateKey;
use better_any::{Tid, TidAble};
use bls12_381::{G1Affine, G1Projective};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::Struct;
use move_vm_types::values::Value;
use smallvec::smallvec;
use std::collections::VecDeque;
use std::ops::Mul;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
}

#[derive(Tid)]
pub struct Bls12381Context {
    scalar_store: Vec<bls12_381::Scalar>,
    g1_point_store: Vec<bls12_381::G1Projective>,
}

impl Bls12381Context {
    pub fn new() -> Self {
        Self {
            scalar_store: vec![],
            g1_point_store: vec![],
        }
    }

    pub fn add_scalar(&mut self, scalar: bls12_381::Scalar) -> usize {
        let ret = self.scalar_store.len();
        self.scalar_store.push(scalar);
        ret
    }

    pub fn get_scalar(&self, handle: usize) -> &bls12_381::Scalar {
        self.scalar_store.get(handle).unwrap()
    }

    pub fn add_point(&mut self, p0: G1Projective) -> usize {
        let ret = self.g1_point_store.len();
        self.g1_point_store.push(p0);
        ret
    }

    pub fn get_point(&self, handle: usize) -> &bls12_381::G1Projective {
        self.g1_point_store.get(handle).unwrap()
    }
}

fn scalar_one_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let handle = context
        .extensions_mut()
        .get_mut::<Bls12381Context>()
        .add_scalar(bls12_381::Scalar::one());
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u64(handle as u64)],
    ))
}

fn scalar_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    let scalar_1 = context
        .extensions()
        .get::<Bls12381Context>()
        .get_scalar(handle_1);
    let scalar_2 = context
        .extensions()
        .get::<Bls12381Context>()
        .get_scalar(handle_2);
    let result = scalar_1.add(scalar_2);
    let result_handle = context
        .extensions_mut()
        .get_mut::<Bls12381Context>()
        .add_scalar(result);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u64(result_handle as u64)],
    ))
}

fn point_identity_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let point = bls12_381::G1Projective::identity();
    let handle = context
        .extensions_mut()
        .get_mut::<Bls12381Context>()
        .add_point(point);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u64(handle as u64)],
    ))
}

fn point_eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    let point_1 = context
        .extensions()
        .get::<Bls12381Context>()
        .get_point(handle_1);
    let point_2 = context
        .extensions()
        .get::<Bls12381Context>()
        .get_point(handle_2);
    let result = point_1.eq(point_2);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::bool(result)],
    ))
}

fn point_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    let point_1 = context
        .extensions()
        .get::<Bls12381Context>()
        .get_point(handle_1);
    let point_2 = context
        .extensions()
        .get::<Bls12381Context>()
        .get_point(handle_2);
    let result = point_1.add(point_2);
    let handle = context
        .extensions_mut()
        .get_mut::<Bls12381Context>()
        .add_point(result);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u64(handle as u64)],
    ))
}

fn point_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let point_handle = pop_arg!(args, u64) as usize;
    let scalar_handle = pop_arg!(args, u64) as usize;
    let point = context
        .extensions()
        .get::<Bls12381Context>()
        .get_point(point_handle);
    let scalar = context
        .extensions()
        .get::<Bls12381Context>()
        .get_scalar(scalar_handle);
    let result = point.mul(scalar.clone());
    let handle = context
        .extensions_mut()
        .get_mut::<Bls12381Context>()
        .add_point(result);
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u64(handle as u64)],
    ))
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "scalar_one_internal",
            make_native_from_func(gas_params.clone(), scalar_one_internal),
        ),
        (
            "scalar_add_internal",
            make_native_from_func(gas_params.clone(), scalar_add_internal),
        ),
        (
            "point_identity_internal",
            make_native_from_func(gas_params.clone(), point_identity_internal),
        ),
        (
            "point_add_internal",
            make_native_from_func(gas_params.clone(), point_add_internal),
        ),
        (
            "point_mul_internal",
            make_native_from_func(gas_params.clone(), point_mul_internal),
        ),
        (
            "point_eq_internal",
            make_native_from_func(gas_params.clone(), point_eq_internal),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![]);

    crate::natives::helpers::make_module_natives(natives)
}
