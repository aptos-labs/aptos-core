// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0


use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::rc::Rc;
use ark_bls12_381::{Fq12, Fr, FrParameters, G1Projective, G2Projective, Parameters};
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::{BigInteger256, Field, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
#[cfg(feature = "testing")]
use ark_std::{test_rng, UniformRand};
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, NumArgs, NumBytes};
use move_core_types::language_storage::TypeTag;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use num_traits::{One, Zero};
use once_cell::sync::Lazy;
use serde::de::Unexpected::Str;
use smallvec::smallvec;
use aptos_types::on_chain_config::{FeatureFlag, Features};
use crate::natives::cryptography::algebra::abort_codes::NOT_IMPLEMENTED;
use crate::natives::cryptography::algebra::gas::GasParameters;
use crate::natives::util::make_native_from_func;
#[cfg(feature = "testing")]
use crate::natives::util::make_test_only_native_from_func;

pub mod gas;

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12_381_Fq12,
    BLS12_381_G1_SUB,
    BLS12_381_G2_SUB,
    BLS12_381_Gt,
    BLS12_381_Fr,
}

impl Structure {
    pub fn from_type_tag(type_tag: &TypeTag) -> Option<Structure> {
        match type_tag.to_string().as_str() {
            "0x1::algebra::BLS12_381_Fr" => Some(Structure::BLS12_381_Fr),
            "0x1::algebra::BLS12_381_Fq12" => Some(Structure::BLS12_381_Fq12),
            "0x1::algebra::BLS12_381_G1" => Some(Structure::BLS12_381_G1_SUB),
            "0x1::algebra::BLS12_381_G2" => Some(Structure::BLS12_381_G2_SUB),
            "0x1::algebra::BLS12_381_Gt" => Some(Structure::BLS12_381_Gt),
            _ => None
        }
    }
}

pub mod abort_codes {
    pub const NOT_IMPLEMENTED: u64 = 0x0c0000;
    pub const NUM_ELEMENTS_SHOULD_MATCH_NUM_SCALARS: u64 = 4;
    pub const NUM_G1_ELEMENTS_SHOULD_MATCH_NUM_G2_ELEMENTS: u64 = 5;
}

#[derive(Tid)]
pub struct AlgebraContext {
    features: Features,
    bls12_381_fr_elements: Vec<Fr>,
    bls12_381_g1_elements: Vec<G1Projective>,
    bls12_381_g2_elements: Vec<G2Projective>,
    bls12_381_fq12_elements: Vec<Fq12>,
    objs: HashMap<Structure, Vec<Rc<dyn Any>>>,
}

impl AlgebraContext {
    pub fn new(features: Features) -> Self {
        Self {
            bls12_381_fr_elements: vec![],
            bls12_381_g1_elements: vec![],
            bls12_381_g2_elements: vec![],
            bls12_381_fq12_elements: vec![],
            objs: HashMap::new(),
            features,
        }
    }
}

macro_rules! abort_if_feature_disabled {
    ($context:expr, $feature:expr) => {
        if !$context.extensions().get::<AlgebraContext>().features.is_enabled($feature) {
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
        Structure::from_type_tag(&type_tag)
    }}
}

macro_rules! borrow_bls12_381_g1 {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<AlgebraContext>().bls12_381_g1_elements.get($handle).unwrap()
    }}
}

macro_rules! store_bls12_381_g1 {
    ($context:expr, $element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<AlgebraContext>();
        let ret = inner_ctxt.bls12_381_g1_elements.len();
        inner_ctxt.bls12_381_g1_elements.push($element);
        ret
    }}
}

macro_rules! borrow_bls12_381_fr {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<AlgebraContext>().bls12_381_fr_elements.get($handle).unwrap()
    }}
}

macro_rules! store_bls12_381_fr {
    ($context:expr, $element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<AlgebraContext>();
        let ret = inner_ctxt.bls12_381_fr_elements.len();
        inner_ctxt.bls12_381_fr_elements.push($element);
        ret
    }}
}

macro_rules! borrow_bls12_381_g2 {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<AlgebraContext>().bls12_381_g2_elements.get($handle).unwrap()
    }}
}

macro_rules! store_bls12_381_g2 {
    ($context:expr, $element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<AlgebraContext>();
        let ret = inner_ctxt.bls12_381_g2_elements.len();
        inner_ctxt.bls12_381_g2_elements.push($element);
        ret
    }}
}

macro_rules! borrow_bls12_381_fq12 {
    ($context:expr, $handle:expr) => {{
        $context.extensions().get::<AlgebraContext>().bls12_381_fq12_elements.get($handle).unwrap()
    }}
}


macro_rules! store_bls12_381_fq12 {
    ($context:expr, $element:expr) => {{
        let inner_ctxt = $context.extensions_mut().get_mut::<AlgebraContext>();
        let ret = inner_ctxt.bls12_381_fq12_elements.len();
        inner_ctxt.bls12_381_fq12_elements.push($element);
        ret
    }}
}

macro_rules! borrow_bls12_381_stuff {
    ($context:expr, $struct_name:expr, $handle:expr, $assign_to:ident) => {
        let pointer: Rc<dyn Any> = $context.extensions().get::<AlgebraContext>().objs.get(&$struct_name).unwrap().get($handle).unwrap();
        let mut $assign_to = pointer.downcast_ref<ark_bls12_381::Fr>().unwrap();

    }
}

macro_rules! store_obj {
    ($context:expr, $structure:expr, $obj:expr) => {{
        let target_vec = $context.extensions_mut().get_mut::<AlgebraContext>().objs.entry($structure).or_insert_with(Vec::new);
        let ret = target_vec.len();
        target_vec.push(Rc::new($obj));
        ret
    }}
}

macro_rules! get_obj_pointer {
    ($context:expr, $structure:expr, $handle:expr) => {{
        let target_vec = $context.extensions_mut().get_mut::<AlgebraContext>().objs.entry($structure).or_insert_with(Vec::new);
        target_vec[$handle].clone()
    }}
}

static BLS12_381_FQ_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("01").unwrap());
static BLS12_381_FQ_BENDIAN_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0101").unwrap());
static BLS12_381_FQ2_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("02").unwrap());
static BLS12_381_FQ6_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("03").unwrap());
static BLS12_381_FQ12_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("04").unwrap());
static BLS12_381_G1_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("05").unwrap());
static BLS12_381_G1_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0501").unwrap());
static BLS12_381_G1_SUB_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("06").unwrap());
static BLS12_381_G1_SUB_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0601").unwrap());
static BLS12_381_G2_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("07").unwrap());
static BLS12_381_G2_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0701").unwrap());
static BLS12_381_G2_SUB_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("08").unwrap());
static BLS12_381_G2_SUB_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0801").unwrap());
static BLS12_381_GT_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("09").unwrap());
static BLS12_381_FR_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0a").unwrap());
static BLS12_381_FR_BENDIAN_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0a01").unwrap());


fn serialize_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    let scheme = pop_arg!(args, Vec<u8>);
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    match (structure_opt, scheme) {
        (Some(structure), scheme) if structure == Structure::BLS12_381_Fr && scheme.as_slice() == BLS12_381_FR_FORMAT.as_slice() => {
            let element_ptr = get_obj_pointer!(context, structure, handle);
            let element = element_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let buf = ark_serialize_uncompressed!(element);
            Ok(NativeResult::ok(
                gas_params.serialize(structure, scheme.clone()),
                smallvec![Value::vector_u8(buf)],
            ))
        }
        (Some(structure), scheme) if structure == Structure::BLS12_381_Fr && scheme.as_slice() == BLS12_381_FR_BENDIAN_FORMAT.as_slice() => {
            let element_ptr = get_obj_pointer!(context, structure, handle);
            let element = element_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let mut buf = ark_serialize_uncompressed!(element);
            buf.reverse();
            Ok(NativeResult::ok(
                gas_params.serialize(structure, scheme.clone()),
                smallvec![Value::vector_u8(buf)],
            ))
        }
        // Some(Structure::BLS12_381_G2) => {
        //     abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
        //     let element = borrow_bls12_381_g2!(context, handle);
        //     let buf = ark_serialize_uncompressed!(element);
        //     Ok(NativeResult::ok(
        //         (gas_params.ark_bls12_381_g2_proj_to_affine + gas_params.ark_bls12_381_g2_affine_ser_uncomp) * NumArgs::one(),
        //         smallvec![Value::vector_u8(buf)],
        //     ))
        // }
        // Some(Structure::BLS12_381_Gt) => {
        //     abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
        //     let element = borrow_bls12_381_fq12!(context, handle);
        //     let buf = ark_serialize_uncompressed!(element);
        //     Ok(NativeResult::ok(
        //         gas_params.ark_bls12_381_fq12_serialize * NumArgs::one(),
        //         smallvec![Value::vector_u8(buf)],
        //     ))
        // }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn deserialize_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let structure = structure_from_ty_arg!(context, &ty_args[0]);
    let mut bytes = pop_arg!(args, Vec<u8>);
    let scheme = pop_arg!(args, Vec<u8>);
    match (structure,scheme) {
        (Some(structure), scheme) if structure == Structure::BLS12_381_Fr && scheme.as_slice() == BLS12_381_FR_FORMAT.as_slice() => {
            match ark_bls12_381::Fr::deserialize_uncompressed(bytes.as_slice()) {
                Ok(element) => {
                    let handle = store_obj!(context, structure, element);
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_fr_deser_base * NumArgs::one() + gas_params.ark_bls12_381_fr_deser_per_byte * NumArgs::from(bytes.len() as u64),
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                },
                _ => {
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_fr_deser_base * NumArgs::one() + gas_params.ark_bls12_381_fr_deser_per_byte * NumArgs::from(bytes.len() as u64),
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        (Some(structure), scheme) if structure == Structure::BLS12_381_Fr && scheme.as_slice() == BLS12_381_FR_BENDIAN_FORMAT.as_slice() => {
            bytes.reverse();
            match ark_bls12_381::Fr::deserialize_uncompressed(bytes.as_slice()) {
                Ok(element) => {
                    let handle = store_obj!(context, structure, element);
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_fr_deser_base * NumArgs::one() + gas_params.ark_bls12_381_fr_deser_per_byte * NumArgs::from(bytes.len() as u64),
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                },
                _ => {
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_fr_deser_base * NumArgs::one() + gas_params.ark_bls12_381_fr_deser_per_byte * NumArgs::from(bytes.len() as u64),
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        (Some(structure), scheme) if structure== Structure::BLS12_381_G1_SUB && scheme.as_slice() == BLS12_381_G1_SUB_UNCOMPRESSED_FORMAT.as_slice() => {
            match ark_bls12_381::G1Affine::deserialize_uncompressed(bytes.as_slice()) {
                Ok(element) => {
                    let handle = store_obj!(context, structure, element.into_projective());
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_g1_affine_deser_uncomp_base * NumArgs::one() + gas_params.ark_bls12_381_g1_affine_deser_uncomp_per_byte * NumArgs::from(bytes.len() as u64) + gas_params.ark_bls12_381_g1_affine_to_proj * NumArgs::one(),
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                }
                _ => {
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_g1_affine_deser_uncomp_base * NumArgs::one() + gas_params.ark_bls12_381_g1_affine_deser_uncomp_per_byte * NumArgs::from(bytes.len() as u64),
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        (Some(structure), scheme) if structure== Structure::BLS12_381_G1_SUB && scheme.as_slice() == BLS12_381_G1_SUB_COMPRESSED_FORMAT.as_slice() => {
            match ark_bls12_381::G1Affine::deserialize(bytes.as_slice()) {
                Ok(element) => {
                    let handle = store_obj!(context, structure, element.into_projective());
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_g1_affine_deser_comp_base * NumArgs::one() + gas_params.ark_bls12_381_g1_affine_deser_comp_per_byte * NumArgs::from(bytes.len() as u64) + gas_params.ark_bls12_381_g1_affine_to_proj * NumArgs::one(),
                        smallvec![Value::bool(true), Value::u64(handle as u64)],
                    ))
                }
                _ => {
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_g1_affine_deser_comp_base * NumArgs::one() + gas_params.ark_bls12_381_g1_affine_deser_comp_per_byte * NumArgs::from(bytes.len() as u64),
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        // Some(Structure::BLS12_381_G2) => {
        //     abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
        //     match ark_bls12_381::G2Affine::deserialize_uncompressed(bytes.as_slice()) {
        //         Ok(element) => {
        //             let handle = store_bls12_381_g2!(context, element.into_projective());
        //             Ok(NativeResult::ok(
        //                 (gas_params.ark_bls12_381_g2_affine_deser_uncomp + gas_params.ark_bls12_381_g2_affine_to_proj) * NumArgs::one(),
        //                 smallvec![Value::bool(true), Value::u64(handle as u64)],
        //             ))
        //         }
        //         _ => {
        //             Ok(NativeResult::ok(
        //                 gas_params.ark_bls12_381_g2_affine_deser_uncomp * NumArgs::one(),
        //                 smallvec![Value::bool(false), Value::u64(0)],
        //             ))
        //         }
        //     }
        // }
        // Some(Structure::BLS12_381_Gt) => {
        //     abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
        //     match Fq12::deserialize(bytes.as_slice()) {
        //         Ok(element) => {
        //             let cost = (gas_params.ark_bls12_381_fq12_deserialize + gas_params.ark_bls12_381_fq12_pow_u256 + gas_params.ark_bls12_381_fq12_eq) * NumArgs::one();
        //             if Fq12::one() == element.pow(BLS12381_R_SCALAR.clone()) {
        //                 let handle = store_bls12_381_fq12!(context, element);
        //                 Ok(NativeResult::ok(
        //                     cost,
        //                     smallvec![Value::bool(true), Value::u64(handle as u64)],
        //                 ))
        //             } else {
        //                 Ok(NativeResult::ok(
        //                     cost,
        //                     smallvec![Value::bool(false), Value::u64(0)],
        //                 ))
        //             }
        //         }
        //         _ => {
        //             Ok(NativeResult::ok(
        //                 gas_params.ark_bls12_381_fq12_deserialize * NumArgs::one(),
        //                 smallvec![Value::bool(false), Value::u64(0)],
        //             ))
        //         }
        //     }
        // }
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_fr!(context, handle);
            let buf = ark_serialize_uncompressed!(element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fr_ser * NumArgs::one(),
                smallvec![Value::vector_u8(buf)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn from_u64_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let value = pop_arg!(args, u64);
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(structure) if structure == Structure::BLS12_381_Fr => {
            let element = ark_bls12_381::Fr::from(value as u128);
            let handle = store_obj!(context, structure, element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fr_from_u128 * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn field_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(structure) if structure == Structure::BLS12_381_Fr => {
            let handle_2 = pop_arg!(args, u64) as usize;
            let handle_1 = pop_arg!(args, u64) as usize;
            let element_1_ptr = get_obj_pointer!(context, structure, handle_1);
            let element_1 = element_1_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let element_2_ptr = get_obj_pointer!(context, structure, handle_2);
            let element_2 = element_2_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let new_element = element_1.add(element_2);
            let new_handle = store_obj!(context, structure, new_element);
            Ok(NativeResult::ok(
                gas_params.field_add(structure),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn field_sub_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(structure) if structure == Structure::BLS12_381_Fr => {
            let element_1_ptr = get_obj_pointer!(context, structure, handle_1);
            let element_1 = element_1_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let element_2_ptr = get_obj_pointer!(context, structure, handle_2);
            let element_2 = element_2_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let new_element = element_1.sub(element_2);
            let new_handle = store_obj!(context, structure, new_element);
            Ok(NativeResult::ok(
                gas_params.field_sub(structure),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Fq12) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let scalar_1 = borrow_bls12_381_fq12!(context, handle_1);
            let scalar_2 = borrow_bls12_381_fq12!(context, handle_2);
            let result = scalar_1.add(scalar_2);
            let result_handle = store_bls12_381_fq12!(context, result);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fq12_sub * NumArgs::one(),
                smallvec![Value::u64(result_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn field_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(structure) if structure == Structure::BLS12_381_Fr => {
            let element_1_ptr = get_obj_pointer!(context, structure, handle_1);
            let element_1 = element_1_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let element_2_ptr = get_obj_pointer!(context, structure, handle_2);
            let element_2 = element_2_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let new_element = element_1.mul(element_2);
            let new_handle = store_obj!(context, structure, new_element);
            Ok(NativeResult::ok(
                gas_params.field_mul(structure),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn vu8_to_vu64(v: &[u8]) -> Vec<u64> {
    let mut ret = Vec::new();
    for (i,&e) in v.iter().enumerate() {
        if i%8==0 {
            ret.push(e as u64);
        } else {
            ret[i/8] += (e as u64) << (8 * (i%8))
        }
    }
    ret
}

#[test]
fn test_vu8_to_vu64() {
    assert_eq!(vec![0x03020100_u64, 0x0504], vu8_to_vu64(vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05].as_slice()));
}

macro_rules! field_pow_internal {
    ($gas_params:ident, $context:ident, $args:ident, $structure:ident, $typ:ty) => {{
        let exponent = vu8_to_vu64(pop_arg!($args, Vec<u8>).as_slice());
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, $structure, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let new_scalar = element.pow(exponent.as_slice());
        let new_handle = store_obj!($context, $structure, new_scalar);
        Ok(NativeResult::ok(
            $gas_params.field_pow($structure, exponent.len()),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn field_pow_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(structure) if structure == Structure::BLS12_381_Fr => field_pow_internal!(gas_params, context, args, structure, ark_bls12_381::Fr),
        Some(structure) if structure == Structure::BLS12_381_Fq12 => field_pow_internal!(gas_params, context, args, structure, ark_bls12_381::Fq12),
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

macro_rules! ark_field_div_internal {
    ($gas_params:ident, $context:ident, $args:ident, $structure:ident, $typ:ty) => {{
            let handle_2 = pop_arg!($args, u64) as usize;
            let handle_1 = pop_arg!($args, u64) as usize;
            let element_1_ptr = get_obj_pointer!($context, $structure, handle_1);
            let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
            let element_2_ptr = get_obj_pointer!($context, $structure, handle_2);
            let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
            if element_2.is_zero() {
                return Ok(NativeResult::ok(
                    InternalGas::zero(),
                    smallvec![Value::bool(false), Value::u64(0_u64)],
                ));
            }
            let new_element = element_1.div(element_2);
            let new_handle = store_obj!($context, $structure, new_element);
            Ok(NativeResult::ok(
                $gas_params.field_div($structure),
                smallvec![Value::bool(true), Value::u64(new_handle as u64)],
            ))

    }}
}
fn field_div_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(s) if s == Structure::BLS12_381_Fr => ark_field_div_internal!(gas_params, context, args, s, ark_bls12_381::Fr),
        Some(s) if s == Structure::BLS12_381_Fq12 => ark_field_div_internal!(gas_params, context, args, s, ark_bls12_381::Fq12),
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn field_neg_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(structure) if structure == Structure::BLS12_381_Fr => {
            let handle = pop_arg!(args, u64) as usize;
            let element_ptr = get_obj_pointer!(context, structure, handle);
            let element = element_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let new_element = element.neg();
            let new_handle = store_obj!(context, structure, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fr_neg * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn field_inv_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(structure) if structure == Structure::BLS12_381_Fr => {
            let handle = pop_arg!(args, u64) as usize;
            let element_ptr = get_obj_pointer!(context, structure, handle);
            let element = element_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            match element.inverse() {
                Some(new_element) => {
                    let new_handle = store_obj!(context, structure, new_element);
                    Ok(NativeResult::ok(
                        gas_params.field_inv(structure),
                        smallvec![Value::bool(true), Value::u64(new_handle as u64)],
                    ))
                }
                None => {
                    Ok(NativeResult::ok(
                        gas_params.ark_bls12_381_fr_inv * NumArgs::one(),
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

fn eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(structure) if structure == Structure::BLS12_381_Fr => {
            let element_1_ptr = get_obj_pointer!(context, structure, handle_1);
            let element_1 = element_1_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            let element_2_ptr = get_obj_pointer!(context, structure, handle_2);
            let element_2 = element_2_ptr.downcast_ref::<ark_bls12_381::Fr>().unwrap();
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fr_eq * NumArgs::one(),
                smallvec![Value::bool(element_1 == element_2)],
            ))
        }
        _ => {
            Ok(NativeResult::err(
                gas_params.ark_bls12_381_fr_eq * NumArgs::one(),
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = G1Projective::zero();
            let handle = store_bls12_381_g1!(context, element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g1_proj_infinity * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = G2Projective::zero();
            let handle = store_bls12_381_g2!(context, element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g2_proj_infinity * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = Fq12::one();
            let handle = store_bls12_381_fq12!(context, element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fq12_one * NumArgs::one(),
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = ark_bls12_381::G1Projective::prime_subgroup_generator();
            let handle = store_bls12_381_g1!(context, element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g1_proj_generator * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = ark_bls12_381::G2Projective::prime_subgroup_generator();
            let handle = store_bls12_381_g2!(context, element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g2_proj_generator * NumArgs::one(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = BLS12381_GT_GENERATOR.clone();
            let handle = store_bls12_381_fq12!(context, element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fq12_clone * NumArgs::one(),
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) | Some(Structure::BLS12_381_G2_SUB) | Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) | Some(Structure::BLS12_381_G2_SUB) | Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = G1Projective::rand(&mut test_rng());
            let handle = store_bls12_381_g1!(context, element);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = G2Projective::rand(&mut test_rng());
            let handle = store_bls12_381_g2!(context, element);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let k = Fr::rand(&mut test_rng());
            let element = BLS12381_GT_GENERATOR.clone().pow(k.into_repr());
            let handle = store_bls12_381_fq12!(context, element);
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element_1 = borrow_bls12_381_g1!(context, handle_1);
            let element_2 = borrow_bls12_381_g1!(context, handle_2);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g1_proj_eq * NumArgs::one(),
                smallvec![Value::bool(element_1.eq(element_2))],
            ))
        }
        Some(Structure::BLS12_381_G2_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element_1 = borrow_bls12_381_g2!(context, handle_1);
            let element_2 = borrow_bls12_381_g2!(context, handle_2);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g2_proj_eq * NumArgs::one(),
                smallvec![Value::bool(element_1.eq(element_2))],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element_1 = borrow_bls12_381_fq12!(context, handle_1);
            let element_2 = borrow_bls12_381_fq12!(context, handle_2);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fq12_eq * NumArgs::one(),
                smallvec![Value::bool(element_1.eq(element_2))],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn group_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle_2 = pop_arg!(args, u64) as usize;
    let handle_1 = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element_1 = borrow_bls12_381_g1!(context, handle_1);
            let element_2 = borrow_bls12_381_g1!(context, handle_2);
            let new_element = element_1.add(element_2);
            let new_handle = store_bls12_381_g1!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g1_proj_add * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element_1 = borrow_bls12_381_g2!(context, handle_1);
            let element_2 = borrow_bls12_381_g2!(context, handle_2);
            let new_element = element_1.add(element_2);
            let new_handle = store_bls12_381_g2!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g2_proj_add * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element_1 = borrow_bls12_381_fq12!(context, handle_1);
            let element_2 = borrow_bls12_381_fq12!(context, handle_2);
            let new_element = element_1.mul(element_2);
            let new_handle = store_bls12_381_fq12!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fq12_mul * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn group_scalar_mul_internal(
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
        (Some(Structure::BLS12_381_G1_SUB), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_g1!(context, element_handle);
            let scalar = borrow_bls12_381_fr!(context, scalar_handle);
            let new_element = element.mul(scalar.into_repr());
            let new_handle = store_bls12_381_g1!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g1_proj_scalar_mul * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        (Some(Structure::BLS12_381_G2_SUB), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_g2!(context, element_handle);
            let scalar = borrow_bls12_381_fr!(context, scalar_handle);
            let new_element = element.mul(scalar.into_repr());
            let new_handle = store_bls12_381_g2!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g2_proj_scalar_mul * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        (Some(Structure::BLS12_381_Gt), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_fq12!(context, element_handle);
            let scalar = borrow_bls12_381_fr!(context, scalar_handle);
            let new_element = element.pow(scalar.into_repr().as_ref());
            let new_handle = store_bls12_381_fq12!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fr_to_repr * NumArgs::one() + gas_params.field_pow(Structure::BLS12_381_Fq12, 32),
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



fn group_multi_scalar_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
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
        (Some(Structure::BLS12_381_G1_SUB), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
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
                gas_params.blst_g1_msm_per_pair * NumArgs::from(num_elements as u64)
                    + gas_params.blst_g1_msm_base * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        (Some(Structure::BLS12_381_G2_SUB), Some(Structure::BLS12_381_Fr)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
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
                gas_params.blst_g2_msm_per_pair * NumArgs::from(num_elements as u64)
                    + gas_params.blst_g2_msm_base * NumArgs::one(),
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

fn group_double_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_g1!(context, handle);
            let new_element = element.double();
            let new_handle = store_bls12_381_g1!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g1_proj_double * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_g2!(context, handle);
            let new_element = element.double();
            let new_handle = store_bls12_381_g2!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g2_proj_double * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_fq12!(context, handle);
            let new_element = element.square();
            let new_handle = store_bls12_381_fq12!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fq12_square * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

fn group_neg_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    let handle = pop_arg!(args, u64) as usize;
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_g1!(context, handle);
            let new_element = element.neg();
            let new_handle = store_bls12_381_g1!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g1_proj_neg * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2_SUB) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_g2!(context, handle);
            let new_element = element.neg();
            let new_handle = store_bls12_381_g2!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_g2_proj_neg * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_Gt) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = borrow_bls12_381_fq12!(context, handle);
            let new_element = element.inverse().unwrap();
            let new_handle = store_bls12_381_fq12!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.ark_bls12_381_fq12_inv * NumArgs::one(),
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
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(3, ty_args.len());
    let g1 = structure_from_ty_arg!(context, &ty_args[0]);
    let g2 = structure_from_ty_arg!(context, &ty_args[1]);
    let gt = structure_from_ty_arg!(context, &ty_args[2]);
    let g2_handles = pop_arg!(args, Vec<u64>);
    let g1_handles = pop_arg!(args, Vec<u64>);
    match (g1, g2, gt) {
        (Some(Structure::BLS12_381_G1_SUB), Some(Structure::BLS12_381_G2_SUB), Some(Structure::BLS12_381_Gt)) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
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
            let new_handle = store_bls12_381_fq12!(context, new_element);
            Ok(NativeResult::ok(
                (gas_params.ark_bls12_381_g1_proj_to_affine + gas_params.ark_bls12_381_g1_affine_to_prepared + gas_params.ark_bls12_381_g2_proj_to_affine + gas_params.ark_bls12_381_g2_affine_to_prepared + gas_params.ark_bls12_381_pairing_product_per_pair) * NumArgs::new(g1_handles.len() as u64) + gas_params.ark_bls12_381_pairing_product_base * NumArgs::one(),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), abort_codes::NOT_IMPLEMENTED))
        }
    }
}

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

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "serialize_internal",
            make_native_from_func(gas_params.clone(), serialize_internal),
        ),
        (
            "deserialize_internal",
            make_native_from_func(gas_params.clone(), deserialize_internal),
        ),
        (
            "eq_internal",
            make_native_from_func(gas_params.clone(), eq_internal),
        ),
        (
            "field_neg_internal",
            make_native_from_func(gas_params.clone(), field_neg_internal),
        ),
        (
            "field_inv_internal",
            make_native_from_func(gas_params.clone(), field_inv_internal),
        ),
        (
            "from_u64_internal",
            make_native_from_func(gas_params.clone(), from_u64_internal),
        ),
        (
            "field_add_internal",
            make_native_from_func(gas_params.clone(), field_add_internal),
        ),
        (
            "field_sub_internal",
            make_native_from_func(gas_params.clone(), field_sub_internal),
        ),
        (
            "field_mul_internal",
            make_native_from_func(gas_params.clone(), field_mul_internal),
        ),
        (
            "field_div_internal",
            make_native_from_func(gas_params.clone(), field_div_internal),
        ),
        (
            "field_pow_internal",
            make_native_from_func(gas_params.clone(), field_pow_internal),
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
            "group_add_internal",
            make_native_from_func(gas_params.clone(), group_add_internal),
        ),
        (
            "group_mul_internal",
            make_native_from_func(gas_params.clone(), group_scalar_mul_internal),
        ),
        (
            "group_multi_scalar_mul_internal",
            make_native_from_func(gas_params.clone(), group_multi_scalar_mul_internal),
        ),
        (
            "group_double_internal",
            make_native_from_func(gas_params.clone(), group_double_internal),
        ),
        (
            "group_neg_internal",
            make_native_from_func(gas_params.clone(), group_neg_internal),
        ),
        (
            "pairing_product_internal",
            make_native_from_func(gas_params.clone(), pairing_product_internal),
        ),
        (
            "group_order_internal",
            make_native_from_func(gas_params.clone(), group_order_internal),
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
