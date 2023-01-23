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
        if !$context.extensions().get::<Bls12381Context>().is_feature_enabled($feature) {
            return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
        }
    };
}

macro_rules! borrow_bls12_381_g1 {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<Bls12381Context>().g1_point_store.get($handle).unwrap()
    }}
}

macro_rules! store_bls12_381_g1 {
    ($context:expr, $new_point:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<Bls12381Context>();
        let ret = inner_ctxt.g1_point_store.len();
        inner_ctxt.g1_point_store.push($new_point);
        ret
    }}
}

pub mod abort_codes {
    pub const NOT_IMPLEMENTED: u64 = 2;
    pub const E_UNKNOWN_PAIRING: u64 = 3;
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
pub struct Bls12381Context {
    features: Features,
    fr_store: Vec<Fr>,
    g1_point_store: Vec<G1Projective>,
    g2_point_store: Vec<G2Projective>,
    gt_point_store: Vec<Fq12>,
    object_map: HashMap<Structure, Vec<Rc<dyn Any>>>,
}

impl Bls12381Context {
    pub fn new(features: Features) -> Self {
        Self {
            fr_store: vec![],
            g1_point_store: vec![],
            g2_point_store: vec![],
            gt_point_store: vec![],
            features,
            object_map: HashMap::new(),
        }
    }

    pub fn is_feature_enabled(&self, flag: FeatureFlag) -> bool {
        self.features.is_enabled(flag)
    }

    pub fn add_scalar(&mut self, scalar: Fr) -> usize {
        let ret = self.fr_store.len();
        self.fr_store.push(scalar);
        ret
    }

    pub fn get_scalar(&self, handle: usize) -> &Fr {
        self.fr_store.get(handle).unwrap()
    }

    pub fn add_g1_point(&mut self, p0: G1Projective) -> usize {
        let ret = self.g1_point_store.len();
        self.g1_point_store.push(p0);
        ret
    }

    pub fn get_g1_point(&self, handle: usize) -> &G1Projective {
        self.g1_point_store.get(handle).unwrap()
    }

    pub fn add_g2_point(&mut self, p0: G2Projective) -> usize {
        let ret = self.g2_point_store.len();
        self.g2_point_store.push(p0);
        ret
    }

    pub fn get_g2_point(&self, handle: usize) -> &G2Projective {
        self.g2_point_store.get(handle).unwrap()
    }

    pub fn add_gt_point(&mut self, point: Fq12) -> usize {
        let ret = self.gt_point_store.len();
        self.gt_point_store.push(point);
        ret
    }

    pub fn get_gt_point(&self, handle: usize) -> &Fq12 {
        self.gt_point_store.get(handle).unwrap()
    }

    pub fn borrow(&self, structure: &Structure, handle: usize) -> Option<Rc<dyn Any>> {
        self.object_map.get(&structure)
            .map_or(None, |objs| objs.get(handle))
            .map_or(None, |obj| Some(obj.clone()))
    }

    pub fn append(&mut self, structure: Structure, obj: Rc<dyn Any>) -> usize {
        let obj_list = self.object_map.entry(structure).or_insert_with(Vec::new);
        let ret = obj_list.len();
        obj_list.push(obj);
        ret
    }
}

fn element_serialize_uncompressed_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    if !context.extensions().get::<Bls12381Context>().is_feature_enabled(FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS) {
        return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
    }
    assert_eq!(1, ty_args.len());
    let type_tag = context
    .type_to_type_tag(ty_args.get(0).unwrap())?
    .to_string();
    let handle = pop_arg!(args, u64) as usize;

    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let mut buf = vec![];
            context.extensions().get::<Bls12381Context>()
                .get_g1_point(handle)
                .into_affine()
                .serialize_uncompressed(&mut buf)
                .unwrap();
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_to_affine + gas_params.bls12_381.g1_affine_serialize_uncompressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
            let mut buf = vec![];
            context.extensions().get::<Bls12381Context>()
                .get_g2_point(handle)
                .into_affine()
                .serialize_uncompressed(&mut buf)
                .unwrap();
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_to_affine + gas_params.bls12_381.g2_affine_serialize_uncompressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let mut buf = vec![];
            context.extensions().get::<Bls12381Context>()
                .get_gt_point(handle)
                .serialize_uncompressed(&mut buf)
                .unwrap();
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
    if context.extensions().get::<Bls12381Context>().features.is_enabled(FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS) {
        return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
    }
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(handle)
                .into_affine()
                .serialize(&mut buf)
                .unwrap();
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_to_affine + gas_params.bls12_381.g1_affine_serialize_compressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(handle)
                .into_affine()
                .serialize(&mut buf)
                .unwrap();
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_to_affine + gas_params.bls12_381.g2_affine_serialize_compressed,
                smallvec![Value::vector_u8(buf)],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle)
                .serialize(&mut buf)
                .unwrap();
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
    if !context.extensions().get::<Bls12381Context>().features.is_enabled(FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS) {
        return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
    }
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let bytes = pop_arg!(args, Vec<u8>);
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let cost = (gas_params.bls12_381.ark_g1_affine_deser_uncomp + gas_params.bls12_381.ark_g1_affine_to_proj) * NumArgs::one();
            let point = ark_bls12_381::G1Affine::deserialize_uncompressed(bytes.as_slice());
            match point {
                Ok(point) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<Bls12381Context>()
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
        "0x1::groups::BLS12_381_G2" => {
            let cost = (gas_params.bls12_381.ark_g2_affine_deser_uncomp + gas_params.bls12_381.ark_g2_affine_to_proj) * NumArgs::one();
            let point = ark_bls12_381::G2Affine::deserialize_uncompressed(bytes.as_slice());
            match point {
                Ok(point) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<Bls12381Context>()
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
        "0x1::groups::BLS12_381_Gt" => {
            let cost = (gas_params.bls12_381.fq12_deserialize + gas_params.bls12_381.ark_fq12_pow_fr + gas_params.bls12_381.fq12_eq) * NumArgs::one();
            let point = Fq12::deserialize(bytes.as_slice());
            match point {
                Ok(point) => {
                    let r_buf = hex::decode(
                        "01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73",
                    )
                        .unwrap();
                    let r =
                        ark_ff::BigInteger256::deserialize_uncompressed(r_buf.as_slice()).unwrap();
                    if Fq12::one() == point.pow(r) {
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
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
    if !context.extensions().get::<Bls12381Context>().features.is_enabled(FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS) {
        return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
    }

    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let bytes = pop_arg!(args, Vec<u8>);
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let cost = (gas_params.bls12_381.ark_g1_affine_deser_comp + gas_params.bls12_381.ark_g1_affine_to_proj) * NumArgs::one();
            match ark_bls12_381::G1Affine::deserialize(bytes.as_slice()) {
                Ok(point) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<Bls12381Context>()
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
        "0x1::groups::BLS12_381_G2" => {
            let cost = (gas_params.bls12_381.ark_g2_affine_deser_comp + gas_params.bls12_381.ark_g2_affine_to_proj) * NumArgs::one();
            let point = ark_bls12_381::G2Affine::deserialize(bytes.as_slice());
            match point {
                Ok(point) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<Bls12381Context>()
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
        "0x1::groups::BLS12_381_Gt" => {
            let cost = (gas_params.bls12_381.fq12_deserialize + gas_params.bls12_381.ark_fq12_pow_fr + gas_params.bls12_381.fq12_eq) * NumArgs::one();
            let point = Fq12::deserialize(bytes.as_slice());
            match point {
                Ok(point) => {
                    let r_buf = hex::decode(
                        "01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73",
                    )
                        .unwrap();
                    let r =
                        ark_ff::BigInteger256::deserialize_uncompressed(r_buf.as_slice()).unwrap();
                    if Fq12::one() == point.pow(r) {
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let bytes = pop_arg!(args, Vec<u8>);
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
            let scalar = Fr::deserialize_uncompressed(bytes.as_slice());
            match scalar {
                Ok(scalar) => {
                    let handle = context
                        .extensions_mut()
                        .get_mut::<Bls12381Context>()
                        .add_scalar(scalar);
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
            let mut buf = vec![];
            context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(handle)
                .serialize_uncompressed(&mut buf).unwrap();
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let value = pop_arg!(args, u64);
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_scalar(ark_bls12_381::Fr::from(value as u128));
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
            let scalar_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(handle_1);
            let scalar_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(handle_2);
            let result = scalar_1.mul(scalar_2);
            let result_handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_scalar(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fr_mul,
                smallvec![Value::u64(result_handle as u64)],
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
            let result = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(handle)
                .neg();
            let result_handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_scalar(result);
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
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
            let op_result = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(handle)
                .inverse();
            match op_result {
                Some(scalar) => {
                    let ret = context
                        .extensions_mut()
                        .get_mut::<Bls12381Context>()
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
            let scalar_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(handle_1);
            let scalar_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(handle_2);
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let point = G1Projective::zero();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_infinity,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
            let point = G2Projective::zero();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_infinity,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let point = Fq12::one();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(point);
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
        "0x1::groups::BLS12_381_G1" => {
            let point = G1Projective::prime_subgroup_generator();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_generator,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
            let point = G2Projective::prime_subgroup_generator();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(point);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_generator,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let point = BLS12381_GT_GENERATOR.clone();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(point);
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" | "0x1::groups::BLS12_381_G2" | "0x1::groups::BLS12_381_Gt" => {
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" | "0x1::groups::BLS12_381_G2" | "0x1::groups::BLS12_381_Gt" => {
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_Fr" => {
            let r = Fr::rand(&mut test_rng());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_scalar(r);
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let point = G1Projective::rand(&mut test_rng());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(point);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
            let point = G2Projective::rand(&mut test_rng());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(point);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let k = Fr::rand(&mut test_rng());
            let point = BLS12381_GT_GENERATOR.clone().pow(k.into_repr());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(point);
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_eq,
                smallvec![Value::bool(point_1.eq(point_2))],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_eq,
                smallvec![Value::bool(point_1.eq(point_2))],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle_2);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_eq * NumArgs::one(),
                smallvec![Value::bool(point_1.eq(point_2))],
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_add * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
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
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_add * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let point_1 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle_1);
            let point_2 = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(handle_2);
            let result = point_1.clone() * point_2.clone();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_mul * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn element_mul_scalar_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(2, ty_args.len());
    let group_type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let scalar_type_tag = context
        .type_to_type_tag(ty_args.get(1).unwrap())?
        .to_string();
    let point_handle = pop_arg!(args, u64) as usize;
    let scalar_handle = pop_arg!(args, u64) as usize;
    match (group_type_tag.as_str(), scalar_type_tag.as_str()) {
        ("0x1::groups::BLS12_381_G1", "0x1::groups::BLS12_381_Fr") => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(point_handle);
            let scalar = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(scalar_handle);
            let result = point.mul(scalar.into_repr());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_mul * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        ("0x1::groups::BLS12_381_G2", "0x1::groups::BLS12_381_Fr") => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(point_handle);
            let scalar = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(scalar_handle);
            let result = point.mul(scalar.into_repr());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_mul * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        ("0x1::groups::BLS12_381_Gt", "0x1::groups::BLS12_381_Fr") => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(point_handle);
            let scalar = context
                .extensions()
                .get::<Bls12381Context>()
                .get_scalar(scalar_handle);
            let result = point.pow(scalar.into_repr().as_ref());
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(result);
            Ok(NativeResult::ok(
                (gas_params.bls12_381.fr_to_repr + gas_params.bls12_381.ark_fq12_pow_fr) * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
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
    assert_eq!(2, ty_args.len());
    let group_type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let scalar_type_tag = context
        .type_to_type_tag(ty_args.get(1).unwrap())?
        .to_string();
    let point_handles = pop_arg!(args, Vec<u64>);
    let num_points = point_handles.len();
    let scalar_handles = pop_arg!(args, Vec<u64>);
    let num_scalars = scalar_handles.len();
    if num_points == num_scalars {
        match (group_type_tag.as_str(), scalar_type_tag.as_str()) {
            ("0x1::groups::BLS12_381_G1", "0x1::groups::BLS12_381_Fr") => {
                // Using blst multi-scalar multiplication API for better performance.
                let blst_g1_proj_points: Vec<blst::blst_p1> = point_handles.iter().map(|&handle|{
                    let ark_point = context
                        .extensions()
                        .get::<Bls12381Context>()
                        .get_g1_point(handle as usize)
                        .into_affine();
                    let blst_g1_affine = ark_g1_affine_to_blst_g1_affine(&ark_point);
                    blst_g1_affine_to_proj(&blst_g1_affine)
                }).collect();

                let mut scalar_bytes: Vec<u8> = Vec::with_capacity(32 * num_scalars);
                for &scalar_handle in scalar_handles.iter() {
                    let mut buf = Vec::with_capacity(32);
                    context
                        .extensions()
                        .get::<Bls12381Context>()
                        .get_scalar(scalar_handle as usize).serialize_uncompressed(&mut buf).unwrap();
                    scalar_bytes.extend_from_slice(buf.as_slice());
                }

                let sum = blst::p1_affines::from(blst_g1_proj_points.as_slice()).mult(scalar_bytes.as_slice(), 256);
                let sum_affine = blst_g1_proj_to_affine(&sum);
                let ark_g1_affine = blst_g1_affine_to_ark_g1_affine(&sum_affine);
                let ark_g1_proj = ark_g1_affine.into_projective();
                let handle = context
                    .extensions_mut()
                    .get_mut::<Bls12381Context>()
                    .add_g1_point(ark_g1_proj);
                Ok(NativeResult::ok(
                    (gas_params.bls12_381.g1_proj_mul + gas_params.bls12_381.g1_proj_add) * NumArgs::from(num_points as u64), //TODO: update gas cost.
                    smallvec![Value::u64(handle as u64)],
                ))
            }
            ("0x1::groups::BLS12_381_G2", "0x1::groups::BLS12_381_Fr") => {
                // Using blst multi-scalar multiplication API for better performance.
                let blst_points: Vec<blst::blst_p2> = point_handles.iter().map(|&handle|{
                    let ark_point = context
                        .extensions()
                        .get::<Bls12381Context>()
                        .get_g2_point(handle as usize)
                        .into_affine();
                    let blst_g2_affine = ark_g2_affine_to_blst_g2_affine(&ark_point);
                    let mut blst_g2_proj = blst::blst_p2::default();
                    unsafe { blst::blst_p2_from_affine(&mut blst_g2_proj, &blst_g2_affine); }
                    blst_g2_proj
                }).collect();

                let mut scalar_bytes: Vec<u8> = Vec::with_capacity(32 * num_scalars);
                for &scalar_handle in scalar_handles.iter() {
                    let mut buf = Vec::with_capacity(32);
                    context
                        .extensions()
                        .get::<Bls12381Context>()
                        .get_scalar(scalar_handle as usize).serialize_uncompressed(&mut buf).unwrap();
                    scalar_bytes.extend_from_slice(buf.as_slice());
                }

                let sum = blst::p2_affines::from(blst_points.as_slice()).mult(scalar_bytes.as_slice(), 256);
                let mut sum_affine = blst::blst_p2_affine::default();
                unsafe { blst::blst_p2_to_affine(&mut sum_affine, &sum); }
                let result = blst_g2_affine_to_ark_g2_affine(&sum_affine).into_projective();
                let handle = context
                    .extensions_mut()
                    .get_mut::<Bls12381Context>()
                    .add_g2_point(result);
                Ok(NativeResult::ok(
                    (gas_params.bls12_381.g2_proj_mul + gas_params.bls12_381.g2_proj_add) * NumArgs::from(num_points as u64), //TODO: update gas cost.
                    smallvec![Value::u64(handle as u64)],
                ))
            }
            ("0x1::groups::BLS12_381_Gt", "0x1::groups::BLS12_381_Fr") => {
                let elements = point_handles.iter().map(|&handle|{
                    context
                        .extensions()
                        .get::<Bls12381Context>()
                        .get_gt_point(handle as usize)
                        .clone()
                });
                let scalars = scalar_handles.iter().map(|&handle|{
                    context
                        .extensions()
                        .get::<Bls12381Context>()
                        .get_scalar(handle as usize)
                        .clone()
                });

                let mut result = Fq12::one();
                for (element, scalar) in elements.zip(scalars) {
                    result.mul_assign(element.pow(scalar.into_repr()));
                }

                let handle = context
                    .extensions_mut()
                    .get_mut::<Bls12381Context>()
                    .add_gt_point(result);
                Ok(NativeResult::ok(
                    (gas_params.bls12_381.fr_to_repr + gas_params.bls12_381.ark_fq12_pow_fr + gas_params.bls12_381.fq12_mul) * NumArgs::from(num_points as u64),
                    smallvec![Value::u64(handle as u64)],
                ))
            }
            _ => {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
    } else {
        Ok(NativeResult::err(InternalGas::zero(), abort_codes::NUM_ELEMENTS_SHOULD_MATCH_NUM_SCALARS))
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let point_handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(point_handle);
            let result = ProjectiveCurve::double(point);
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_double,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(point_handle);
            let result = ProjectiveCurve::double(point);
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_double,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let element = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(point_handle);
            let result = element.square();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_square,
                smallvec![Value::u64(handle as u64)],
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
    assert_eq!(1, ty_args.len());
    let type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let point_handle = pop_arg!(args, u64) as usize;
    match type_tag.as_str() {
        "0x1::groups::BLS12_381_G1" => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g1_point(point_handle);
            let result = point.neg();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g1_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g1_proj_neg,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_G2" => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_g2_point(point_handle);
            let result = point.neg();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_g2_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.g2_proj_neg,
                smallvec![Value::u64(handle as u64)],
            ))
        }
        "0x1::groups::BLS12_381_Gt" => {
            let point = context
                .extensions()
                .get::<Bls12381Context>()
                .get_gt_point(point_handle);
            let result = point.inverse().unwrap();
            let handle = context
                .extensions_mut()
                .get_mut::<Bls12381Context>()
                .add_gt_point(result);
            Ok(NativeResult::ok(
                gas_params.bls12_381.fq12_inv,
                smallvec![Value::u64(handle as u64)],
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
    let g2_handles = pop_arg!(args, Vec<u64>);
    let g1_handles = pop_arg!(args, Vec<u64>);
    match (
        type_tag_0.as_str(),
        type_tag_1.as_str(),
        type_tag_2.as_str(),
    ) {
        ("0x1::groups::BLS12_381_G1", "0x1::groups::BLS12_381_G2", "0x1::groups::BLS12_381_Gt") => {
            let g1_prepared: Vec<ark_ec::models::bls12::g1::G1Prepared<Parameters>> = g1_handles
                .iter()
                .map(|&handle| {
                    let element = context
                        .extensions()
                        .get::<Bls12381Context>()
                        .get_g1_point(handle as usize);
                    ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(element.into_affine())
                })
                .collect();
            let g2_prepared: Vec<ark_ec::models::bls12::g2::G2Prepared<Parameters>> = g2_handles
                .iter()
                .map(|&handle| {
                    let element = context
                        .extensions()
                        .get::<Bls12381Context>()
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
                .get_mut::<Bls12381Context>()
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

fn hash_to_scalar(bytes: &[u8]) -> ark_ff::BigInteger256 {
    let digest = Sha256::digest(bytes).to_vec();
    ark_ff::BigInteger256::deserialize_uncompressed(digest.as_slice()).unwrap()
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
    let hasher_type_tag = context
        .type_to_type_tag(ty_args.get(0).unwrap())?
        .to_string();
    let group_type_tag = context
        .type_to_type_tag(ty_args.get(1).unwrap())?
        .to_string();
    let bytes = pop_arg!(args, Vec<u8>);
    match (hasher_type_tag.as_str(), group_type_tag.as_str()) {
        ("0x1::groups::SHA256", "0x1::groups::BLS12_381_G1") => {
            let blst_g1_proj = hash_to_blst_g1(bytes.as_slice());
            let blst_g1_affine = blst_g1_proj_to_affine(&blst_g1_proj);
            let ark_g1_affine = blst_g1_affine_to_ark_g1_affine(&blst_g1_affine);
            let ark_g1_proj = ark_g1_affine.into_projective();
            let handle = store_bls12_381_g1!(context, ark_g1_proj);
            Ok(NativeResult::ok(
                gas_params.bls12_381.blst_hash_to_g1_proj(bytes.len())
                    + (
                    gas_params.bls12_381.blst_g1_proj_to_affine
                        + gas_params.bls12_381.blst_g1_affine_ser
                        + gas_params.bls12_381.ark_g1_affine_deser_uncomp
                        + gas_params.bls12_381.ark_g1_affine_to_proj
                ) * NumArgs::one(),
                smallvec![Value::u64(handle as u64)]
            ))
        }
        ("0x1::groups::SHA256", "0x1::groups::BLS12_381_G2") => {
            let blst_g2_proj = hash_to_blst_g2(bytes.as_slice());
            let blst_g2_affine = blst_g2_proj_to_affine(&blst_g2_proj);
            let ark_g2_affine = blst_g2_affine_to_ark_g2_affine(&blst_g2_affine);
            let ark_g2_proj = ark_g2_affine.into_projective();
            let handle = context.extensions_mut().get_mut::<Bls12381Context>().add_g2_point(ark_g2_proj);
            Ok(NativeResult::ok(
                gas_params.bls12_381.blst_hash_to_g2_proj(bytes.len())
                    + (gas_params.bls12_381.blst_g2_proj_to_affine
                    + gas_params.bls12_381.blst_g2_affine_ser
                    + gas_params.bls12_381.ark_g2_affine_deser_uncomp
                    + gas_params.bls12_381.ark_g2_affine_to_proj
                ) * NumArgs::one(),
                smallvec![Value::u64(handle as u64)]
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn feature_for_api(api: API) -> FeatureFlag {
    match api {
        API::ElementSerializeCompressed
        | API::ScalarAdd
        | API::ScalarDeserialize
        | API::ScalarEq
        | API::ScalarFromU64
        | API::ScalarInv
        | API::ScalarMul
        | API::ScalarNeg
        | API::ScalarSerialize
        | API::ElementAdd
        | API::ElementDouble
        | API::ElementEq
        | API::ElementScalarMul
        | API::ElementMultiScalarMul
        | API::ElementNeg
        | API::ElementDeserializeCompressed
        | API::ElementDeserializeUncompressed
        | API::ElementSerializeUncompressed
        | API::GroupGenerator
        | API::GroupIdentity
        | API::GroupOrder
        | API::PairingProduct
        | API::HashToElement
        | API::RandomElement
        | API::RandomScalar => GENERIC_GROUP_BASIC_OPERATIONS,
    }
}

fn feature_for_structure(structure: Structure) -> FeatureFlag {
    match structure {
        Structure::BLS12_381_Fr
        | Structure::BLS12_381_G1
        | Structure::BLS12_381_G2
        | Structure::BLS12_381_Gt => {
            FeatureFlag::BLS12_381_GROUPS
        }
    }
}

fn feature_for_hash_alg(hash_alg: HashAlg) -> FeatureFlag {
    match hash_alg {
        HashAlg::SHA256 => FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS
    }
}

fn uber(
    api_name: API,
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_GROUP_BASIC_OPERATIONS);
    if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_api(api_name)) {
        return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
    }
    let type_tags: Vec<TypeTag> = ty_args.iter()
        .map(|ty_arg| context.type_to_type_tag(ty_arg).unwrap())
        .collect();
    match api_name {
        API::ElementAdd => {
            assert_eq!(1, type_tags.len());
            let handle_1 = pop_arg!(args, u64) as usize;
            let handle_0 = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let point_0 = obj_0.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let point_1 = obj_1.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
                        let new_point = point_0.add(point_1);
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_add * NumArgs::one(),
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_G2 => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let point_0 = obj_0.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let point_1 = obj_1.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
                        let new_point = point_0.add(point_1);
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_add * NumArgs::one(),
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_Gt => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let element_0 = obj_0.downcast_ref::<ark_bls12_381::Fq12>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let element_1 = obj_1.downcast_ref::<ark_bls12_381::Fq12>().unwrap();
                        let new_element = element_0.mul(element_1);
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(new_element));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fq12_mul * NumArgs::one(),
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementDeserializeUncompressed => {
            assert_eq!(1, type_tags.len());
            let bytes = pop_arg!(args, Vec<u8>);
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 => {
                        let cost = (gas_params.bls12_381.ark_g1_affine_deser_uncomp + gas_params.bls12_381.ark_g1_affine_to_proj) * NumArgs::one();
                        match ark_bls12_381::G1Affine::deserialize_uncompressed(bytes.as_slice()) {
                            Ok(point) => {
                                let point_proj = point.into_projective();
                                let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(point_proj));
                                Ok(NativeResult::ok(
                                    cost,
                                    smallvec![Value::bool(true), Value::u64(new_handle as u64)],
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
                    Structure::BLS12_381_G2 => {
                        let cost = (gas_params.bls12_381.ark_g2_affine_deser_uncomp + gas_params.bls12_381.ark_g2_affine_to_proj) * NumArgs::one();
                        match ark_bls12_381::G2Affine::deserialize_uncompressed(bytes.as_slice()) {
                            Ok(point) => {
                                let point_proj = point.into_projective();
                                let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(point_proj));
                                Ok(NativeResult::ok(
                                    cost,
                                    smallvec![Value::bool(true), Value::u64(new_handle as u64)],
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
                    Structure::BLS12_381_Gt => {
                        let cost = (gas_params.bls12_381.fq12_deserialize + gas_params.bls12_381.ark_fq12_pow_fr + gas_params.bls12_381.fq12_eq) * NumArgs::one();
                        match ark_bls12_381::Fq12::deserialize_uncompressed(bytes.as_slice()) {
                            Ok(element) => {
                                let r_buf = hex::decode(
                                    "01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73",
                                )
                                    .unwrap();
                                let r = ark_ff::BigInteger256::deserialize_uncompressed(r_buf.as_slice()).unwrap();
                                if Fq12::one() == element.pow(r) {
                                    let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(element));
                                    Ok(NativeResult::ok(
                                        cost,
                                        smallvec![Value::bool(true), Value::u64(new_handle as u64)],
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
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementDeserializeCompressed => {
            assert_eq!(1, type_tags.len());
            let bytes = pop_arg!(args, Vec<u8>);
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 => {
                        let cost = (gas_params.bls12_381.ark_g1_affine_deser_comp + gas_params.bls12_381.ark_g1_affine_to_proj) * NumArgs::one();
                        match ark_bls12_381::G1Affine::deserialize(bytes.as_slice()) {
                            Ok(point) => {
                                let point_proj = point.into_projective();
                                let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(point_proj));
                                Ok(NativeResult::ok(
                                    cost,
                                    smallvec![Value::bool(true), Value::u64(new_handle as u64)],
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
                    Structure::BLS12_381_G2 => {
                        let cost = (gas_params.bls12_381.ark_g2_affine_deser_comp + gas_params.bls12_381.ark_g2_affine_to_proj) * NumArgs::one();
                        match ark_bls12_381::G2Affine::deserialize(bytes.as_slice()) {
                            Ok(point) => {
                                let point_proj = point.into_projective();
                                let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(point_proj));
                                Ok(NativeResult::ok(
                                    cost,
                                    smallvec![Value::bool(true), Value::u64(new_handle as u64)],
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
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementDouble => {
            assert_eq!(1, type_tags.len());
            let handle = pop_arg!(args, u64);
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 => {
                        let obj = context
                            .extensions()
                            .get::<Bls12381Context>()
                            .borrow(&structure, handle as usize).unwrap();
                        let point = obj.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
                        let new_point = ProjectiveCurve::double(point);
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_double,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_G2 => {
                        let obj = context
                            .extensions()
                            .get::<Bls12381Context>()
                            .borrow(&structure, handle as usize).unwrap();
                        let point = obj.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
                        let new_point = ProjectiveCurve::double(point);
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_double,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementEq => {
            assert_eq!(1, type_tags.len());
            let handle_1 = pop_arg!(args, u64) as usize;
            let handle_0 = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let point_0 = obj_0.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let point_1 = obj_1.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_eq,
                            smallvec![Value::bool(point_0.eq(point_1))],
                        ))
                    }
                    Structure::BLS12_381_G2 => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let point_0 = obj_0.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let point_1 = obj_1.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_eq,
                            smallvec![Value::bool(point_0.eq(point_1))],
                        ))
                    }
                    Structure::BLS12_381_Gt => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let element_0 = obj_0.downcast_ref::<ark_bls12_381::Fq12>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let element_1 = obj_1.downcast_ref::<ark_bls12_381::Fq12>().unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fq12_eq * NumArgs::one(),
                            smallvec![Value::bool(element_0.eq(element_1))],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementMultiScalarMul => {
            assert_eq!(2, type_tags.len());
            let scalar_handles = pop_arg!(args, Vec<u64>);
            let num_scalars = scalar_handles.len();
            let point_handles = pop_arg!(args, Vec<u64>);
            let num_points = point_handles.len();
            if let (Some(structure_0), Some(structure_1)) = (structure_from_type_tag(&type_tags[0]), structure_from_type_tag(&type_tags[1])) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure_0)) || !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure_1)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match (structure_0, structure_1) {
                    (Structure::BLS12_381_G1, Structure::BLS12_381_Fr) => {
                        // Using blst multi-scalar multiplication API for better performance.
                        let blst_g1_proj_points: Vec<blst::blst_p1> = point_handles.iter().map(|&handle|{
                            let ark_point = context
                                .extensions()
                                .get::<Bls12381Context>()
                                .borrow(&structure_0,handle as usize).unwrap()
                                .downcast_ref::<ark_bls12_381::G1Projective>().unwrap()
                                .into_affine();
                            let blst_g1_affine = ark_g1_affine_to_blst_g1_affine(&ark_point);
                            blst_g1_affine_to_proj(&blst_g1_affine)
                        }).collect();

                        let mut scalar_bytes: Vec<u8> = Vec::with_capacity(32 * num_scalars);
                        for &scalar_handle in scalar_handles.iter() {
                            let mut buf = Vec::with_capacity(32);
                            context
                                .extensions()
                                .get::<Bls12381Context>()
                                .borrow(&structure_1, scalar_handle as usize).unwrap()
                                .downcast_ref::<ark_bls12_381::Fr>().unwrap()
                                .serialize_uncompressed(&mut buf).unwrap();
                            scalar_bytes.extend_from_slice(buf.as_slice());
                        }

                        let sum = blst::p1_affines::from(blst_g1_proj_points.as_slice()).mult(scalar_bytes.as_slice(), 256);
                        let sum_affine = blst_g1_proj_to_affine(&sum);
                        let ark_g1_affine = blst_g1_affine_to_ark_g1_affine(&sum_affine);
                        let ark_g1_proj = ark_g1_affine.into_projective();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure_0, Rc::new(ark_g1_proj));
                        Ok(NativeResult::ok(
                            (gas_params.bls12_381.g1_proj_mul + gas_params.bls12_381.g1_proj_add) * NumArgs::from(num_points as u64), //TODO: update gas cost.
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    (Structure::BLS12_381_G2, Structure::BLS12_381_Fr) => {
                        // Using blst multi-scalar multiplication API for better performance.
                        let blst_g2_proj_points: Vec<blst::blst_p2> = point_handles.iter().map(|&handle|{
                            let ark_point = context
                                .extensions()
                                .get::<Bls12381Context>()
                                .borrow(&structure_0,handle as usize).unwrap()
                                .downcast_ref::<ark_bls12_381::G2Projective>().unwrap()
                                .into_affine();
                            let blst_g2_affine = ark_g2_affine_to_blst_g2_affine(&ark_point);
                            blst_g2_affine_to_proj(&blst_g2_affine)
                        }).collect();

                        let mut scalar_bytes: Vec<u8> = Vec::with_capacity(32 * num_scalars);
                        for &scalar_handle in scalar_handles.iter() {
                            let mut buf = Vec::with_capacity(32);
                            context
                                .extensions()
                                .get::<Bls12381Context>()
                                .borrow(&structure_1, scalar_handle as usize).unwrap()
                                .downcast_ref::<ark_bls12_381::Fr>().unwrap()
                                .serialize_uncompressed(&mut buf).unwrap();
                            scalar_bytes.extend_from_slice(buf.as_slice());
                        }

                        let sum = blst::p2_affines::from(blst_g2_proj_points.as_slice()).mult(scalar_bytes.as_slice(), 256);
                        let sum_affine = blst_g2_proj_to_affine(&sum);
                        let ark_g2_affine = blst_g2_affine_to_ark_g2_affine(&sum_affine);
                        let ark_g2_proj = ark_g2_affine.into_projective();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure_0, Rc::new(ark_g2_proj));
                        Ok(NativeResult::ok(
                            (gas_params.bls12_381.g2_proj_mul + gas_params.bls12_381.g2_proj_add) * NumArgs::from(num_points as u64), //TODO: update gas cost.
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementNeg => {
            assert_eq!(1, type_tags.len());
            let handle = pop_arg!(args, u64);
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 => {
                        let obj = context
                            .extensions()
                            .get::<Bls12381Context>()
                            .borrow(&structure, handle as usize).unwrap();
                        let point = obj.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
                        let new_point = point.neg();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_neg,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_G2 => {
                        let obj = context
                            .extensions()
                            .get::<Bls12381Context>()
                            .borrow(&structure, handle as usize).unwrap();
                        let point = obj.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
                        let new_point = point.neg();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_neg,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_Gt => {
                        let obj = context
                            .extensions()
                            .get::<Bls12381Context>()
                            .borrow(&structure, handle as usize).unwrap();
                        let element = obj.downcast_ref::<ark_bls12_381::Fq12>().unwrap();
                        let new_element = element.inverse().unwrap();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(new_element));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fq12_inv,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementScalarMul => {
            assert_eq!(2, type_tags.len());
            let handle_1 = pop_arg!(args, u64) as usize;
            let handle_0 = pop_arg!(args, u64) as usize;
            if let (Some(group_structure), Some(scalar_structure)) = (structure_from_type_tag(&type_tags[0]), structure_from_type_tag(&type_tags[1])) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(group_structure)) || !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(scalar_structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match (group_structure, scalar_structure) {
                    (Structure::BLS12_381_G1, Structure::BLS12_381_Fr) => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&group_structure, handle_0).unwrap();
                        let point = obj_0.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&scalar_structure, handle_1).unwrap();
                        let scalar = obj_1.downcast_ref::<ark_bls12_381::Fr>().unwrap();
                        let new_point = point.mul(scalar.into_repr());
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(group_structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_mul * NumArgs::one(),
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    (Structure::BLS12_381_G2, Structure::BLS12_381_Fr) => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&group_structure, handle_0).unwrap();
                        let point = obj_0.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&scalar_structure, handle_1).unwrap();
                        let scalar = obj_1.downcast_ref::<ark_bls12_381::Fr>().unwrap();
                        let new_point = point.mul(scalar.into_repr());
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(group_structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_mul * NumArgs::one(),
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    (Structure::BLS12_381_Gt, Structure::BLS12_381_Fr) => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&group_structure, handle_0).unwrap();
                        let element = obj_0.downcast_ref::<ark_bls12_381::Fq12>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&scalar_structure, handle_1).unwrap();
                        let scalar = obj_1.downcast_ref::<ark_bls12_381::Fr>().unwrap();
                        let new_point = element.pow(scalar.into_repr());
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(group_structure, Rc::new(new_point));
                        Ok(NativeResult::ok(
                            (gas_params.bls12_381.fr_to_repr + gas_params.bls12_381.ark_fq12_pow_fr) * NumArgs::one(),
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementSerializeCompressed => {
            assert_eq!(1, type_tags.len());
            let handle = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                let obj = context
                    .extensions()
                    .get::<Bls12381Context>()
                    .borrow(&structure, handle).unwrap();
                let mut buf = vec![];
                match structure {
                    Structure::BLS12_381_G1 => {
                        obj.downcast_ref::<ark_bls12_381::G1Projective>().unwrap().into_affine().serialize(&mut buf).unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_to_affine + gas_params.bls12_381.g1_affine_serialize_compressed,
                            smallvec![Value::vector_u8(buf)],
                        ))
                    }
                    Structure::BLS12_381_G2 => {
                        obj.downcast_ref::<ark_bls12_381::G2Projective>().unwrap().into_affine().serialize(&mut buf).unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_to_affine + gas_params.bls12_381.g2_affine_serialize_compressed,
                            smallvec![Value::vector_u8(buf)],
                        ))
                    }
                    Structure::BLS12_381_Gt => {
                        obj.downcast_ref::<ark_bls12_381::Fq12>().unwrap().serialize(&mut buf).unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fq12_serialize,
                            smallvec![Value::vector_u8(buf)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ElementSerializeUncompressed => {
            assert_eq!(1, type_tags.len());
            let handle = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                let obj = context
                    .extensions()
                    .get::<Bls12381Context>()
                    .borrow(&structure, handle).unwrap();
                let mut buf = vec![];
                match structure {
                    Structure::BLS12_381_G1 => {
                        obj.downcast_ref::<ark_bls12_381::G1Projective>().unwrap().into_affine().serialize_uncompressed(&mut buf).unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_to_affine + gas_params.bls12_381.g1_affine_serialize_uncompressed,
                            smallvec![Value::vector_u8(buf)],
                        ))
                    }
                    Structure::BLS12_381_G2 => {
                        obj.downcast_ref::<ark_bls12_381::G2Projective>().unwrap().into_affine().serialize_uncompressed(&mut buf).unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_to_affine + gas_params.bls12_381.g2_affine_serialize_uncompressed,
                            smallvec![Value::vector_u8(buf)],
                        ))
                    }
                    Structure::BLS12_381_Gt => {
                        obj.downcast_ref::<ark_bls12_381::Fq12>().unwrap().serialize_uncompressed(&mut buf).unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fq12_serialize,
                            smallvec![Value::vector_u8(buf)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::GroupOrder => {
            assert_eq!(1, type_tags.len());
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 | Structure::BLS12_381_G2 | Structure::BLS12_381_Gt => {
                        Ok(NativeResult::ok(
                            InternalGas::zero(),
                            smallvec![Value::vector_u8(BLS12381_R_BYTES_LENDIAN.clone())],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::GroupIdentity => {
            assert_eq!(1, type_tags.len());
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 => {
                        let point = G1Projective::zero();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_infinity,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_G2 => {
                        let point = G2Projective::zero();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_infinity,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_Gt => {
                        let element = Fq12::one();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(element));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fq12_one,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::GroupGenerator => {
            assert_eq!(1, type_tags.len());
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_G1 => {
                        let point = G1Projective::prime_subgroup_generator();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g1_proj_generator,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_G2 => {
                        let point = G2Projective::prime_subgroup_generator();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(point));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.g2_proj_generator,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    Structure::BLS12_381_Gt => {
                        let element = BLS12381_GT_GENERATOR.clone();
                        let handle = context
                            .extensions_mut()
                            .get_mut::<Bls12381Context>()
                            .append(structure, Rc::new(element));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fq12_clone,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::HashToElement => {
            assert_eq!(2, type_tags.len());
            let bytes = pop_arg!(args, Vec<u8>);
            if let (Some(hash_alg),Some(target_structure)) = (hashalg(&type_tags[0]), structure_from_type_tag(&type_tags[1])) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(target_structure))
                    || !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_hash_alg(hash_alg)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match (hash_alg, target_structure) {
                    (HashAlg::SHA256, Structure::BLS12_381_G1) => {
                        let blst_g1_proj = hash_to_blst_g1(bytes.as_slice());
                        let blst_g1_affine = blst_g1_proj_to_affine(&blst_g1_proj);
                        let ark_g1_affine = blst_g1_affine_to_ark_g1_affine(&blst_g1_affine);
                        let ark_g1_proj = ark_g1_affine.into_projective();
                        let handle = context.extensions_mut().get_mut::<Bls12381Context>().append(target_structure, Rc::new(ark_g1_proj));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.blst_hash_to_g1_proj(bytes.len())
                                + (
                                gas_params.bls12_381.blst_g1_proj_to_affine
                                    + gas_params.bls12_381.blst_g1_affine_ser
                                    + gas_params.bls12_381.ark_g1_affine_deser_uncomp
                                    + gas_params.bls12_381.ark_g1_affine_to_proj
                            ) * NumArgs::one(),
                            smallvec![Value::u64(handle as u64)]
                        ))
                    }
                    (HashAlg::SHA256, Structure::BLS12_381_G2) => {
                        let blst_g2_proj = hash_to_blst_g2(bytes.as_slice());
                        let blst_g2_affine = blst_g2_proj_to_affine(&blst_g2_proj);
                        let ark_g2_affine = blst_g2_affine_to_ark_g2_affine(&blst_g2_affine);
                        let ark_g2_proj = ark_g2_affine.into_projective();
                        let handle = context.extensions_mut().get_mut::<Bls12381Context>().append(target_structure, Rc::new(ark_g2_proj));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.blst_hash_to_g2_proj(bytes.len())
                                + (gas_params.bls12_381.blst_g2_proj_to_affine
                                + gas_params.bls12_381.blst_g2_affine_ser
                                + gas_params.bls12_381.ark_g2_affine_deser_uncomp
                                + gas_params.bls12_381.ark_g2_affine_to_proj
                            ) * NumArgs::one(),
                            smallvec![Value::u64(handle as u64)]
                        ))
                    }
                    _ => {
                        return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::PairingProduct => {
            assert_eq!(3, type_tags.len());
            let g2_handles = pop_arg!(args, Vec<u64>);
            let g1_handles = pop_arg!(args, Vec<u64>);
            if g2_handles.len() != g1_handles.len() {
                return Ok(NativeResult::err(InternalGas::zero(), NUM_G1_ELEMENTS_SHOULD_MATCH_NUM_G2_ELEMENTS));
            }

            match (
                structure_from_type_tag(&type_tags[0]),
                structure_from_type_tag(&type_tags[1]),
                structure_from_type_tag(&type_tags[2]),
            ) {
                (Some(g1_structure), Some(g2_structure), Some(gt_structure)) => {
                    if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(g1_structure))
                        || !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(g2_structure))
                        || !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(gt_structure)) {
                        return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                    }
                    match (g1_structure, g2_structure, gt_structure) {
                        (Structure::BLS12_381_G1, Structure::BLS12_381_G2, Structure::BLS12_381_Gt) => {
                            let g1_prepared: Vec<ark_ec::models::bls12::g1::G1Prepared<Parameters>> = g1_handles
                                .iter()
                                .map(|&handle| {
                                    let obj = context
                                        .extensions()
                                        .get::<Bls12381Context>()
                                        .borrow(&g1_structure, handle as usize).unwrap();
                                    let element = obj.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
                                    ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(element.into_affine())
                                })
                                .collect();
                            let g2_prepared: Vec<ark_ec::models::bls12::g2::G2Prepared<Parameters>> = g2_handles
                                .iter()
                                .map(|&handle| {
                                    let obj = context
                                        .extensions()
                                        .get::<Bls12381Context>()
                                        .borrow(&g2_structure, handle as usize).unwrap();
                                    let element = obj.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
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
                                .get_mut::<Bls12381Context>()
                                .append(gt_structure, Rc::new(result));
                            Ok(NativeResult::ok(
                                (gas_params.bls12_381.g1_affine_to_prepared + gas_params.bls12_381.g2_affine_to_prepared + gas_params.bls12_381.pairing_product_per_pair) * NumArgs::new(g1_handles.len() as u64) + gas_params.bls12_381.pairing_product_base,
                                smallvec![Value::u64(result_handle as u64)],
                            ))
                        }
                        _ => {
                            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                        }
                    }
                }
                _ => {
                    Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))

                }
            }
        }
        API::ScalarDeserialize => {
            assert_eq!(1, type_tags.len());
            let bytes = pop_arg!(args, Vec<u8>);
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_Fr => {
                        match ark_bls12_381::Fr::deserialize_uncompressed(bytes.as_slice()) {
                            Ok(scalar) => {
                                let handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(scalar));
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
                            }
                        }
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ScalarAdd => {
            assert_eq!(1, type_tags.len());
            let handle_1 = pop_arg!(args, u64) as usize;
            let handle_0 = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_Fr => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let s_0 = obj_0.downcast_ref::<ark_bls12_381::Fr>().unwrap().clone();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let s_1 = obj_1.downcast_ref::<ark_bls12_381::Fr>().unwrap().clone();
                        let s_new = s_0 + s_1;
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(s_new));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fr_eq,
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ScalarEq => {
            assert_eq!(1, type_tags.len());
            let handle_1 = pop_arg!(args, u64) as usize;
            let handle_0 = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_Fr => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let s_0 = obj_0.downcast_ref::<ark_bls12_381::Fr>().unwrap();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let s_1 = obj_1.downcast_ref::<ark_bls12_381::Fr>().unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fr_eq,
                            smallvec![Value::bool(s_0.eq(s_1))],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ScalarFromU64 => {
            assert_eq!(1, type_tags.len());
            let val = pop_arg!(args, u64);
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_Fr => {
                        let obj = Rc::new(ark_bls12_381::Fr::from(val as u128));
                        let handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, obj);
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fr_from_u64,
                            smallvec![Value::u64(handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ScalarInv => {
            assert_eq!(1, type_tags.len());
            let handle = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_Fr => {
                        let obj = context.extensions().get::<Bls12381Context>().borrow(&structure, handle).unwrap();
                        let s = obj.downcast_ref::<ark_bls12_381::Fr>().unwrap().clone();
                        match s.inverse() {
                            None => {
                                Ok(NativeResult::ok(
                                    InternalGas::zero(),
                                    smallvec![Value::bool(false), Value::u64(0)],
                                ))
                            }
                            Some(scalar) => {
                                let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(scalar));
                                Ok(NativeResult::ok(
                                    gas_params.bls12_381.fr_inv,
                                    smallvec![Value::bool(true), Value::u64(new_handle as u64)],
                                ))
                            }
                        }
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ScalarMul => {
            assert_eq!(1, type_tags.len());
            let handle_1 = pop_arg!(args, u64) as usize;
            let handle_0 = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_Fr => {
                        let obj_0 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_0).unwrap();
                        let s_0 = obj_0.downcast_ref::<ark_bls12_381::Fr>().unwrap().clone();
                        let obj_1 = context.extensions().get::<Bls12381Context>().borrow(&structure, handle_1).unwrap();
                        let s_1 = obj_1.downcast_ref::<ark_bls12_381::Fr>().unwrap().clone();
                        let s_new = s_0 * s_1;
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(s_new));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fr_eq,
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ScalarNeg => {
            assert_eq!(1, type_tags.len());
            let handle = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_Fr => {
                        let obj = context.extensions().get::<Bls12381Context>().borrow(&structure, handle).unwrap();
                        let scalar = obj.downcast_ref::<ark_bls12_381::Fr>().unwrap();
                        let new_scalar = scalar.neg();
                        let new_handle = context.extensions_mut().get_mut::<Bls12381Context>().append(structure, Rc::new(new_scalar));
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fr_inv,
                            smallvec![Value::u64(new_handle as u64)],
                        ))
                    }
                    _ => {
                        Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                    }
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
        }
        API::ScalarSerialize => {
            assert_eq!(1, type_tags.len());
            let handle = pop_arg!(args, u64) as usize;
            if let Some(structure) = structure_from_type_tag(&type_tags[0]) {
                if !context.extensions().get::<Bls12381Context>().is_feature_enabled(feature_for_structure(structure)) {
                    return Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED));
                }
                match structure {
                    Structure::BLS12_381_Fr => {
                        let mut buf = vec![];
                        let obj = context.extensions().get::<Bls12381Context>().borrow(&structure, handle).unwrap();
                        let s = obj.downcast_ref::<ark_bls12_381::Fr>().unwrap();
                        s.serialize(&mut buf).unwrap();
                        Ok(NativeResult::ok(
                            gas_params.bls12_381.fr_serialize,
                            smallvec![Value::vector_u8(buf)],
                        ))
                    }
                    _ => Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
                }
            } else {
                Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
            }
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
            make_native_from_func(gas_params.clone(), |a,b,c,d| uber(API::ElementSerializeUncompressed,a,b,c,d)),
        ),
        (
            "element_serialize_compressed_internal",
            make_native_from_func(gas_params.clone(), |x,y,z,w| uber(API::ElementSerializeCompressed, x, y, z, w)),
        ),
        (
            "element_deserialize_uncompressed_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d| uber(API::ElementDeserializeUncompressed,a,b,c,d),
            ),
        ),
        (
            "element_deserialize_compressed_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ElementDeserializeCompressed,a,b,c,d)),
        ),
        (
            "scalar_deserialize_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ScalarDeserialize,a,b,c,d)),
        ),
        (
            "scalar_serialize_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ScalarSerialize,a,b,c,d)),
        ),
        (
            "scalar_eq_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ScalarEq,a,b,c,d)),
        ),
        (
            "scalar_neg_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ScalarNeg,a,b,c,d)),
        ),
        (
            "scalar_inv_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ScalarInv,a,b,c,d)),
        ),
        (
            "scalar_from_u64_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ScalarFromU64,a,b,c,d)),
        ),
        (
            "scalar_add_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ScalarAdd,a,b,c,d)),
        ),
        (
            "scalar_mul_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ScalarMul,a,b,c,d)),
        ),
        (
            "group_identity_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::GroupIdentity,a,b,c,d)),
        ),
        (
            "group_generator_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::GroupGenerator,a,b,c,d)),
        ),
        (
            "element_add_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ElementAdd,a,b,c,d)),
        ),
        (
            "element_mul_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ElementScalarMul, a, b, c, d)),
        ),
        (
            "element_multi_scalar_mul_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ElementMultiScalarMul,a,b,c,d)),
        ),
        (
            "element_double_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ElementDouble,a,b,c,d)),
        ),
        (
            "element_neg_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ElementNeg,a,b,c,d)),
        ),
        (
            "element_eq_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::ElementEq,a,b,c,d)),
        ),
        (
            "pairing_product_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::PairingProduct,a,b,c,d)),
        ),
        (
            "group_order_internal",
            make_native_from_func(gas_params.clone(), |a,b,c,d|uber(API::GroupOrder,a,b,c,d)),
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

fn hashalg(type_tag: &TypeTag) -> Option<HashAlg> {
    match type_tag.to_string().as_str() {
        "0x1::groups::SHA256" => Some(HashAlg::SHA256),
        _ => None
    }
}
