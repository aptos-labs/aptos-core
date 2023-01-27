// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0


use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg};
use std::rc::Rc;
use ark_bls12_381::{Fq12, Fr, FrParameters, G1Projective, G2Projective, Parameters};
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::{Field, Fp256, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
#[cfg(feature = "testing")]
use ark_std::{test_rng, UniformRand};
use better_any::{Tid, TidAble, TidExt};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, InternalGasPerByte, NumArgs, NumBytes};
use move_core_types::language_storage::TypeTag;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use num_traits::{One, Zero};
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};
use smallvec::smallvec;
use aptos_types::on_chain_config::{FeatureFlag, Features};
use aptos_types::on_chain_config::FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS;
use crate::natives::cryptography::groups::abort_codes::{NOT_IMPLEMENTED, NUM_G1_ELEMENTS_SHOULD_MATCH_NUM_G2_ELEMENTS};
use crate::natives::cryptography::groups::API::ScalarDeserialize;
use crate::natives::util::make_native_from_func;
#[cfg(feature = "testing")]
use crate::natives::util::make_test_only_native_from_func;


macro_rules! abort_if_feature_disabled {
    ($context:expr, $feature:expr) => {
        if !$context.extensions().get::<GroupContext>().features.is_enabled($feature) {
            return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
        }
    };
}

macro_rules! ark_serialize_uncompressed {
    ($ark_element:expr) => {{
        let mut buf = vec![];
        $ark_element.serialize_uncompressed(&mut buf).unwrap();
        buf
    }}
}

macro_rules! ark_serialize_compressed {
    ($ark_element:expr) => {{
        let mut buf = vec![];
        $ark_element.serialize(&mut buf).unwrap();
        buf
    }}
}

macro_rules! structure_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        structure_from_type_tag(&type_tag)
    }}
}

macro_rules! hash_alg_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        hash_alg_from_type_tag(&type_tag)
    }}
}

macro_rules! borrow_bls12_381_g1 {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<GroupContext>().bls12_381_g1_elements.get($handle).unwrap()
    }}
}

macro_rules! store_bls12_381_g1 {
    ($context:expr, $element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<GroupContext>();
        let ret = inner_ctxt.bls12_381_g1_elements.len();
        inner_ctxt.bls12_381_g1_elements.push($element);
        ret
    }}
}

macro_rules! borrow_bls12_381_fr {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<GroupContext>().bls12_381_fr_elements.get($handle).unwrap()
    }}
}

macro_rules! store_bls12_381_fr {
    ($context:expr, $element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<GroupContext>();
        let ret = inner_ctxt.bls12_381_fr_elements.len();
        inner_ctxt.bls12_381_fr_elements.push($element);
        ret
    }}
}

macro_rules! borrow_bls12_381_g2 {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<GroupContext>().bls12_381_g2_elements.get($handle).unwrap()
    }}
}

macro_rules! store_bls12_381_g2 {
    ($context:expr, $element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<GroupContext>();
        let ret = inner_ctxt.bls12_381_g2_elements.len();
        inner_ctxt.bls12_381_g2_elements.push($element);
        ret
    }}
}

macro_rules! borrow_bls12_381_gt {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<GroupContext>().bls12_381_gt_elements.get($handle).unwrap()
    }}
}

macro_rules! store_bls12_381_gt {
    ($context:expr, $element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<GroupContext>();
        let ret = inner_ctxt.bls12_381_gt_elements.len();
        inner_ctxt.bls12_381_gt_elements.push($element);
        ret
    }}
}

macro_rules! borrow_bls12_381_gt {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<GroupContext>().bls12_381_gt_elements.get($handle).unwrap()
    }}
}

pub mod abort_codes {
    pub const NOT_IMPLEMENTED: u64 = 2;
    pub const NUM_ELEMENTS_SHOULD_MATCH_NUM_SCALARS: u64 = 4;
    pub const NUM_G1_ELEMENTS_SHOULD_MATCH_NUM_G2_ELEMENTS: u64 = 5;
}

#[derive(Debug, Clone)]
pub struct Bls12381GasParameters {
    pub blst_hash_to_g1_proj_base: InternalGasPerArg,
    pub blst_hash_to_g1_proj_per_byte: InternalGasPerByte,
    pub blst_hash_to_g2_proj_base: InternalGasPerArg,
    pub blst_hash_to_g2_proj_per_byte: InternalGasPerByte,
    pub blst_g1_proj_to_affine: InternalGasPerArg,
    pub blst_g1_affine_ser: InternalGasPerArg,
    pub blst_g2_proj_to_affine: InternalGasPerArg,
    pub blst_g2_affine_ser: InternalGasPerArg,
    pub fr_add: InternalGas,
    pub ark_fr_deser: InternalGasPerArg,
    pub fr_div: InternalGas,
    pub fr_eq: InternalGas,
    pub fr_from_u64: InternalGas,
    pub fr_inv: InternalGas,
    pub fr_mul: InternalGas,
    pub fr_neg: InternalGas,
    pub fr_rand: InternalGas,
    pub fr_serialize: InternalGas,
    pub fr_sub: InternalGas,
    pub fr_to_repr: InternalGasPerArg,
    pub fq12_clone: InternalGas,
    pub fq12_deserialize: InternalGasPerArg,
    pub fq12_eq: InternalGasPerArg,
    pub fq12_inv: InternalGas,
    pub fq12_mul: InternalGasPerArg,
    pub fq12_one: InternalGas,
    pub ark_fq12_pow_fr: InternalGasPerArg,
    pub fq12_serialize: InternalGas,
    pub fq12_square: InternalGas,
    pub g1_affine_add: InternalGas,
    pub ark_g1_affine_deser_comp: InternalGasPerArg,
    pub ark_g1_affine_deser_uncomp: InternalGasPerArg,
    pub g1_affine_eq_proj: InternalGas,
    pub g1_affine_generator: InternalGas,
    pub g1_affine_infinity: InternalGas,
    pub g1_affine_mul_to_proj: InternalGas,
    pub g1_affine_neg: InternalGas,
    pub g1_affine_serialize_uncompressed: InternalGas,
    pub g1_affine_serialize_compressed: InternalGas,
    pub g1_affine_to_prepared: InternalGasPerArg,
    pub ark_g1_affine_to_proj: InternalGasPerArg,
    pub g1_proj_add: InternalGasPerArg,
    pub g1_proj_addassign: InternalGasPerArg,
    pub g1_proj_double: InternalGas,
    pub g1_proj_eq: InternalGas,
    pub g1_proj_generator: InternalGas,
    pub g1_proj_infinity: InternalGas,
    pub g1_proj_mul: InternalGasPerArg,
    pub g1_proj_mulassign: InternalGasPerArg,
    pub g1_proj_neg: InternalGas,
    pub g1_proj_rand: InternalGas,
    pub g1_proj_sub: InternalGas,
    pub g1_proj_subassign: InternalGas,
    pub g1_proj_to_affine: InternalGas,
    pub g1_proj_to_prepared: InternalGasPerArg,
    pub g2_affine_add: InternalGas,
    pub ark_g2_affine_deser_comp: InternalGasPerArg,
    pub ark_g2_affine_deser_uncomp: InternalGasPerArg,
    pub g2_affine_eq_proj: InternalGas,
    pub g2_affine_generator: InternalGas,
    pub g2_affine_infinity: InternalGas,
    pub g2_affine_mul_to_proj: InternalGas,
    pub g2_affine_neg: InternalGas,
    pub g2_affine_serialize_compressed: InternalGas,
    pub g2_affine_serialize_uncompressed: InternalGas,
    pub g2_affine_to_prepared: InternalGasPerArg,
    pub ark_g2_affine_to_proj: InternalGasPerArg,
    pub g2_proj_add: InternalGasPerArg,
    pub g2_proj_addassign: InternalGasPerArg,
    pub g2_proj_double: InternalGas,
    pub g2_proj_eq: InternalGas,
    pub g2_proj_generator: InternalGas,
    pub g2_proj_infinity: InternalGas,
    pub g2_proj_mul: InternalGasPerArg,
    pub g2_proj_mulassign: InternalGasPerArg,
    pub g2_proj_neg: InternalGas,
    pub g2_proj_rand: InternalGas,
    pub g2_proj_sub: InternalGas,
    pub g2_proj_subassign: InternalGas,
    pub g2_proj_to_affine: InternalGas,
    pub g2_proj_to_prepared: InternalGasPerArg,
    pub pairing_product_base: InternalGas,
    pub pairing_product_per_pair: InternalGasPerArg,
}

impl Bls12381GasParameters {
    fn blst_hash_to_g1_proj(&self, num_input_bytes: usize) -> InternalGas {
        self.blst_hash_to_g1_proj_per_byte * NumBytes::from(num_input_bytes as u64) + self.blst_hash_to_g1_proj_base * NumArgs::one()
    }

    fn blst_hash_to_g2_proj(&self, num_input_bytes: usize) -> InternalGas {
        self.blst_hash_to_g2_proj_per_byte * NumBytes::from(num_input_bytes as u64) + self.blst_hash_to_g2_proj_base * NumArgs::one()
    }

    fn sha256(&self, num_input_bytes: usize) -> InternalGas {
        //TODO
        InternalGas::zero()
    }
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub bls12_381: Bls12381GasParameters,
}

#[derive(Tid)]
pub struct GroupContext {
    features: Features,
    bls12_381_fr_elements: Vec<Fr>,
    bls12_381_g1_elements: Vec<G1Projective>,
    bls12_381_g2_elements: Vec<G2Projective>,
    bls12_381_gt_elements: Vec<Fq12>,
}

impl GroupContext {
    pub fn new(features: Features) -> Self {
        Self {
            bls12_381_fr_elements: vec![],
            bls12_381_g1_elements: vec![],
            bls12_381_g2_elements: vec![],
            bls12_381_gt_elements: vec![],
            features,
        }
    }
}

fn element_serialize_uncompressed_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_g1!(context, handle);
            let buf = ark_serialize_uncompressed!(element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_to_affine + gas_params.bls12_381.g1_affine_serialize_uncompressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_g2!(context, handle);
            let buf = ark_serialize_uncompressed!(element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_to_affine + gas_params.bls12_381.g2_affine_serialize_uncompressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_gt!(context, handle);
            let buf = ark_serialize_uncompressed!(element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_serialize,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn element_serialize_compressed_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            let element = borrow_bls12_381_g1!(context, handle);
            let buf = ark_serialize_compressed!(element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_to_affine + gas_params.bls12_381.g1_affine_serialize_compressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            let element = borrow_bls12_381_g2!(context, handle);
            let buf = ark_serialize_compressed!(element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_to_affine + gas_params.bls12_381.g2_affine_serialize_compressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            let element = borrow_bls12_381_gt!(context, handle);
            let buf = ark_serialize_compressed!(element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_serialize,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn element_deserialize_uncompressed_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let bytes = pop_arg!(args, Vec<u8>);
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let cost = (gas_params.bls12_381.ark_g1_affine_deser_uncomp + gas_params.bls12_381.ark_g1_affine_to_proj) * NumArgs::one();
            match ark_bls12_381::G1Affine::deserialize_uncompressed(bytes.as_slice()) {
                Ok(element) => {
                    let handle = store_bls12_381_g1!(context, element.into_projective());
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
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let cost = (gas_params.bls12_381.ark_g2_affine_deser_uncomp + gas_params.bls12_381.ark_g2_affine_to_proj) * NumArgs::one();
            match ark_bls12_381::G2Affine::deserialize_uncompressed(bytes.as_slice()) {
                Ok(element) => {
                    let handle = store_bls12_381_g2!(context, element.into_projective());
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
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let cost = (gas_params.bls12_381.fq12_deserialize + gas_params.bls12_381.ark_fq12_pow_fr + gas_params.bls12_381.fq12_eq) * NumArgs::one();
            match Fq12::deserialize(bytes.as_slice()) {
                Ok(element) => {
                    if Fq12::one() == element.pow(BLS12381_R_SCALAR.clone()) {
                        let handle = store_bls12_381_gt!(context, element);
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
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn element_deserialize_compressed_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let bytes = pop_arg!(args, Vec<u8>);
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let cost = (gas_params.bls12_381.ark_g1_affine_deser_comp + gas_params.bls12_381.ark_g1_affine_to_proj) * NumArgs::one();
            match ark_bls12_381::G1Affine::deserialize(bytes.as_slice()) {
                Ok(element) => {
                    let handle = store_bls12_381_g1!(context, element.into_projective());
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
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let cost = (gas_params.bls12_381.ark_g2_affine_deser_comp + gas_params.bls12_381.ark_g2_affine_to_proj) * NumArgs::one();
            match ark_bls12_381::G2Affine::deserialize(bytes.as_slice()) {
                Ok(element) => {
                    let handle = store_bls12_381_g2!(context, element.into_projective());
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
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let cost = (gas_params.bls12_381.fq12_deserialize + gas_params.bls12_381.ark_fq12_pow_fr + gas_params.bls12_381.fq12_eq) * NumArgs::one();
            match ark_bls12_381::Fq12::deserialize(bytes.as_slice()) {
                Ok(element) => {
                    if Fq12::one() == element.pow(BLS12381_R_SCALAR.clone()) {
                        let handle = store_bls12_381_gt!(context, element);
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
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn scalar_deserialize_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    let bytes = pop_arg!(args, Vec<u8>);
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            match Fr::deserialize_uncompressed(bytes.as_slice()) {
                Ok(scalar) => {
                    let handle = store_bls12_381_fr!(context, scalar);
                    Ok(NativeResult::ok(
                        gas_params.bls12_381.ark_fr_deser * NumArgs::one(),
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                }
                _ => {
                    Ok(NativeResult::ok(
                        gas_params.bls12_381.ark_fr_deser * NumArgs::one(),
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                },
            }
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn scalar_serialize_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_fr!(context, handle);
            let buf = ark_serialize_uncompressed!(element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_serialize,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn scalar_from_u64_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let value = pop_arg!(args, u64);
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = ark_bls12_381::Fr::from(value as u128);
            let handle = store_bls12_381_fr!(context, element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_from_u64,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn scalar_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let scalar_1 = borrow_bls12_381_fr!(context, handle_1);
            let scalar_2 = borrow_bls12_381_fr!(context, handle_2);
            let result = scalar_1.add(scalar_2);
            let result_handle = store_bls12_381_fr!(context, result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_add,
                smallvec![Value::u64(result_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn scalar_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let scalar_1 = borrow_bls12_381_fr!(context, handle_1);
            let scalar_2 = borrow_bls12_381_fr!(context, handle_2);
            let new_scalar = scalar_1.mul(scalar_2);
            let new_handle = store_bls12_381_fr!(context, new_scalar);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_mul,
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn scalar_neg_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let scalar = borrow_bls12_381_fr!(context, handle);
            let result = scalar.neg();
            let result_handle = store_bls12_381_fr!(context, result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_neg,
                smallvec![Value::u64(result_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
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
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_fr!(context, handle);
            match element.inverse() {
                Some(scalar) => {
                    let result_handle = store_bls12_381_fr!(context, scalar);
                    Ok(NativeResult::ok(
                        gas_params.bls12_381.fr_inv,
                        smallvec![Value::bool(true), Value::u64(result_handle as u64)],
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
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn scalar_eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let scalar_1 = borrow_bls12_381_fr!(context, handle_1);
            let scalar_2 = borrow_bls12_381_fr!(context, handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_eq,
                smallvec![Value::bool(scalar_1 == scalar_2)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                gas_params.bls12_381.fr_eq,
                NOT_IMPLEMENTED,
            ))
        }
    }
}

fn group_identity_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = G1Projective::zero();
            let handle = store_bls12_381_g1!(context, element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_infinity,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = G2Projective::zero();
            let handle = store_bls12_381_g2!(context, element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_infinity,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = Fq12::one();
            let handle = store_bls12_381_gt!(context, element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_one,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                NOT_IMPLEMENTED,
            ))
        }
    }
}

static BLS12381_GT_GENERATOR: Lazy<Fq12> = Lazy::new(||{
    let buf = hex::decode("b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f").unwrap();
    Fq12::deserialize(buf.as_slice()).unwrap()
});

static BLS12381_R_BYTES_LENDIAN: Lazy<Vec<u8>> = Lazy::new(||{
    hex::decode("01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73").unwrap()
});

static BLS12381_R_SCALAR: Lazy<ark_ff::BigInteger256> = Lazy::new(||{
    ark_ff::BigInteger256::deserialize_uncompressed(BLS12381_R_BYTES_LENDIAN.as_slice()).unwrap()
});

fn group_generator_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = ark_bls12_381::G1Projective::prime_subgroup_generator();
            let handle = store_bls12_381_g1!(context, element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_generator,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = ark_bls12_381::G2Projective::prime_subgroup_generator();
            let handle = store_bls12_381_g2!(context, element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_generator,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = BLS12381_GT_GENERATOR.clone();
            let handle = store_bls12_381_gt!(context, element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_clone,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                NOT_IMPLEMENTED,
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) | Some(Structure::BLS12_381_G2) | Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::vector_u8(BLS12381_R_BYTES_LENDIAN.clone())],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                NOT_IMPLEMENTED,
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) | Some(Structure::BLS12_381_G2) | Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::bool(true)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

#[cfg(feature = "testing")]
fn random_scalar_internal(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let scalar = Fr::rand(&mut test_rng());
            let handle = store_bls12_381_fr!(context, scalar);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                NOT_IMPLEMENTED,
            ))
        }
    }
}

#[cfg(feature = "testing")]
fn random_element_internal(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = G1Projective::rand(&mut test_rng());
            let handle = store_bls12_381_g1!(context, element);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = G2Projective::rand(&mut test_rng());
            let handle = store_bls12_381_g2!(context, element);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let k = Fr::rand(&mut test_rng());
            let element = BLS12381_GT_GENERATOR.clone().pow(k.into_repr());
            let handle = store_bls12_381_gt!(context, element);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                NOT_IMPLEMENTED,
            ))
        }
    }
}

fn element_eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element_1 = borrow_bls12_381_g1!(context, handle_1);
            let element_2 = borrow_bls12_381_g1!(context, handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_eq,
                smallvec![Value::bool(element_1.eq(element_2))],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element_1 = borrow_bls12_381_g2!(context, handle_1);
            let element_2 = borrow_bls12_381_g2!(context, handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_eq,
                smallvec![Value::bool(element_1.eq(element_2))],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element_1 = borrow_bls12_381_gt!(context, handle_1);
            let element_2 = borrow_bls12_381_gt!(context, handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_eq * NumArgs::one(),
                smallvec![Value::bool(element_1.eq(element_2))],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn element_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element_1 = borrow_bls12_381_g1!(context, handle_1);
            let element_2 = borrow_bls12_381_g1!(context, handle_2);
            let new_element = element_1.add(element_2);
            let new_handle = store_bls12_381_g1!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_add * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element_1 = borrow_bls12_381_g2!(context, handle_1);
            let element_2 = borrow_bls12_381_g2!(context, handle_2);
            let new_element = element_1.add(element_2);
            let new_handle = store_bls12_381_g2!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_add * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element_1 = borrow_bls12_381_gt!(context, handle_1);
            let element_2 = borrow_bls12_381_gt!(context, handle_2);
            let new_element = element_1.mul(element_2);
            let new_handle = store_bls12_381_gt!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_mul * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn element_scalar_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(2, ty_args.len());
    let group_structure = structure_from_ty_arg!(context, &ty_args[0]);
    let scalar_structure = structure_from_ty_arg!(context, &ty_args[1]);
    let scalar_handle = pop_arg!(args, u64) as usize;
    let element_handle = pop_arg!(args, u64) as usize;
    match (group_structure, scalar_structure) {
        (Some(Structure::BLS12_381_G1), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_g1!(context, element_handle);
            let scalar = borrow_bls12_381_fr!(context, scalar_handle);
            let new_element = element.mul(scalar.into_repr());
            let new_handle = store_bls12_381_g1!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_mul * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        (Some(Structure::BLS12_381_G2), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_g2!(context, element_handle);
            let scalar = borrow_bls12_381_fr!(context, scalar_handle);
            let new_element = element.mul(scalar.into_repr());
            let new_handle = store_bls12_381_g2!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_mul * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        (Some(Structure::BLS12_381_Gt), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_gt!(context, element_handle);
            let scalar = borrow_bls12_381_fr!(context, scalar_handle);
            let new_element = element.pow(scalar.into_repr().as_ref());
            let new_handle = store_bls12_381_gt!(context, new_element);
            Ok(NativeResult::ok(
                (gas_params.bls12_381.fr_to_repr + gas_params.bls12_381.ark_fq12_pow_fr) * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn ark_g1_affine_to_blst_g1_affine(ark: &ark_bls12_381::G1Affine) -> blst::blst_p1_affine {
    let mut buf = vec![];
    ark.serialize_uncompressed(&mut buf).unwrap();
    let is_inf = (buf[95] & 0x40) != 0;
    if is_inf {
        buf[0] |= 0x40;
        buf[48] = 0;
        buf[95] = 0;
    } else {
        buf.as_mut_slice()[0..48].reverse();
        buf.as_mut_slice()[48..96].reverse();
    }
    let mut ret = blst::blst_p1_affine::default();
    unsafe { blst::blst_p1_deserialize(&mut ret, buf.as_ptr()); }
    ret
}

#[test]
fn test_ark_g1_affine_to_blst_g1_affine() {
    // Generator.
    let expected = unsafe { blst::blst_p1_affine_generator().read() };
    let actual = ark_g1_affine_to_blst_g1_affine(&ark_bls12_381::G1Affine::prime_subgroup_generator());
    unsafe { assert!(blst::blst_p1_affine_is_equal(&expected, &actual)); }

    // Generator negated.
    let mut blst_generator_neg = unsafe { blst::blst_p1_generator().read() };
    unsafe { blst::blst_p1_cneg(&mut blst_generator_neg, true); }
    let mut expected = blst::blst_p1_affine::default();
    unsafe { blst::blst_p1_to_affine(&mut expected, &blst_generator_neg); }
    let actual = ark_g1_affine_to_blst_g1_affine(&ark_bls12_381::G1Affine::prime_subgroup_generator().neg());
    unsafe { assert!(blst::blst_p1_affine_is_equal(&expected, &actual)); }

    // Infinity.
    let blst_generator = unsafe { blst::blst_p1_generator().read() };
    let scalar_0_bytes = vec![0_u8; 32];
    let mut blst_inf = blst::blst_p1::default();
    unsafe { blst::blst_p1_mult(&mut blst_inf, &blst_generator, scalar_0_bytes.as_ptr(), 256); }
    let mut expected = blst::blst_p1_affine::default();
    unsafe { blst::blst_p1_to_affine(&mut expected, &blst_inf); }
    let actual = ark_g1_affine_to_blst_g1_affine(&ark_bls12_381::G1Affine::zero());
    unsafe { assert!(blst::blst_p1_affine_is_equal(&expected, &actual)); }
}

fn blst_g1_affine_to_ark_g1_affine(blst_point: &blst::blst_p1_affine) -> ark_bls12_381::G1Affine {
    let mut buf = vec![0; 96];
    unsafe { blst::blst_p1_affine_serialize(buf.as_mut_ptr(), blst_point); }
    let is_inf = (buf[0] & 0x40) != 0;
    if is_inf {
        buf[95] |= 0x40;
        buf[48] = 0x01;
        buf[0] = 0;
    } else {
        buf.as_mut_slice()[0..48].reverse();
        buf.as_mut_slice()[48..96].reverse();
    }
    ark_bls12_381::G1Affine::deserialize_uncompressed(buf.as_slice()).unwrap()
}

#[test]
fn test_blst_g1_affine_to_ark_g1_affine() {
    // Generator.
    let blst_generator = unsafe { blst::blst_p1_affine_generator().read() };
    let actual = blst_g1_affine_to_ark_g1_affine(&blst_generator);
    let expected = ark_bls12_381::G1Affine::prime_subgroup_generator();
    assert_eq!(expected, actual);

    // Generator negated.
    let mut blst_generator_neg = unsafe { blst::blst_p1_generator().read() };
    unsafe { blst::blst_p1_cneg(&mut blst_generator_neg, true); }
    let mut blst_generator_neg_affine = blst::blst_p1_affine::default();
    unsafe { blst::blst_p1_to_affine(&mut blst_generator_neg_affine, &blst_generator_neg); }
    let actual = blst_g1_affine_to_ark_g1_affine(&blst_generator_neg_affine);
    let expected = ark_bls12_381::G1Affine::prime_subgroup_generator().neg();
    assert_eq!(expected, actual);

    // Infinity.
    let blst_generator = unsafe { blst::blst_p1_generator().read() };
    let scalar_0_bytes = vec![0_u8; 32];
    let mut blst_inf = blst::blst_p1::default();
    unsafe { blst::blst_p1_mult(&mut blst_inf, &blst_generator, scalar_0_bytes.as_ptr(), 256); }
    let mut blst_inf_affine = blst::blst_p1_affine::default();
    unsafe { blst::blst_p1_to_affine(&mut blst_inf_affine, &blst_inf); }
    let actual = blst_g1_affine_to_ark_g1_affine(&blst_inf_affine);
    let expected = ark_bls12_381::G1Affine::zero();
    assert_eq!(expected, actual);
}

fn ark_g2_affine_to_blst_g2_affine(ark: &ark_bls12_381::G2Affine) -> blst::blst_p2_affine {
    let mut buf = Vec::with_capacity(192);
    ark.serialize_uncompressed(&mut buf).unwrap();
    let is_inf = (buf[191] & 0x40) != 0;
    if is_inf {
        buf[0] |= 0x40;
        buf[96] = 0;
        buf[191] = 0;
    } else {
        buf.as_mut_slice()[0..96].reverse();
        buf.as_mut_slice()[96..192].reverse();
    }
    let mut ret = blst::blst_p2_affine::default();
    unsafe { blst::blst_p2_deserialize(&mut ret, buf.as_ptr()); }
    ret
}

#[test]
fn test_ark_g2_affine_to_blst_g2_affine() {
    // Generator.
    let expected = unsafe { blst::blst_p2_affine_generator().read() };
    let actual = ark_g2_affine_to_blst_g2_affine(&ark_bls12_381::G2Affine::prime_subgroup_generator());
    unsafe { assert!(blst::blst_p2_affine_is_equal(&expected, &actual)); }

    // Generator negated.
    let mut blst_generator_neg = unsafe { blst::blst_p2_generator().read() };
    unsafe { blst::blst_p2_cneg(&mut blst_generator_neg, true); }
    let mut expected = blst::blst_p2_affine::default();
    unsafe { blst::blst_p2_to_affine(&mut expected, &blst_generator_neg); }
    let actual = ark_g2_affine_to_blst_g2_affine(&ark_bls12_381::G2Affine::prime_subgroup_generator().neg());
    unsafe { assert!(blst::blst_p2_affine_is_equal(&expected, &actual)); }

    // Infinity.
    let blst_generator = unsafe { blst::blst_p2_generator().read() };
    let scalar_0_bytes = vec![0_u8; 32];
    let mut blst_inf = blst::blst_p2::default();
    unsafe { blst::blst_p2_mult(&mut blst_inf, &blst_generator, scalar_0_bytes.as_ptr(), 256); }
    let mut expected = blst::blst_p2_affine::default();
    unsafe { blst::blst_p2_to_affine(&mut expected, &blst_inf); }
    let actual = ark_g2_affine_to_blst_g2_affine(&ark_bls12_381::G2Affine::zero());
    unsafe { assert!(blst::blst_p2_affine_is_equal(&expected, &actual)); }
}

fn blst_g2_affine_to_ark_g2_affine(blst_point: &blst::blst_p2_affine) -> ark_bls12_381::G2Affine {
    let mut buf = vec![0; 192];
    unsafe { blst::blst_p2_affine_serialize(buf.as_mut_ptr(), blst_point); }
    let is_inf = (buf[0] & 0x40) != 0;
    if is_inf {
        buf[191] |= 0x40;
        buf[96] = 0x01;
        buf[0] = 0;
    } else {
        buf.as_mut_slice()[0..96].reverse();
        buf.as_mut_slice()[96..192].reverse();
    }
    ark_bls12_381::G2Affine::deserialize_uncompressed(buf.as_slice()).unwrap()
}

#[test]
fn test_blst_g2_affine_to_ark_g2_affine() {
    // Generator.
    let blst_generator = unsafe { blst::blst_p2_affine_generator().read() };
    let actual = blst_g2_affine_to_ark_g2_affine(&blst_generator);
    let expected = ark_bls12_381::G2Affine::prime_subgroup_generator();
    assert_eq!(expected, actual);

    // Generator negated.
    let mut blst_generator_neg = unsafe { blst::blst_p2_generator().read() };
    unsafe { blst::blst_p2_cneg(&mut blst_generator_neg, true); }
    let mut blst_generator_neg_affine = blst::blst_p2_affine::default();
    unsafe { blst::blst_p2_to_affine(&mut blst_generator_neg_affine, &blst_generator_neg); }
    let actual = blst_g2_affine_to_ark_g2_affine(&blst_generator_neg_affine);
    let expected = ark_bls12_381::G2Affine::prime_subgroup_generator().neg();
    assert_eq!(expected, actual);

    // Infinity.
    let blst_generator = unsafe { blst::blst_p2_generator().read() };
    let scalar_0_bytes = vec![0_u8; 32];
    let mut blst_inf = blst::blst_p2::default();
    unsafe { blst::blst_p2_mult(&mut blst_inf, &blst_generator, scalar_0_bytes.as_ptr(), 256); }
    let mut blst_inf_affine = blst::blst_p2_affine::default();
    unsafe { blst::blst_p2_to_affine(&mut blst_inf_affine, &blst_inf); }
    let actual = blst_g2_affine_to_ark_g2_affine(&blst_inf_affine);
    let expected = ark_bls12_381::G2Affine::zero();
    assert_eq!(expected, actual);
}



fn element_multi_scalar_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(2, ty_args.len());
    let group_structure = structure_from_ty_arg!(context, &ty_args[0]);
    let scalar_structure = structure_from_ty_arg!(context, &ty_args[1]);
    let scalar_handles = pop_arg!(args, Vec<u64>);
    let num_scalars = scalar_handles.len();
    let element_handles = pop_arg!(args, Vec<u64>);
    let num_elements = element_handles.len();
    if num_elements != num_scalars {
        return Ok(NativeResult::err(InternalGas::zero(), abort_codes::NUM_ELEMENTS_SHOULD_MATCH_NUM_SCALARS));
    }
    match (group_structure, scalar_structure) {
        (Some(Structure::BLS12_381_G1), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            // Using blst multi-scalar multiplication API for better performance.
            let blst_g1_proj_points: Vec<blst::blst_p1> = element_handles.iter().map(|&handle|{
                let ark_point = borrow_bls12_381_g1!(context, handle as usize).into_affine();
                let blst_g1_affine = ark_g1_affine_to_blst_g1_affine(&ark_point);
                blst_g1_affine_to_proj(&blst_g1_affine)
            }).collect();

            let mut scalar_bytes: Vec<u8> = Vec::with_capacity(32 * num_scalars);
            for &scalar_handle in scalar_handles.iter() {
                let scalar = borrow_bls12_381_fr!(context, scalar_handle as usize);
                let buf = ark_serialize_uncompressed!(scalar);
                scalar_bytes.extend_from_slice(buf.as_slice());
            }

            let sum = blst::p1_affines::from(blst_g1_proj_points.as_slice()).mult(scalar_bytes.as_slice(), 256);
            let sum_affine = blst_g1_proj_to_affine(&sum);
            let ark_g1_affine = blst_g1_affine_to_ark_g1_affine(&sum_affine);
            let ark_g1_proj = ark_g1_affine.into_projective();
            let new_handle = store_bls12_381_g1!(context, ark_g1_proj);
            Ok(NativeResult::ok(
                (gas_params.bls12_381.g1_proj_mul + gas_params.bls12_381.g1_proj_add) * NumArgs::from(num_elements as u64), //TODO: update gas cost.
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        (Some(Structure::BLS12_381_G2), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            // Using blst multi-scalar multiplication API for better performance.
            let blst_points: Vec<blst::blst_p2> = element_handles.iter().map(|&handle|{
                let ark_point = borrow_bls12_381_g2!(context, handle as usize).into_affine();
                let blst_g2_affine = ark_g2_affine_to_blst_g2_affine(&ark_point);
                blst_g2_affine_to_proj(&blst_g2_affine)
            }).collect();

            let mut scalar_bytes: Vec<u8> = Vec::with_capacity(32 * num_scalars);
            for &scalar_handle in scalar_handles.iter() {
                let scalar = borrow_bls12_381_fr!(context, scalar_handle as usize);
                let buf = ark_serialize_uncompressed!(scalar);
                scalar_bytes.extend_from_slice(buf.as_slice());
            }

            let sum = blst::p2_affines::from(blst_points.as_slice()).mult(scalar_bytes.as_slice(), 256);
            let sum_affine = blst_g2_proj_to_affine(&sum);
            let ark_g2_affine = blst_g2_affine_to_ark_g2_affine(&sum_affine);
            let ark_g2_proj = ark_g2_affine.into_projective();
            let new_handle = store_bls12_381_g2!(context, ark_g2_proj);
            Ok(NativeResult::ok(
                (gas_params.bls12_381.g2_proj_mul + gas_params.bls12_381.g2_proj_add) * NumArgs::from(num_elements as u64), //TODO: update gas cost.
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn blst_g1_affine_to_proj(point: &blst::blst_p1_affine) -> blst::blst_p1 {
    let mut ret = blst::blst_p1::default();
    unsafe { blst::blst_p1_from_affine(&mut ret, point); }
    ret
}

fn blst_g2_affine_to_proj(point: &blst::blst_p2_affine) -> blst::blst_p2 {
    let mut ret = blst::blst_p2::default();
    unsafe { blst::blst_p2_from_affine(&mut ret, point); }
    ret
}

fn element_double_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_g1!(context, handle);
            let new_element = element.double();
            let new_handle = store_bls12_381_g1!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_double,
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_g2!(context, handle);
            let new_element = element.double();
            let new_handle = store_bls12_381_g2!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_double,
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_gt!(context, handle);
            let new_element = element.square();
            let new_handle = store_bls12_381_gt!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_square,
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn element_neg_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_g1!(context, handle);
            let new_element = element.neg();
            let new_handle = store_bls12_381_g1!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_neg,
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_g2!(context, handle);
            let new_element = element.neg();
            let new_handle = store_bls12_381_g2!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_neg,
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let element = borrow_bls12_381_gt!(context, handle);
            let new_element = element.inverse().unwrap();
            let new_handle = store_bls12_381_gt!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_inv,
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn pairing_product_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(3, ty_args.len());
    let g1 = structure_from_ty_arg!(context, &ty_args[0]);
    let g2 = structure_from_ty_arg!(context, &ty_args[1]);
    let gt = structure_from_ty_arg!(context, &ty_args[2]);
    let g2_handles = pop_arg!(args, Vec<u64>);
    let g1_handles = pop_arg!(args, Vec<u64>);
    match (g1, g2, gt) {
        (Some(Structure::BLS12_381_G1), Some(Structure::BLS12_381_G2), Some(Structure::BLS12_381_Gt)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let g1_prepared: Vec<ark_ec::models::bls12::g1::G1Prepared<Parameters>> = g1_handles
                .iter()
                .map(|&handle| {
                    let element = borrow_bls12_381_g1!(context, handle as usize);
                    ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(element.into_affine())
                })
                .collect();
            let g2_prepared: Vec<ark_ec::models::bls12::g2::G2Prepared<Parameters>> = g2_handles
                .iter()
                .map(|&handle| {
                    let element = borrow_bls12_381_g2!(context, handle as usize);
                    ark_ec::prepare_g2::<ark_bls12_381::Bls12_381>(element.into_affine())
                })
                .collect();

            let input_pairs: Vec<(
                ark_ec::models::bls12::g1::G1Prepared<Parameters>,
                ark_ec::models::bls12::g2::G2Prepared<Parameters>,
            )> = g1_prepared
                .into_iter()
                .zip(g2_prepared.into_iter())
                .collect();
            let new_element = ark_bls12_381::Bls12_381::product_of_pairings(input_pairs.as_slice());
            let new_handle = store_bls12_381_gt!(context, new_element);
            Ok(NativeResult::ok(
                (gas_params.bls12_381.g1_affine_to_prepared + gas_params.bls12_381.g2_affine_to_prepared + gas_params.bls12_381.pairing_product_per_pair) * NumArgs::new(g1_handles.len() as u64) + gas_params.bls12_381.pairing_product_base,
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::NOT_IMPLEMENTED))
        }
    }
}

const DST: &str = "";
const AUG: &str = "";

fn blst_g1_proj_to_affine(point: &blst::blst_p1) -> blst::blst_p1_affine {
    let mut ret = blst::blst_p1_affine::default();
    unsafe { blst::blst_p1_to_affine(&mut ret, point); }
    ret
}

fn blst_g2_proj_to_affine(point: &blst::blst_p2) -> blst::blst_p2_affine {
    let mut ret = blst::blst_p2_affine::default();
    unsafe { blst::blst_p2_to_affine(&mut ret, point); }
    ret
}

fn hash_to_blst_g1(bytes: &[u8]) -> blst::blst_p1 {
    let mut ret = blst::blst_p1::default();
    unsafe { blst::blst_hash_to_g1(&mut ret, bytes.as_ptr(), bytes.len(), DST.as_ptr(), DST.len(), AUG.as_ptr(), AUG.len()); }
    ret
}

fn hash_to_blst_g2(bytes: &[u8]) -> blst::blst_p2 {
    let mut ret = blst::blst_p2::default();
    unsafe { blst::blst_hash_to_g2(&mut ret, bytes.as_ptr(), bytes.len(), DST.as_ptr(), DST.len(), AUG.as_ptr(), AUG.len()); }
    ret
}

fn hash_to_element_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    assert_eq!(2, ty_args.len());
    let hash_alg = hash_alg_from_ty_arg!(context, &ty_args[0]);
    let target_group = structure_from_ty_arg!(context, &ty_args[1]);
    let bytes = pop_arg!(args, Vec<u8>);
    match (hash_alg, target_group) {
        (Some(HashAlg::SHA256), Some(Structure::BLS12_381_G1)) => {
            abort_if_feature_disabled!(context, FeatureFlag::SHA256_TO_GROUP);
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let blst_g1_proj = hash_to_blst_g1(bytes.as_slice());
            let blst_g1_affine = blst_g1_proj_to_affine(&blst_g1_proj);
            let ark_g1_affine = blst_g1_affine_to_ark_g1_affine(&blst_g1_affine);
            let ark_g1_proj = ark_g1_affine.into_projective();
            let new_handle = store_bls12_381_g1!(context, ark_g1_proj);
            Ok(NativeResult::ok(
                gas_params.bls12_381.blst_hash_to_g1_proj(bytes.len())
                    + (
                    gas_params.bls12_381.blst_g1_proj_to_affine
                        + gas_params.bls12_381.blst_g1_affine_ser
                        + gas_params.bls12_381.ark_g1_affine_deser_uncomp
                        + gas_params.bls12_381.ark_g1_affine_to_proj
                ) * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)]
            ))
        }
        (Some(HashAlg::SHA256), Some(Structure::BLS12_381_G2)) => {
            abort_if_feature_disabled!(context, FeatureFlag::SHA256_TO_GROUP);
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_GROUPS);
            let blst_g2_proj = hash_to_blst_g2(bytes.as_slice());
            let blst_g2_affine = blst_g2_proj_to_affine(&blst_g2_proj);
            let ark_g2_affine = blst_g2_affine_to_ark_g2_affine(&blst_g2_affine);
            let ark_g2_proj = ark_g2_affine.into_projective();
            let new_handle = store_bls12_381_g2!(context, ark_g2_proj);
            Ok(NativeResult::ok(
                gas_params.bls12_381.blst_hash_to_g2_proj(bytes.len())
                    + (gas_params.bls12_381.blst_g2_proj_to_affine
                    + gas_params.bls12_381.blst_g2_affine_ser
                    + gas_params.bls12_381.ark_g2_affine_deser_uncomp
                    + gas_params.bls12_381.ark_g2_affine_to_proj
                ) * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)]
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "element_serialize_uncompressed_internal",
            make_native_from_func(gas_params.clone(), element_serialize_uncompressed_internal),
        ),
        (
            "element_serialize_compressed_internal",
            make_native_from_func(gas_params.clone(), element_serialize_compressed_internal),
        ),
        (
            "element_deserialize_uncompressed_internal",
            make_native_from_func(gas_params.clone(), element_deserialize_uncompressed_internal),
        ),
        (
            "element_deserialize_compressed_internal",
            make_native_from_func(gas_params.clone(), element_deserialize_compressed_internal),
        ),
        (
            "scalar_deserialize_internal",
            make_native_from_func(gas_params.clone(), scalar_deserialize_internal),
        ),
        (
            "scalar_serialize_internal",
            make_native_from_func(gas_params.clone(), scalar_serialize_internal),
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
            make_native_from_func(gas_params.clone(), element_scalar_mul_internal),
        ),
        (
            "element_multi_scalar_mul_internal",
            make_native_from_func(gas_params.clone(), element_multi_scalar_mul_internal),
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
            "pairing_product_internal",
            make_native_from_func(gas_params.clone(), pairing_product_internal),
        ),
        (
            "group_order_internal",
            make_native_from_func(gas_params.clone(), group_order_internal),
        ),
        (
            "hash_to_element_internal",
            make_native_from_func(gas_params.clone(), hash_to_element_internal),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![
        (
            "random_element_internal",
            make_test_only_native_from_func(random_element_internal),
        ),
        (
            "random_scalar_internal",
            make_test_only_native_from_func(random_scalar_internal),
        ),
    ]);

    crate::natives::helpers::make_module_natives(natives)
}

#[derive(Copy, Clone)]
pub enum API {
    ScalarAdd,
    ScalarDeserialize,
    ScalarEq,
    ScalarFromU64,
    ScalarInv,
    ScalarMul,
    ScalarNeg,
    ScalarSerialize,
    ElementAdd,
    ElementDouble,
    ElementEq,
    ElementScalarMul,
    ElementMultiScalarMul,
    ElementNeg,
    ElementDeserializeCompressed,
    ElementDeserializeUncompressed,
    ElementSerializeCompressed,
    ElementSerializeUncompressed,
    GroupGenerator,
    GroupIdentity,
    GroupOrder,
    PairingProduct,
    HashToElement,
    RandomElement,
    RandomScalar,
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12_381_Fr,
    BLS12_381_G1,
    BLS12_381_G2,
    BLS12_381_Gt,
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum HashAlg {
    SHA256,
}

fn structure_from_type_tag(type_tag: &TypeTag) -> Option<Structure> {
    match type_tag.to_string().as_str() {
        "0x1::groups::BLS12_381_G1" => Some(Structure::BLS12_381_G1),
        "0x1::groups::BLS12_381_G2" => Some(Structure::BLS12_381_G2),
        "0x1::groups::BLS12_381_Gt" => Some(Structure::BLS12_381_Gt),
        "0x1::groups::BLS12_381_Fr" => Some(Structure::BLS12_381_Fr),
        _ => None
    }
}

fn hash_alg_from_type_tag(type_tag: &TypeTag) -> Option<HashAlg> {
    match type_tag.to_string().as_str() {
        "0x1::groups::SHA256" => Some(HashAlg::SHA256),
        _ => None
    }
}
