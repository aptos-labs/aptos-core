// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::groth16_bls12381_bellman::BellmanContext;
use crate::natives::util::{make_native_from_func, make_test_only_native_from_func};
use crate::pop_vec_arg;
use aptos_crypto::bls12381::arithmetics::Scalar;
use aptos_crypto::bls12381::PrivateKey;
use ark_ec::ProjectiveCurve;
use ark_ec::{AffineCurve, PairingEngine};
use ark_ff::fields::Field;
use ark_serialize::CanonicalDeserialize;
use better_any::{Tid, TidAble};
use bls12_381;
// use group::{Group};
use ark_bls12_381::{Fq12, Fr, Parameters};
use ark_ec::bls12::{Bls12Parameters, G1Prepared};
use ark_ec::group::Group;
use ark_ff::PrimeField;
use bls12_381::G2Prepared;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
use move_core_types::language_storage::TypeTag;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::Struct;
use move_vm_types::values::Value;
use num_traits::identities::Zero;
use smallvec::smallvec;
use std::collections::VecDeque;
use std::iter::Map;
use std::ops::{Add, Mul, Neg};
use std::slice::Iter;

pub mod abort_codes {
    pub const E_CURVE_TYPE_NOT_SUPPORTED: u64 = 1;
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
}

#[derive(Tid)]
pub struct ArksContext {
    fr_store: Vec<ark_bls12_381::Fr>,
    g1_point_store: Vec<ark_bls12_381::G1Projective>,
    g2_point_store: Vec<ark_bls12_381::G2Projective>,
    gt_point_store: Vec<ark_bls12_381::Fq12>,
}

impl ArksContext {
    pub fn new() -> Self {
        Self {
            fr_store: vec![],
            g1_point_store: vec![],
            g2_point_store: vec![],
            gt_point_store: vec![],
        }
    }

    pub fn add_scalar(&mut self, scalar: ark_bls12_381::Fr) -> usize {
        let ret = self.fr_store.len();
        self.fr_store.push(scalar);
        ret
    }

    pub fn get_scalar(&self, handle: usize) -> &ark_bls12_381::Fr {
        self.fr_store.get(handle).unwrap()
    }

    pub fn add_g1_point(&mut self, p0: ark_bls12_381::G1Projective) -> usize {
        let ret = self.g1_point_store.len();
        self.g1_point_store.push(p0);
        ret
    }

    pub fn get_g1_point(&self, handle: usize) -> &ark_bls12_381::G1Projective {
        self.g1_point_store.get(handle).unwrap()
    }

    pub fn add_g2_point(&mut self, p0: ark_bls12_381::G2Projective) -> usize {
        let ret = self.g2_point_store.len();
        self.g2_point_store.push(p0);
        ret
    }

    pub fn get_g2_point(&self, handle: usize) -> &ark_bls12_381::G2Projective {
        self.g2_point_store.get(handle).unwrap()
    }

    pub fn add_gt_point(&mut self, point: ark_bls12_381::Fq12) -> usize {
        let ret = self.gt_point_store.len();
        self.gt_point_store.push(point);
        ret
    }

    pub fn get_gt_point(&self, handle: usize) -> &ark_bls12_381::Fq12 {
        self.gt_point_store.get(handle).unwrap()
    }
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

fn element_from_bytes_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let bytes = pop_arg!(args, Vec<u8>);
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point = ark_bls12_381::G1Affine::deserialize_uncompressed(bytes.as_slice())
                .unwrap()
                .into_projective();
            context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(point)
        }
        "0x1::curves::BLS12_381_G2" => {
            let point = ark_bls12_381::G2Affine::deserialize_uncompressed(bytes.as_slice())
                .unwrap()
                .into_projective();
            context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(point)
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(handle as u8)],
    ))
}

fn scalar_from_bytes_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let bytes = pop_arg!(args, Vec<u8>);
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" => {
            let scalar = ark_bls12_381::Fr::deserialize_uncompressed(bytes.as_slice()).unwrap();
            context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_scalar(scalar)
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::bool(true), Value::u8(handle as u8)],
    ))
}

fn scalar_from_u64_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let v = pop_arg!(args, u64);
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_scalar(ark_bls12_381::Fr::from(v as u128));
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
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let scalar_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle_1);
            let scalar_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle_2);
            let result = scalar_1.add(scalar_2);
            let result_handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
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

fn scalar_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let scalar_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle_1);
            let scalar_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle_2);
            let result = scalar_1.mul(scalar_2);
            let result_handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
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

fn scalar_neg_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u8) as usize;
    let result_handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let result = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle)
                .neg();
            let result_handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_scalar(result);
            result_handle
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::u8(result_handle as u8)],
    ))
}

fn scalar_inv_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u8) as usize;
    let (succeeded, result_handle) = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let op_result = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle)
                .inverse();
            match op_result {
                Some(scalar) => {
                    let ret = context
                        .extensions_mut()
                        .get_mut::<ArksContext>()
                        .add_scalar(scalar);
                    (true, ret)
                }
                None => (false, 0),
            }
        }
        _ => todo!(),
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::bool(succeeded), Value::u8(result_handle as u8)],
    ))
}

fn scalar_eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let result = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let scalar_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle_1);
            let scalar_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle_2);
            scalar_1 == scalar_2
        }
        _ => {
            return Ok(NativeResult::err(
                gas_params.base,
                abort_codes::E_CURVE_TYPE_NOT_SUPPORTED,
            ))
        }
    };
    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::bool(result)],
    ))
}

fn point_identity_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point = ark_bls12_381::G1Projective::zero();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(point);
            handle
        }
        "0x1::curves::BLS12_381_G2" => {
            let point = ark_bls12_381::G2Projective::zero();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(point);
            handle
        }
        "0x1::curves::BLS12_381_Gt" => {
            let point = ark_bls12_381::Fq12::zero();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
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

pub const PID_BLS12_381: u8 = 1;

fn point_generator_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point = ark_bls12_381::G1Projective::prime_subgroup_generator();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(point);
            handle
        }
        "0x1::curves::BLS12_381_G2" => {
            let point = ark_bls12_381::G2Projective::prime_subgroup_generator();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(point);
            handle
        }
        "0x1::curves::BLS12_381_Gt" => {
            let point = ark_bls12_381::Fq12::zero();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
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
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let result = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle_2);
            let result = point_1.eq(point_2);
            result
        }
        "0x1::curves::BLS12_381_G2" => {
            let point_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(handle_2);
            let result = point_1.eq(point_2);
            result
        }
        "0x1::curves::BLS12_381_Gt" => {
            let point_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_gt_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<ArksContext>()
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
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle_2);
            let result = point_1.add(point_2);
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(result);
            handle
        }
        "0x1::curves::BLS12_381_G2" => {
            let point_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(handle_2);
            let result = point_1.add(point_2);
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(result);
            handle
        }
        "0x1::curves::BLS12_381_Gt" => {
            let point_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_gt_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_gt_point(handle_2);
            let result = point_1.clone() * point_2.clone();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
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
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let point_handle = pop_arg!(args, u8) as usize;
    let scalar_handle = pop_arg!(args, u8) as usize;
    let handle = match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(point_handle);
            let scalar = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(scalar_handle);
            let result = point.mul(scalar);
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(result);
            handle
        }
        "0x1::curves::BLS12_381_G2" => {
            let point = context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(point_handle);
            let scalar = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(scalar_handle);
            let result = point.mul(scalar);
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(result);
            handle
        }
        "0x1::curves::BLS12_381_Gt" => {
            let point = context
                .extensions()
                .get::<ArksContext>()
                .get_gt_point(point_handle);
            let scalar = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(scalar_handle);
            let result = point.pow(scalar.into_repr().as_ref());
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
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
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(3, ty_args.len());
    let type_tag_0 = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let type_tag_1 = context
        .type_to_type_tag(ty_args.get(1).unwrap())?
        .to_string();
    let type_tag_2 = context
        .type_to_type_tag(ty_args.get(2).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u8) as usize;
    let handle_1 = pop_arg!(args, u8) as usize;
    let handle = match (
        type_tag_0.as_str(),
        type_tag_1.as_str(),
        type_tag_2.as_str(),
    ) {
        ("0x1::curves::BLS12_381_G1", "0x1::curves::BLS12_381_G2", "0x1::curves::BLS12_381_Gt") => {
            let point_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle_1)
                .into_affine();
            let point_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(handle_2)
                .into_affine();
            let result = ark_bls12_381::Bls12_381::pairing(point_1, point_2);
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
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

fn multi_pairing_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(3, ty_args.len());
    let type_tag_0 = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let type_tag_1 = context
        .type_to_type_tag(ty_args.get(1).unwrap())?
        .to_string();
    let type_tag_2 = context
        .type_to_type_tag(ty_args.get(2).unwrap())?
        .to_string();
    let g2_handles = pop_arg!(args, Vec<u8>);
    let g1_handles = pop_arg!(args, Vec<u8>);
    let handle = match (
        type_tag_0.as_str(),
        type_tag_1.as_str(),
        type_tag_2.as_str(),
    ) {
        ("0x1::curves::BLS12_381_G1", "0x1::curves::BLS12_381_G2", "0x1::curves::BLS12_381_Gt") => {
            let g1_prepared: Vec<ark_ec::models::bls12::g1::G1Prepared<Parameters>> = g1_handles
                .iter()
                .map(|&handle| {
                    let element = context
                        .extensions()
                        .get::<ArksContext>()
                        .get_g1_point(handle as usize);
                    ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(element.into_affine())
                })
                .collect();
            let g2_prepared: Vec<ark_ec::models::bls12::g2::G2Prepared<Parameters>> = g2_handles
                .iter()
                .map(|&handle| {
                    let element = context
                        .extensions()
                        .get::<ArksContext>()
                        .get_g2_point(handle as usize);
                    ark_ec::prepare_g2::<ark_bls12_381::Bls12_381>(element.into_affine())
                })
                .collect();

            let z: Vec<(
                ark_ec::models::bls12::g1::G1Prepared<Parameters>,
                ark_ec::models::bls12::g2::G2Prepared<Parameters>,
            )> = g1_prepared
                .into_iter()
                .zip(g2_prepared.into_iter())
                .collect();
            let result = ark_bls12_381::Bls12_381::product_of_pairings(z.as_slice());
            let result_handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_gt_point(result);
            result_handle
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
            "element_from_bytes_internal",
            make_native_from_func(gas_params.clone(), element_from_bytes_internal),
        ),
        (
            "scalar_from_bytes_internal",
            make_native_from_func(gas_params.clone(), scalar_from_bytes_internal),
        ),
        (
            "scalar_eq_internal",
            make_native_from_func(gas_params.clone(), scalar_eq_internal),
        ),
        (
            "scalar_neg_internal",
            make_native_from_func(gas_params.clone(), scalar_neg_internal),
        ),
        (
            "scalar_inv_internal",
            make_native_from_func(gas_params.clone(), scalar_inv_internal),
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
            "scalar_mul_internal",
            make_native_from_func(gas_params.clone(), scalar_mul_internal),
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
        (
            "multi_pairing_internal",
            make_native_from_func(gas_params.clone(), multi_pairing_internal),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![]);

    crate::natives::helpers::make_module_natives(natives)
}
