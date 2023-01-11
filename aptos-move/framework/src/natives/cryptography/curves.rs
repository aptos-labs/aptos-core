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
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use better_any::{Tid, TidAble};
use bls12_381;
// use group::{Group};
use ark_bls12_381::{Fq12, Fr, Parameters};
use ark_ec::bls12::{Bls12Parameters, G1Prepared};
use ark_ec::group::Group;
use ark_ff::PrimeField;
use bls12_381::G2Prepared;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, NumArgs};
use move_core_types::language_storage::TypeTag;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use move_vm_types::values::{Struct, Vector};
use num_traits::identities::Zero;
use num_traits::One;
use smallvec::smallvec;
use std::collections::VecDeque;
use std::iter::Map;
use std::ops::{Add, Mul, Neg};
use std::slice::Iter;
use once_cell::sync::Lazy;
use crate::natives::cryptography::curves::abort_codes::E_UNKNOWN_GROUP;

pub mod abort_codes {
    pub const E_UNKNOWN_GROUP: u64 = 2;
    pub const E_UNKNOWN_PAIRING: u64 = 3;
}

#[derive(Debug, Clone)]
pub struct Bls12381GasParameters {
    pub fr_add: InternalGas,
    pub fr_deserialize: InternalGas,
    pub fr_div: InternalGas,
    pub fr_eq: InternalGas,
    pub fr_from_u64: InternalGas,
    pub fr_inv: InternalGas,
    pub fr_mul: InternalGas,
    pub fr_neg: InternalGas,
    pub fr_serialize: InternalGas,
    pub fr_sub: InternalGas,
    pub fr_to_repr: InternalGas,
    pub fq12_clone: InternalGas,
    pub fq12_deserialize: InternalGas,
    pub fq12_eq: InternalGas,
    pub fq12_inv: InternalGas,
    pub fq12_mul: InternalGas,
    pub fq12_one: InternalGas,
    pub fq12_pow_fr: InternalGas,
    pub fq12_serialize: InternalGas,
    pub fq12_square: InternalGas,
    pub g1_affine_add: InternalGas,
    pub g1_affine_deserialize_compressed: InternalGas,
    pub g1_affine_deserialize_uncompressed: InternalGas,
    pub g1_affine_eq_proj: InternalGas,
    pub g1_affine_generator: InternalGas,
    pub g1_affine_infinity: InternalGas,
    pub g1_affine_mul_to_proj: InternalGas,
    pub g1_affine_neg: InternalGas,
    pub g1_affine_serialize_uncompressed: InternalGas,
    pub g1_affine_serialize_compressed: InternalGas,
    pub g1_affine_to_prepared: InternalGasPerArg,
    pub g1_affine_to_proj: InternalGas,
    pub g1_proj_add: InternalGas,
    pub g1_proj_addassign: InternalGas,
    pub g1_proj_double: InternalGas,
    pub g1_proj_eq: InternalGas,
    pub g1_proj_generator: InternalGas,
    pub g1_proj_infinity: InternalGas,
    pub g1_proj_mul: InternalGas,
    pub g1_proj_mulassign: InternalGas,
    pub g1_proj_neg: InternalGas,
    pub g1_proj_sub: InternalGas,
    pub g1_proj_subassign: InternalGas,
    pub g1_proj_to_affine: InternalGas,
    pub g1_proj_to_prepared: InternalGasPerArg,
    pub g2_affine_add: InternalGas,
    pub g2_affine_deserialize_compressed: InternalGas,
    pub g2_affine_deserialize_uncompressed: InternalGas,
    pub g2_affine_eq_proj: InternalGas,
    pub g2_affine_generator: InternalGas,
    pub g2_affine_infinity: InternalGas,
    pub g2_affine_mul_to_proj: InternalGas,
    pub g2_affine_neg: InternalGas,
    pub g2_affine_serialize_compressed: InternalGas,
    pub g2_affine_serialize_uncompressed: InternalGas,
    pub g2_affine_to_prepared: InternalGasPerArg,
    pub g2_affine_to_proj: InternalGas,
    pub g2_proj_add: InternalGas,
    pub g2_proj_addassign: InternalGas,
    pub g2_proj_double: InternalGas,
    pub g2_proj_eq: InternalGas,
    pub g2_proj_generator: InternalGas,
    pub g2_proj_infinity: InternalGas,
    pub g2_proj_mul: InternalGas,
    pub g2_proj_mulassign: InternalGas,
    pub g2_proj_neg: InternalGas,
    pub g2_proj_sub: InternalGas,
    pub g2_proj_subassign: InternalGas,
    pub g2_proj_to_affine: InternalGas,
    pub g2_proj_to_prepared: InternalGasPerArg,
    pub pairing_product_base: InternalGas,
    pub pairing_product_per_pair: InternalGasPerArg,
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub bls12_381: Bls12381GasParameters,
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

fn serialize_element_uncompressed_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle)
                .into_affine()
                .serialize_uncompressed(&mut buf);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_to_affine + gas_params.bls12_381.g1_affine_serialize_uncompressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        "0x1::curves::BLS12_381_G2" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(handle)
                .into_affine()
                .serialize_uncompressed(&mut buf);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_to_affine + gas_params.bls12_381.g2_affine_serialize_uncompressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        "0x1::curves::BLS12_381_Gt" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<ArksContext>()
                .get_gt_point(handle)
                .serialize_uncompressed(&mut buf);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_serialize,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
}

fn serialize_element_compressed_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle)
                .into_affine()
                .serialize(&mut buf);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_to_affine + gas_params.bls12_381.g1_affine_serialize_compressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        "0x1::curves::BLS12_381_G2" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(handle)
                .into_affine()
                .serialize(&mut buf);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_to_affine + gas_params.bls12_381.g2_affine_serialize_compressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        "0x1::curves::BLS12_381_Gt" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<ArksContext>()
                .get_gt_point(handle)
                .serialize(&mut buf);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_serialize,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
}

fn deserialize_element_uncompressed_internal(
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
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let cost = gas_params.bls12_381.g1_affine_deserialize_uncompressed + gas_params.bls12_381.g1_affine_to_proj;
            let point = ark_bls12_381::G1Affine::deserialize_uncompressed(bytes.as_slice());
            match point {
                Ok(point) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<ArksContext>()
                        .add_g1_point(point.into_projective());
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                }
                _ => {
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        "0x1::curves::BLS12_381_G2" => {
            let cost = gas_params.bls12_381.g2_affine_deserialize_uncompressed + gas_params.bls12_381.g2_affine_to_proj;
            let point = ark_bls12_381::G2Affine::deserialize_uncompressed(bytes.as_slice());
            match point {
                Ok(point) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<ArksContext>()
                        .add_g2_point(point.into_projective());
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                }
                _ => {
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        "0x1::curves::BLS12_381_Gt" => {
            let cost = gas_params.bls12_381.fq12_deserialize + gas_params.bls12_381.fq12_pow_fr + gas_params.bls12_381.fq12_eq;
            let point = ark_bls12_381::Fq12::deserialize(bytes.as_slice());
            match point {
                Ok(point) => {
                    let r_buf = hex::decode(
                        "01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73",
                    )
                    .unwrap();
                    let r =
                        ark_ff::BigInteger256::deserialize_uncompressed(r_buf.as_slice()).unwrap();
                    if ark_bls12_381::Fq12::one() == point.pow(r) {
                        let handle = context
                            .extensions_mut()
                            .get_mut::<ArksContext>()
                            .add_gt_point(point);
                        Ok(NativeResult::ok(
                            cost,
                            smallvec![Value::bool(true), Value::u64(handle as u64)],
                        ))
                    } else {
                        Ok(NativeResult::ok(
                            cost,
                            smallvec![Value::bool(false), Value::u64(0)],
                        ))
                    }
                }
                _ => {
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
}

fn deserialize_element_compressed_internal(
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
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let cost = gas_params.bls12_381.g1_affine_deserialize_compressed + gas_params.bls12_381.g1_affine_to_proj;
            let point = ark_bls12_381::G1Affine::deserialize(bytes.as_slice());
            match point {
                Ok(point) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<ArksContext>()
                        .add_g1_point(point.into_projective());
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                }
                _ => {
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        "0x1::curves::BLS12_381_G2" => {
            let cost = gas_params.bls12_381.g2_affine_deserialize_compressed + gas_params.bls12_381.g2_affine_to_proj;
            let point = ark_bls12_381::G2Affine::deserialize(bytes.as_slice());
            match point {
                Ok(point) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<ArksContext>()
                        .add_g2_point(point.into_projective());
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                }
                _ => {
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        "0x1::curves::BLS12_381_Gt" => {
            let cost = gas_params.bls12_381.fq12_deserialize + gas_params.bls12_381.fq12_pow_fr + gas_params.bls12_381.fq12_eq;
            let point = ark_bls12_381::Fq12::deserialize(bytes.as_slice());
            match point {
                Ok(point) => {
                    let r_buf = hex::decode(
                        "01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73",
                    )
                    .unwrap();
                    let r =
                        ark_ff::BigInteger256::deserialize_uncompressed(r_buf.as_slice()).unwrap();
                    if ark_bls12_381::Fq12::one() == point.pow(r) {
                        let handle = context
                            .extensions_mut()
                            .get_mut::<ArksContext>()
                            .add_gt_point(point);
                        Ok(NativeResult::ok(
                            cost,
                            smallvec![Value::bool(true), Value::u64(handle as u64)],
                        ))
                    } else {
                        Ok(NativeResult::ok(
                            cost,
                            smallvec![Value::bool(false), Value::u64(0)],
                        ))
                    }
                }
                _ => {
                    Ok(NativeResult::ok(
                        cost,
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
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
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let scalar = ark_bls12_381::Fr::deserialize_uncompressed(bytes.as_slice());
            match (scalar) {
                Ok(scalar) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<ArksContext>()
                        .add_scalar(scalar);
                    Ok(NativeResult::ok(
                        gas_params.bls12_381.fr_deserialize,
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                }
                _ => {
                    Ok(NativeResult::ok(
                        gas_params.bls12_381.fr_deserialize,
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                },
            }
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
}

fn scalar_to_bytes_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle)
                .serialize_uncompressed(&mut buf);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_serialize,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
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
    let value = pop_arg!(args, u64);
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_scalar(ark_bls12_381::Fr::from(value as u128));
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_from_u64,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
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
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_add,
                smallvec![Value::u64(result_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
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
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_mul,
                smallvec![Value::u64(result_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
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
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_neg,
                smallvec![Value::u64(result_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
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
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
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
                    Ok(NativeResult::ok(
                        gas_params.bls12_381.fr_inv,
                        smallvec![Value::bool(true), Value::u64(ret as u64)],
                    ))

                }
                None => {
                    Ok(NativeResult::ok(
                        gas_params.bls12_381.fr_inv,
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                },
            }
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), E_UNKNOWN_GROUP))
        }
    }
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
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            let scalar_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle_1);
            let scalar_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_scalar(handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_eq,
                smallvec![Value::bool(scalar_1 == scalar_2)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                gas_params.bls12_381.fr_eq,
                abort_codes::E_UNKNOWN_GROUP,
            ))
        }
    }
}

fn group_identity_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point = ark_bls12_381::G1Projective::zero();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_infinity,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::curves::BLS12_381_G2" => {
            let point = ark_bls12_381::G2Projective::zero();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_infinity,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::curves::BLS12_381_Gt" => {
            let point = ark_bls12_381::Fq12::one();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_gt_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_one,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                abort_codes::E_UNKNOWN_GROUP,
            ))
        }
    }
}

static BLS12381_GT_GENERATOR: Lazy<ark_bls12_381::Fq12> = Lazy::new(||{
    let buf = hex::decode("b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f").unwrap();
    ark_bls12_381::Fq12::deserialize(buf.as_slice()).unwrap()
});

static BLS12381_R_BYTES_LENDIAN: Lazy<Vec<u8>> = Lazy::new(||{
    hex::decode("01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73").unwrap()
});

fn group_generator_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point = ark_bls12_381::G1Projective::prime_subgroup_generator();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_generator,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::curves::BLS12_381_G2" => {
            let point = ark_bls12_381::G2Projective::prime_subgroup_generator();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_generator,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::curves::BLS12_381_Gt" => {
            let point = BLS12381_GT_GENERATOR.clone();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_gt_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_clone,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                abort_codes::E_UNKNOWN_GROUP,
            ))
        }
    }
}

fn group_order_internal(
    _gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::vector_u8(BLS12381_R_BYTES_LENDIAN.clone())],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                abort_codes::E_UNKNOWN_GROUP,
            ))
        }
    }
}

fn is_prime_order_internal(
    _gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" | "0x1::curves::BLS12_381_G2" | "0x1::curves::BLS12_381_Gt" => {
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::bool(true)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::E_UNKNOWN_GROUP))
        }
    }
}

fn element_eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point_1 = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_eq,
                smallvec![Value::bool(point_1.eq(point_2))],
            ))
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_eq,
                smallvec![Value::bool(point_1.eq(point_2))],
            ))
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_eq,
                smallvec![Value::bool(point_1.eq(point_2))],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::E_UNKNOWN_GROUP))
        }
    }
}

fn element_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_add,
                smallvec![Value::u64(handle as u64)],
            ))
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_add,
                smallvec![Value::u64(handle as u64)],
            ))
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_mul,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::E_UNKNOWN_GROUP))
        }
    }
}

fn element_mul_scalar_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let point_handle = pop_arg!(args, u64) as usize;
    let scalar_handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_mul,
                smallvec![Value::u64(handle as u64)],
            ))
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_mul,
                smallvec![Value::u64(handle as u64)],
            ))
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_to_repr + gas_params.bls12_381.fq12_pow_fr,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::E_UNKNOWN_GROUP))
        }
    }
}

fn element_double_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let point_handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(point_handle);
            let result = ark_ec::ProjectiveCurve::double(point);
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_double,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::curves::BLS12_381_G2" => {
            let point = context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(point_handle);
            let result = ark_ec::ProjectiveCurve::double(point);
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_double,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::curves::BLS12_381_Gt" => {
            let element = context
                .extensions()
                .get::<ArksContext>()
                .get_gt_point(point_handle);
            let result = element.square();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_gt_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_square,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::E_UNKNOWN_GROUP))
        }
    }
}

fn element_neg_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let point_handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::curves::BLS12_381_G1" => {
            let point = context
                .extensions()
                .get::<ArksContext>()
                .get_g1_point(point_handle);
            let result = point.neg();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g1_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_neg,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::curves::BLS12_381_G2" => {
            let point = context
                .extensions()
                .get::<ArksContext>()
                .get_g2_point(point_handle);
            let result = point.neg();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_g2_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_neg,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::curves::BLS12_381_Gt" => {
            let point = context
                .extensions()
                .get::<ArksContext>()
                .get_gt_point(point_handle);
            let result = point.inverse().unwrap();
            let handle = context
                .extensions_mut()
                .get_mut::<ArksContext>()
                .add_gt_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_inv,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::E_UNKNOWN_GROUP))
        }
    }
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
    let g2_handles = pop_vec_u64(&mut args)?;
    let g1_handles = pop_vec_u64(&mut args)?;
    match (
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
            Ok(NativeResult::ok(
                (gas_params.bls12_381.g1_affine_to_prepared + gas_params.bls12_381.g2_affine_to_prepared + gas_params.bls12_381.pairing_product_per_pair) * NumArgs::new(g1_handles.len() as u64) + gas_params.bls12_381.pairing_product_base,
                smallvec![Value::u64(result_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::E_UNKNOWN_PAIRING))
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "serialize_element_uncompressed_internal",
            make_native_from_func(gas_params.clone(), serialize_element_uncompressed_internal),
        ),
        (
            "serialize_element_compressed_internal",
            make_native_from_func(gas_params.clone(), serialize_element_compressed_internal),
        ),
        (
            "deserialize_element_uncompressed_internal",
            make_native_from_func(
                gas_params.clone(),
                deserialize_element_uncompressed_internal,
            ),
        ),
        (
            "deserialize_element_compressed_internal",
            make_native_from_func(gas_params.clone(), deserialize_element_compressed_internal),
        ),
        (
            "scalar_from_bytes_internal",
            make_native_from_func(gas_params.clone(), scalar_from_bytes_internal),
        ),
        (
            "scalar_to_bytes_internal",
            make_native_from_func(gas_params.clone(), scalar_to_bytes_internal),
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
            "group_identity_internal",
            make_native_from_func(gas_params.clone(), group_identity_internal),
        ),
        (
            "group_generator_internal",
            make_native_from_func(gas_params.clone(), group_generator_internal),
        ),
        (
            "element_add_internal",
            make_native_from_func(gas_params.clone(), element_add_internal),
        ),
        (
            "element_mul_internal",
            make_native_from_func(gas_params.clone(), element_mul_scalar_internal),
        ),
        (
            "element_double_internal",
            make_native_from_func(gas_params.clone(), element_double_internal),
        ),
        (
            "element_neg_internal",
            make_native_from_func(gas_params.clone(), element_neg_internal),
        ),
        (
            "element_eq_internal",
            make_native_from_func(gas_params.clone(), element_eq_internal),
        ),
        (
            "multi_pairing_internal",
            make_native_from_func(gas_params.clone(), multi_pairing_internal),
        ),
        (
            "group_order_internal",
            make_native_from_func(gas_params.clone(), group_order_internal),
        ),
        (
            "is_prime_order_internal",
            make_native_from_func(gas_params.clone(), is_prime_order_internal),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![]);

    crate::natives::helpers::make_module_natives(natives)
}

/// A workaround for popping a vector<u64> from the argument stack,
/// before [a proper fix](`https://github.com/move-language/move/pull/773`) is complete.
/// It requires the move native function to push 2 items in the argument stack:
/// first the length of the vector as a u64, then the vector itself.
/// TODO: Remove this once working with `vector<u64>` in rust is well supported.
fn pop_vec_u64(args: &mut VecDeque<Value>) -> PartialVMResult<Vec<u64>> {
    let vector = args.pop_back().unwrap().value_as::<Vector>()?;
    let vector_len = args.pop_back().unwrap().value_as::<u64>()?;
    let mut values = Vec::with_capacity(vector_len as usize);
    for item in vector.unpack(&Type::U64, vector_len)?.into_iter() {
        let value = item.value_as::<u64>()?;
        values.push(value);
    }
    Ok(values)
}
