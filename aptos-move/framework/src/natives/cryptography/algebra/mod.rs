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
    BLS12_381_G1,
    BLS12_381_G2,
    BLS12_381_Gt,
    BLS12_381_Fr,
}

impl Structure {
    pub fn from_type_tag(type_tag: &TypeTag) -> Option<Structure> {
        match type_tag.to_string().as_str() {
            "0x1::algebra::BLS12_381_Fr" => Some(Structure::BLS12_381_Fr),
            "0x1::algebra::BLS12_381_Fq12" => Some(Structure::BLS12_381_Fq12),
            "0x1::algebra::BLS12_381_G1" => Some(Structure::BLS12_381_G1),
            "0x1::algebra::BLS12_381_G2" => Some(Structure::BLS12_381_G2),
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
static BLS12_381_G1_PARENT_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("05").unwrap());
static BLS12_381_G1_PARENT_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0501").unwrap());
static BLS12_381_G1_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("06").unwrap());
static BLS12_381_G1_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0601").unwrap());
static BLS12_381_G2_PARENT_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("07").unwrap());
static BLS12_381_G2_PARENT_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0701").unwrap());
static BLS12_381_G2_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("08").unwrap());
static BLS12_381_G2_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0801").unwrap());
static BLS12_381_GT_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("09").unwrap());
static BLS12_381_FR_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0a").unwrap());
static BLS12_381_FR_BENDIAN_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0a01").unwrap());


macro_rules! ark_serialize_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $handle:expr, $scheme:expr, $typ:ty, $ser_func:ident) => {{
            let element_ptr = get_obj_pointer!($context, $structure, $handle);
            let element = element_ptr.downcast_ref::<$typ>().unwrap();
            let mut buf = Vec::new();
            element.$ser_func(&mut buf).unwrap();
            let cost = $gas_params.serialize($structure, $scheme);
            (cost, buf)
    }}
}

macro_rules! ark_ec_point_serialize_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $handle:expr, $scheme:expr, $typ:ty, $ser_func:ident) => {{
            let element_ptr = get_obj_pointer!($context, $structure, $handle);
            let element = element_ptr.downcast_ref::<$typ>().unwrap()
            let element_affine = element.into_affine();
            let mut buf = Vec::new();
            element_affine.$ser_func(&mut buf).unwrap();
            let cost = $gas_params.serialize($structure, $scheme);
            (cost, buf)
    }}
}

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
        (Some(Structure::BLS12_381_Fr), scheme) if scheme.as_slice() == BLS12_381_FR_FORMAT.as_slice() => {
            let (cost, mut buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12_381_Fr, handle, scheme.as_slice(), ark_bls12_381::Fr, serialize_uncompressed);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12_381_Fr), scheme) if scheme.as_slice() == BLS12_381_FR_BENDIAN_FORMAT.as_slice() => {
            let (cost, mut buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12_381_Fr, handle, scheme.as_slice(), ark_bls12_381::Fr, serialize_uncompressed);
            buf.reverse();
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12_381_Fq12), scheme) if scheme.as_slice() == BLS12_381_FQ12_FORMAT.as_slice() => {
            let (cost, mut buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12_381_Fq12, handle, scheme.as_slice(), ark_bls12_381::Fq12, serialize_uncompressed);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12_381_G1), scheme) if scheme.as_slice() == BLS12_381_G1_UNCOMPRESSED_FORMAT.as_slice() => {
            let (cost, mut buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12_381_G1, handle, scheme.as_slice(), ark_bls12_381::G1Projective, serialize_uncompressed);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12_381_G1), scheme) if scheme.as_slice() == BLS12_381_G1_COMPRESSED_FORMAT.as_slice() => {
            let (cost, mut buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12_381_G1, handle, scheme.as_slice(), ark_bls12_381::G1Projective, serialize);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
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

macro_rules! ark_deserialize_internal {
    ($gas_params:expr, $context:expr, $bytes:ident, $structure:expr, $scheme:expr, $expected_length:expr, $typ:ty, $deser_func:ident) => {{
        if $bytes.len() != $expected_length {
            return Ok(NativeResult::ok(
                $gas_params.deserialize($structure, $scheme),
                smallvec![Value::bool(false), Value::u64(0)],
            ));
        }
        match <$typ>::$deser_func($bytes.as_slice()) {
            Ok(element) => {
                let handle = store_obj!($context, $structure, element);
                Ok(NativeResult::ok(
                    $gas_params.deserialize($structure, $scheme),
                    smallvec![Value::bool(true), Value::u64(handle as u64)],
                ))
            },
            _ => {
                Ok(NativeResult::ok(
                    $gas_params.deserialize($structure, $scheme),
                    smallvec![Value::bool(false), Value::u64(0)],
                ))
            }
        }
    }}
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
            ark_deserialize_internal!(gas_params, context, bytes, structure, scheme.as_slice(), 32, ark_bls12_381::Fr, deserialize_uncompressed)
        }
        (Some(structure), scheme) if structure == Structure::BLS12_381_Fr && scheme.as_slice() == BLS12_381_FR_BENDIAN_FORMAT.as_slice() => {
            bytes.reverse();
            ark_deserialize_internal!(gas_params, context, bytes, structure, scheme.as_slice(), 32, ark_bls12_381::Fr, deserialize_uncompressed)
        }
        (Some(structure), scheme) if structure == Structure::BLS12_381_Fq12 && scheme.as_slice() == BLS12_381_FQ12_FORMAT.as_slice() => {
            ark_deserialize_internal!(gas_params, context, bytes, structure, scheme.as_slice(), 576, ark_bls12_381::Fq12, deserialize_uncompressed)
        }
        (Some(structure), scheme) if structure == Structure::BLS12_381_G1 && scheme.as_slice() == BLS12_381_G1_UNCOMPRESSED_FORMAT.as_slice() => {
            ark_deserialize_internal!(gas_params, context, bytes, structure, scheme.as_slice(), 96, ark_bls12_381::G1Affine, deserialize_uncompressed)
        }
        (Some(structure), scheme) if structure == Structure::BLS12_381_G1 && scheme.as_slice() == BLS12_381_G1_COMPRESSED_FORMAT.as_slice() => {
            ark_deserialize_internal!(gas_params, context, bytes, structure, scheme.as_slice(), 48, ark_bls12_381::G1Affine, deserialize)
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

macro_rules! from_u64_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
            let value = pop_arg!($args, u64);
            let element = <$typ>::from(value as u128);
            let handle = store_obj!($context, $structure, element);
            Ok(NativeResult::ok(
                $gas_params.from_u128($structure),
                smallvec![Value::u64(handle as u64)],
            ))
    }}
}

fn from_u64_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => from_u64_internal!(gas_params, context, args, Structure::BLS12_381_Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12_381_Fq12) => from_u64_internal!(gas_params, context, args, Structure::BLS12_381_Fq12, ark_bls12_381::Fq12),
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

macro_rules! ark_field_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = pop_arg!($args, u64) as usize;
        let handle_1 = pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, $structure, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, $structure, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element_1.add(element_2);
        let new_handle = store_obj!($context, $structure, new_element);
        Ok(NativeResult::ok(
            $gas_params.field_add($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn field_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => ark_field_add_internal!(gas_params, context, args, Structure::BLS12_381_Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12_381_Fq12) => ark_field_add_internal!(gas_params, context, args, Structure::BLS12_381_Fq12, ark_bls12_381::Fq12),
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

macro_rules! ark_field_sub_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = pop_arg!($args, u64) as usize;
        let handle_1 = pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, $structure, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, $structure, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element_1.sub(element_2);
        let new_handle = store_obj!($context, $structure, new_element);
        Ok(NativeResult::ok(
            $gas_params.field_sub($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn field_sub_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => ark_field_sub_internal!(gas_params, context, args, Structure::BLS12_381_Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12_381_Fq12) => ark_field_sub_internal!(gas_params, context, args, Structure::BLS12_381_Fq12, ark_bls12_381::Fq12),
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

macro_rules! ark_field_mul_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = pop_arg!($args, u64) as usize;
        let handle_1 = pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, $structure, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, $structure, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element_1.mul(element_2);
        let new_handle = store_obj!($context, $structure, new_element);
        Ok(NativeResult::ok(
            $gas_params.field_mul($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn field_mul_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => ark_field_mul_internal!(gas_params, context, args, Structure::BLS12_381_Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12_381_Fq12) => ark_field_mul_internal!(gas_params, context, args, Structure::BLS12_381_Fq12, ark_bls12_381::Fq12),
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

macro_rules! ark_neg_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, $structure, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element.neg();
        let new_handle = store_obj!($context, $structure, new_element);
        Ok(NativeResult::ok(
            $gas_params.neg($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn field_neg_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => ark_neg_internal!(gas_params, context, args, Structure::BLS12_381_Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12_381_Fq12) => ark_neg_internal!(gas_params, context, args, Structure::BLS12_381_Fq12, ark_bls12_381::Fq12),
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

macro_rules! ark_field_inv_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, $structure, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        match element.inverse() {
            Some(new_element) => {
                let new_handle = store_obj!($context, $structure, new_element);
                Ok(NativeResult::ok(
                    $gas_params.field_inv($structure),
                    smallvec![Value::bool(true), Value::u64(new_handle as u64)],
                ))
            }
            None => {
                Ok(NativeResult::ok(
                    $gas_params.field_inv($structure),
                    smallvec![Value::bool(false), Value::u64(0)],
                ))
            },
        }
    }}
}

fn field_inv_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => ark_field_inv_internal!(gas_params, context, args, Structure::BLS12_381_Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12_381_Fq12) => ark_field_inv_internal!(gas_params, context, args, Structure::BLS12_381_Fq12, ark_bls12_381::Fq12),
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}


macro_rules! ark_eq_internal {
    ($gas_params:ident, $context:ident, $args:ident, $structure:expr, $typ:ty) => {{
            let handle_2 = pop_arg!($args, u64) as usize;
            let handle_1 = pop_arg!($args, u64) as usize;
            let element_1_ptr = get_obj_pointer!($context, $structure, handle_1);
            let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
            let element_2_ptr = get_obj_pointer!($context, $structure, handle_2);
            let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
            Ok(NativeResult::ok(
                $gas_params.eq($structure),
                smallvec![Value::bool(element_1 == element_2)],
            ))
    }}
}

fn eq_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_Fr) => ark_eq_internal!(gas_params, context, args, Structure::BLS12_381_Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12_381_Fq12) => ark_eq_internal!(gas_params, context, args, Structure::BLS12_381_Fq12, ark_bls12_381::Fq12),
        Some(Structure::BLS12_381_G1) => ark_eq_internal!(gas_params, context, args, Structure::BLS12_381_G1, ark_bls12_381::G1Projective),
        _ => {
            Ok(NativeResult::err(
                InternalGas::zero(),
                NOT_IMPLEMENTED,
            ))
        }
    }
}

macro_rules! ark_group_identity_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $typ:ty) => {{
        let element = <$typ>::zero();
            let handle = store_obj!($context, $structure, element);
        Ok(NativeResult::ok(
            $gas_params.group_identity($structure),
            smallvec![Value::u64(handle as u64)],
        ))
    }}
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
        Some(Structure::BLS12_381_G1) => ark_group_identity_internal!(gas_params, context, Structure::BLS12_381_G1, ark_bls12_381::G1Projective),
        Some(Structure::BLS12_381_G2) => ark_group_identity_internal!(gas_params, context, Structure::BLS12_381_G2, ark_bls12_381::G2Projective),
        Some(Structure::BLS12_381_Gt) => {
            let element = Fq12::one();
            let handle = store_obj!(context, Structure::BLS12_381_Gt, element);
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

static BLS12381_R_LENDIAN: Lazy<Vec<u8>> = Lazy::new(||{
    hex::decode("01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73").unwrap()
});

static BLS12381_R_SCALAR: Lazy<ark_ff::BigInteger256> = Lazy::new(||{
    ark_ff::BigInteger256::deserialize_uncompressed(BLS12381_R_LENDIAN.as_slice()).unwrap()
});

macro_rules! ark_group_generator_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $typ:ty) => {{
        let element = <$typ>::prime_subgroup_generator();
        let handle = store_obj!($context, $structure, element);
        Ok(NativeResult::ok(
            $gas_params.group_generator($structure),
            smallvec![Value::u64(handle as u64)],
        ))
    }}
}

fn group_generator_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => ark_group_generator_internal!(gas_params, context, Structure::BLS12_381_G1, ark_bls12_381::G1Projective),
        Some(Structure::BLS12_381_G2) => ark_group_generator_internal!(gas_params, context, Structure::BLS12_381_G2, ark_bls12_381::G2Projective),
        Some(Structure::BLS12_381_Gt) => {
            let element = BLS12381_GT_GENERATOR.clone();
            let handle = store_obj!(context, Structure::BLS12_381_Gt, element);
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
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) | Some(Structure::BLS12_381_G2) | Some(Structure::BLS12_381_Gt) => {
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::vector_u8(BLS12381_R_LENDIAN.clone())],
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
        Some(Structure::BLS12_381_G1) => {
            abort_if_feature_disabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let element = G1Projective::rand(&mut test_rng());
            let handle = store_bls12_381_g1!(context, element);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        Some(Structure::BLS12_381_G2) => {
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

macro_rules! ark_group_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle_2 = pop_arg!($args, u64) as usize;
        let handle_1 = pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, $structure, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, $structure, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element_1.$op(element_2);
        let new_handle = store_obj!($context, $structure, new_element);
        Ok(NativeResult::ok(
            $gas_params.group_add($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn group_add_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    abort_if_feature_disabled!(context, FeatureFlag::GENERIC_ALGEBRAIC_BASIC_OPERATIONS);
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => ark_group_add_internal!(gas_params, context, args, Structure::BLS12_381_G1, ark_bls12_381::G1Projective, add),
        Some(Structure::BLS12_381_G2) => ark_group_add_internal!(gas_params, context, args, Structure::BLS12_381_G2, ark_bls12_381::G2Projective, add),
        Some(Structure::BLS12_381_Gt) => ark_group_add_internal!(gas_params, context, args, Structure::BLS12_381_Gt, ark_bls12_381::Fq12, mul),
        _ => Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
    }
}

macro_rules! ark_group_scalar_mul_internal {
    ($gas_params:expr, $context:expr, $args:ident, $group_structure:expr, $scalar_structure:expr, $group_typ:ty, $scalar_typ:ty, $op:ident) => {{
        let scalar_handle = pop_arg!($args, u64) as usize;
        let element_handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, $group_structure, element_handle);
        let element = element_ptr.downcast_ref::<$group_typ>().unwrap();
        let scalar_ptr = get_obj_pointer!($context, $scalar_structure, scalar_handle);
        let scalar = scalar_ptr.downcast_ref::<$scalar_typ>().unwrap();
        let new_element = element.$op(scalar.into_repr());
        let new_handle = store_obj!($context, $group_structure, new_element);
        Ok(NativeResult::ok(
            $gas_params.group_scalar_mul($group_structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
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
    match (group_structure, scalar_structure) {
        (Some(Structure::BLS12_381_G1), Some(Structure::BLS12_381_Fr)) => {
            ark_group_scalar_mul_internal!(gas_params, context, args, Structure::BLS12_381_G1, Structure::BLS12_381_Fr, ark_bls12_381::G1Projective, ark_bls12_381::Fr, mul)
        }
        (Some(Structure::BLS12_381_G2), Some(Structure::BLS12_381_Fr)) => {
            ark_group_scalar_mul_internal!(gas_params, context, args, Structure::BLS12_381_G2, Structure::BLS12_381_Fr, ark_bls12_381::G2Projective, ark_bls12_381::Fr, mul)
        }
        (Some(Structure::BLS12_381_Gt), Some(Structure::BLS12_381_Fr)) => {
            ark_group_scalar_mul_internal!(gas_params, context, args, Structure::BLS12_381_Gt, Structure::BLS12_381_Fr, ark_bls12_381::Fq12, ark_bls12_381::Fr, pow)
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

macro_rules! ark_group_double_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, $structure, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element.$op();
        let new_handle = store_obj!($context, $structure, new_element);
        Ok(NativeResult::ok(
            $gas_params.group_double($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn group_double_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            ark_group_double_internal!(gas_params, context, args, Structure::BLS12_381_G1, ark_bls12_381::G1Projective, double)
        }
        Some(Structure::BLS12_381_G2) => {
            ark_group_double_internal!(gas_params, context, args, Structure::BLS12_381_G2, ark_bls12_381::G2Projective, double)
        }
        Some(Structure::BLS12_381_Gt) => {
            ark_group_double_internal!(gas_params, context, args, Structure::BLS12_381_Gt, ark_bls12_381::Fq12, square)
        }
        _ => {
            Ok(NativeResult::err(InternalGas::zero(), NOT_IMPLEMENTED))
        }
    }
}

macro_rules! ark_group_neg_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, $structure, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element.$op();
        let new_handle = store_obj!($context, $structure, new_element);
        Ok(NativeResult::ok(
            $gas_params.group_neg($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
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
    let x = Fq12::one();
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12_381_G1) => {
            ark_group_neg_internal!(gas_params, context, args, Structure::BLS12_381_G1, ark_bls12_381::G1Projective, neg)
        }
        Some(Structure::BLS12_381_G2) => {
            ark_group_neg_internal!(gas_params, context, args, Structure::BLS12_381_G2, ark_bls12_381::G2Projective, neg)
        }
        Some(Structure::BLS12_381_Gt) => {
            ark_group_neg_internal!(gas_params, context, args, Structure::BLS12_381_Gt, ark_bls12_381::Fq12, inverse)
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
    let g1 = structure_from_ty_arg!(context, &ty_args[0]);
    let g2 = structure_from_ty_arg!(context, &ty_args[1]);
    let gt = structure_from_ty_arg!(context, &ty_args[2]);
    let g2_handles = pop_arg!(args, Vec<u64>);
    let g1_handles = pop_arg!(args, Vec<u64>);
    match (g1, g2, gt) {
        (Some(Structure::BLS12_381_G1), Some(Structure::BLS12_381_G2), Some(Structure::BLS12_381_Gt)) => {
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
