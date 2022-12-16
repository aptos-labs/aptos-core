// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::groth16_bls12381_bellman::BellmanContext;
use crate::natives::util::{make_native_from_func, make_test_only_native_from_func};
use crate::pop_vec_arg;
use aptos_crypto::bls12381::arithmetics::Scalar;
use aptos_crypto::bls12381::PrivateKey;
use better_any::{Tid, TidAble};
use bls12_381;
use group::{Group, GroupEncoding};
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
    g2_point_store: Vec<bls12_381::G2Projective>,
    gt_point_store: Vec<bls12_381::Gt>,
}

impl Bls12381Context {
    pub fn new() -> Self {
        Self {
            scalar_store: vec![],
            g1_point_store: vec![],
            g2_point_store: vec![],
            gt_point_store: vec![],
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

    pub fn add_g1_point(&mut self, p0: bls12_381::G1Projective) -> usize {
        let ret = self.g1_point_store.len();
        self.g1_point_store.push(p0);
        ret
    }

    pub fn get_g1_point(&self, handle: usize) -> &bls12_381::G1Projective {
        self.g1_point_store.get(handle).unwrap()
    }

    pub fn add_g2_point(&mut self, p0: bls12_381::G2Projective) -> usize {
        let ret = self.g2_point_store.len();
        self.g2_point_store.push(p0);
        ret
    }

    pub fn get_g2_point(&self, handle: usize) -> &bls12_381::G2Projective {
        self.g2_point_store.get(handle).unwrap()
    }

    pub fn add_gt_point(&mut self, point: bls12_381::Gt) -> usize {
        let ret = self.gt_point_store.len();
        self.gt_point_store.push(point);
        ret
    }

    pub fn get_gt_point(&self, handle: usize) -> &bls12_381::Gt {
        self.gt_point_store.get(handle).unwrap()
    }
}

fn bytes_into_point_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let group_id = pop_arg!(args, u8);
    let bytes = pop_arg!(args, Vec<u8>);
    let handle = match group_id {
        GID_BLS12_381_G1 => {
            let bytes_2 = <[u8; 48]>::try_from(bytes).unwrap();
            let point = bls12_381::G1Affine::from_compressed(&bytes_2)
                .unwrap()
                .mul(bls12_381::Scalar::one());
            context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(point)
        }
        GID_BLS12_381_G2 => {
            let bytes_2 = <[u8; 96]>::try_from(bytes.as_slice()).unwrap();
            let point = bls12_381::G2Affine::from_compressed(&bytes_2)
                .unwrap()
                .mul(bls12_381::Scalar::one());
            context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(point)
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn bytes_into_scalar_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let group_id = pop_arg!(args, u8);
    let bytes = pop_arg!(args, Vec<u8>);
    let handle = match group_id {
        GID_BLS12_381_G1 | GID_BLS12_381_G2 => {
            let scalar =
                bls12_381::Scalar::from_bytes(&<[u8; 32]>::try_from(bytes).unwrap()).unwrap();
            context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_scalar(scalar)
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn scalar_one_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let group_id = pop_arg!(args, u8);
    let handle = match group_id {
        GID_BLS12_381_G1 | GID_BLS12_381_G2 | GID_BLS12_381_Gt => {
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_scalar(bls12_381::Scalar::one());
            handle
        }
        _ => todo!(),
    };

    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn scalar_from_u64_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let group_id = pop_arg!(args, u8);
    let v = pop_arg!(args, u64);
    let handle = match group_id {
        GID_BLS12_381_G1 | GID_BLS12_381_G2 | GID_BLS12_381_Gt => {
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_scalar(bls12_381::Scalar::from(v));
            handle
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn scalar_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let group_id = pop_arg!(args, u8);
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let handle = match group_id {
        GID_BLS12_381_G1 | GID_BLS12_381_G2 | GID_BLS12_381_Gt => {
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
            result_handle
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn point_identity_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let group_id = pop_arg!(args, u8);
    let handle = match group_id {
        GID_BLS12_381_G1 => {
            let point = bls12_381::G1Projective::identity();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(point);
            handle
        }
        GID_BLS12_381_G2 => {
            let point = bls12_381::G2Projective::identity();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(point);
            handle
        }
        GID_BLS12_381_Gt => {
            let point = bls12_381::Gt::identity();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(point);
            handle
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

/// Group/bilinear mapping ID assignments.
/// The assignment here should match what is in `/aptos-move/framework/aptos-stdlib/sources/cryptography/curves.move`.
pub const GID_BLS12_381_G1: u8 = 1;
pub const GID_BLS12_381_G2: u8 = 2;
pub const GID_BLS12_381_Gt: u8 = 3;
pub const PID_BLS12_381: u8 = 1;

fn point_generator_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(0, _ty_args.len());
    assert_eq!(1, args.len());
    let group_id = pop_arg!(args, u8);
    let handle = match group_id {
        GID_BLS12_381_G1 => {
            let point = bls12_381::G1Projective::generator();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(point);
            handle
        }
        GID_BLS12_381_G2 => {
            let point = bls12_381::G2Projective::generator();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(point);
            handle
        }
        GID_BLS12_381_Gt => {
            let point = bls12_381::Gt::generator();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(point);
            handle
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn point_eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let group_id = pop_arg!(args, u8);
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let result = match group_id {
        GID_BLS12_381_G1 => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(handle_2);
            let result = point_1.eq(point_2);
            result
        }
        GID_BLS12_381_G2 => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(handle_2);
            let result = point_1.eq(point_2);
            result
        }
        GID_BLS12_381_Gt => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle_2);
            let result = point_1.eq(point_2);
            result
        }
        _ => todo!(),
    };

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
    let group_id = pop_arg!(args, u8);
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let handle = match group_id {
        GID_BLS12_381_G1 => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(handle_2);
            let result = point_1.add(point_2);
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(result);
            handle
        }
        GID_BLS12_381_G2 => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(handle_2);
            let result = point_1.add(point_2);
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(result);
            handle
        }
        GID_BLS12_381_Gt => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle_2);
            let result = point_1.clone() + point_2.clone();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(result);
            handle
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn point_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let group_id = pop_arg!(args, u8);
    let point_handle = pop_arg!(args, u8) as usize;
    let scalar_handle = pop_arg!(args, u8) as usize;
    let handle = match group_id {
        GID_BLS12_381_G1 => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(point_handle);
            let scalar = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(scalar_handle);
            let result = point.mul(scalar.clone());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(result);
            handle
        }
        GID_BLS12_381_G2 => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(point_handle);
            let scalar = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(scalar_handle);
            let result = point.mul(scalar.clone());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(result);
            handle
        }
        GID_BLS12_381_Gt => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(point_handle);
            let scalar = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(scalar_handle);
            let result = point.mul(scalar.clone());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(result);
            handle
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn pairing_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let pid = pop_arg!(args, u8);
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let handle = match pid {
        PID_BLS12_381 => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(handle_2);
            let mut point_1_affine = bls12_381::G1Affine::default();
            bls12_381::G1Projective::batch_normalize(&[*point_1], &mut [point_1_affine]);
            let mut point_2_affine = bls12_381::G2Affine::default();
            bls12_381::G2Projective::batch_normalize(&[*point_2], &mut [point_2_affine]);

            let result = bls12_381::pairing(&point_1_affine, &point_2_affine);
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(result);
            handle
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "bytes_into_point_internal",
            make_native_from_func(gas_params.clone(), bytes_into_point_internal),
        ),
        (
            "bytes_into_scalar_internal",
            make_native_from_func(gas_params.clone(), bytes_into_scalar_internal),
        ),
        (
            "scalar_one_internal",
            make_native_from_func(gas_params.clone(), scalar_one_internal),
        ),
        (
            "scalar_from_u64_internal",
            make_native_from_func(gas_params.clone(), scalar_from_u64_internal),
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
            "point_generator_internal",
            make_native_from_func(gas_params.clone(), point_generator_internal),
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
        (
            "pairing_internal",
            make_native_from_func(gas_params.clone(), pairing_internal),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![]);

    crate::natives::helpers::make_module_natives(natives)
}
