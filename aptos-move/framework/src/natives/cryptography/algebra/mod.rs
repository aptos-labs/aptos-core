// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0


use std::any::Any;
use std::collections::VecDeque;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::rc::Rc;
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::{Field, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
#[cfg(feature = "testing")]
use ark_std::{test_rng, UniformRand};
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, NumArgs};
use move_core_types::language_storage::TypeTag;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use move_vm_types::values::VectorRef;

use num_traits::{One, Zero};
use once_cell::sync::Lazy;
use smallvec::smallvec;
use crate::natives::cryptography::algebra::gas::GasParameters;
use crate::natives::util::make_native_from_func;
#[cfg(feature = "testing")]
use crate::natives::util::make_test_only_native_from_func;

pub mod gas;

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12381Fq12,
    BLS12381G1,
    BLS12381G2,
    BLS12381Gt,
    BLS12381Fr,
}

impl Structure {
    pub fn from_type_tag(type_tag: &TypeTag) -> Option<Structure> {
        match type_tag.to_string().as_str() {
            "0x1::algebra::BLS12_381_Fr" => Some(Structure::BLS12381Fr),
            "0x1::algebra::BLS12_381_Fq12" => Some(Structure::BLS12381Fq12),
            "0x1::algebra::BLS12_381_G1" => Some(Structure::BLS12381G1),
            "0x1::algebra::BLS12_381_G2" => Some(Structure::BLS12381G2),
            "0x1::algebra::BLS12_381_Gt" => Some(Structure::BLS12381Gt),
            _ => None
        }
    }
}

#[derive(Tid)]
#[derive(Default)]
pub struct AlgebraContext {
    objs: Vec<Rc<dyn Any>>,
}

impl AlgebraContext {
    pub fn new() -> Self {
        Self {
            objs: Vec::new(),
        }
    }
}

macro_rules! structure_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        Structure::from_type_tag(&type_tag)
    }}
}

macro_rules! store_obj {
    ($context:expr, $obj:expr) => {{
        let target_vec = &mut $context.extensions_mut().get_mut::<AlgebraContext>().objs;
        let ret = target_vec.len();
        target_vec.push(Rc::new($obj));
        ret
    }}
}

macro_rules! get_obj_pointer {
    ($context:expr, $handle:expr) => {{
        $context.extensions_mut().get_mut::<AlgebraContext>().objs[$handle].clone()
    }}
}

// Pre-defined serialization scheme IDs.
// They has to match those in `aptos-move/framework/aptos-stdlib/sources/cryptography/algebra.move`.
static BLS12_381_FQ12_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("04").unwrap());
static BLS12_381_G1_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("06").unwrap());
static BLS12_381_G1_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0601").unwrap());
static BLS12_381_G2_UNCOMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("08").unwrap());
static BLS12_381_G2_COMPRESSED_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0801").unwrap());
static BLS12_381_GT_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("09").unwrap());
static BLS12_381_FR_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0a").unwrap());
static BLS12_381_FR_BENDIAN_FORMAT: Lazy<Vec<u8>> = Lazy::new(||hex::decode("0a01").unwrap());


macro_rules! ark_serialize_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $handle:expr, $scheme:expr, $typ:ty, $ser_func:ident) => {{
            let element_ptr = get_obj_pointer!($context, $handle);
            let element = element_ptr.downcast_ref::<$typ>().unwrap();
            let mut buf = Vec::new();
            element.$ser_func(&mut buf).unwrap();
            let cost = $gas_params.serialize($structure, $scheme);
            (cost, buf)
    }}
}

macro_rules! ark_ec_point_serialize_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $handle:expr, $scheme:expr, $typ:ty, $ser_func:ident) => {{
        let element_ptr = get_obj_pointer!($context, $handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
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
        (Some(Structure::BLS12381Fr), scheme) if scheme.as_slice() == BLS12_381_FR_FORMAT.as_slice() => {
            let (cost, buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12381Fr, handle, scheme.as_slice(), ark_bls12_381::Fr, serialize_uncompressed);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12381Fr), scheme) if scheme.as_slice() == BLS12_381_FR_BENDIAN_FORMAT.as_slice() => {
            let (cost, mut buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12381Fr, handle, scheme.as_slice(), ark_bls12_381::Fr, serialize_uncompressed);
            buf.reverse();
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12381Fq12), scheme) if scheme.as_slice() == BLS12_381_FQ12_FORMAT.as_slice() => {
            let (cost, buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12381Fq12, handle, scheme.as_slice(), ark_bls12_381::Fq12, serialize_uncompressed);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12381G1), scheme) if scheme.as_slice() == BLS12_381_G1_UNCOMPRESSED_FORMAT.as_slice() => {
            let (cost, buf) = ark_ec_point_serialize_internal!(gas_params, context, Structure::BLS12381G1, handle, scheme.as_slice(), ark_bls12_381::G1Projective, serialize_uncompressed);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12381G1), scheme) if scheme.as_slice() == BLS12_381_G1_COMPRESSED_FORMAT.as_slice() => {
            let (cost, buf) = ark_ec_point_serialize_internal!(gas_params, context, Structure::BLS12381G1, handle, scheme.as_slice(), ark_bls12_381::G1Projective, serialize);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12381G2), scheme) if scheme.as_slice() == BLS12_381_G2_UNCOMPRESSED_FORMAT.as_slice() => {
            let (cost, buf) = ark_ec_point_serialize_internal!(gas_params, context, Structure::BLS12381G2, handle, scheme.as_slice(), ark_bls12_381::G2Projective, serialize_uncompressed);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12381G2), scheme) if scheme.as_slice() == BLS12_381_G2_COMPRESSED_FORMAT.as_slice() => {
            let (cost, buf) = ark_ec_point_serialize_internal!(gas_params, context, Structure::BLS12381G2, handle, scheme.as_slice(), ark_bls12_381::G2Projective, serialize);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        (Some(Structure::BLS12381Gt), scheme) if scheme.as_slice() == BLS12_381_GT_FORMAT.as_slice() => {
            let (cost, buf) = ark_serialize_internal!(gas_params, context, Structure::BLS12381Gt, handle, scheme.as_slice(), ark_bls12_381::Fq12, serialize_uncompressed);
            Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(buf)]))
        }
        _ => unreachable!()
    }
}

macro_rules! ark_deserialize_internal {
    ($gas_params:expr, $context:expr, $bytes:expr, $structure:expr, $scheme:expr, $typ:ty, $deser_func:ident) => {{
        match <$typ>::$deser_func($bytes) {
            Ok(element) => {
                let handle = store_obj!($context, element);
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

macro_rules! ark_ec_point_deserialize_internal {
    ($gas_params:expr, $context:expr, $bytes:expr, $structure:expr, $scheme:expr, $typ:ty, $deser_func:ident) => {{
        match <$typ>::$deser_func($bytes) {
            Ok(element) => {
                let element_proj = element.into_projective();
                let handle = store_obj!($context, element_proj);
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
    let vector_ref = pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let bytes = bytes_ref.as_slice();
    let scheme = pop_arg!(args, Vec<u8>);
    match (structure,scheme) {
        (Some(Structure::BLS12381Fr), scheme) if scheme.as_slice() == BLS12_381_FR_FORMAT.as_slice() => {
            if bytes.len() != 32 {
                return Ok(NativeResult::ok(InternalGas::zero(), smallvec![Value::bool(false), Value::u64(0)]));
            }
            ark_deserialize_internal!(gas_params, context, bytes, Structure::BLS12381Fr, scheme.as_slice(), ark_bls12_381::Fr, deserialize_uncompressed)
        }
        (Some(Structure::BLS12381Fr), scheme) if scheme.as_slice() == BLS12_381_FR_BENDIAN_FORMAT.as_slice() => {
            if bytes.len() != 32 {
                return Ok(NativeResult::ok(InternalGas::zero(), smallvec![Value::bool(false), Value::u64(0)]));
            }
            let mut lendian: Vec<u8> = bytes.to_vec();
            lendian.reverse();
            let bytes = lendian.as_slice();
            ark_deserialize_internal!(gas_params, context, bytes, Structure::BLS12381Fr, scheme.as_slice(), ark_bls12_381::Fr, deserialize_uncompressed)
        }
        (Some(Structure::BLS12381Fq12), scheme) if scheme.as_slice() == BLS12_381_FQ12_FORMAT.as_slice() => {
            if bytes.len() != 576 {
                return Ok(NativeResult::ok(InternalGas::zero(), smallvec![Value::bool(false), Value::u64(0)]));
            }
            ark_deserialize_internal!(gas_params, context, bytes, Structure::BLS12381Fq12, scheme.as_slice(), ark_bls12_381::Fq12, deserialize_uncompressed)
        }
        (Some(Structure::BLS12381G1), scheme) if scheme.as_slice() == BLS12_381_G1_UNCOMPRESSED_FORMAT.as_slice() => {
            if bytes.len() != 96 {
                return Ok(NativeResult::ok(InternalGas::zero(), smallvec![Value::bool(false), Value::u64(0)]));
            }
            ark_ec_point_deserialize_internal!(gas_params, context, bytes, Structure::BLS12381G1, scheme.as_slice(), ark_bls12_381::G1Affine, deserialize_uncompressed)
        }
        (Some(Structure::BLS12381G1), scheme) if scheme.as_slice() == BLS12_381_G1_COMPRESSED_FORMAT.as_slice() => {
            if bytes.len() != 48 {
                return Ok(NativeResult::ok(InternalGas::zero(), smallvec![Value::bool(false), Value::u64(0)]));
            }
            ark_ec_point_deserialize_internal!(gas_params, context, bytes, Structure::BLS12381G1, scheme.as_slice(), ark_bls12_381::G1Affine, deserialize)
        }
        (Some(Structure::BLS12381G2), scheme) if scheme.as_slice() == BLS12_381_G2_UNCOMPRESSED_FORMAT.as_slice() => {
            if bytes.len() != 192 {
                return Ok(NativeResult::ok(InternalGas::zero(), smallvec![Value::bool(false), Value::u64(0)]));
            }
            ark_ec_point_deserialize_internal!(gas_params, context, bytes, Structure::BLS12381G2, scheme.as_slice(), ark_bls12_381::G2Affine, deserialize_uncompressed)
        }
        (Some(Structure::BLS12381G2), scheme) if scheme.as_slice() == BLS12_381_G2_COMPRESSED_FORMAT.as_slice() => {
            if bytes.len() != 96 {
                return Ok(NativeResult::ok(InternalGas::zero(), smallvec![Value::bool(false), Value::u64(0)]));
            }
            ark_ec_point_deserialize_internal!(gas_params, context, bytes, Structure::BLS12381G2, scheme.as_slice(), ark_bls12_381::G2Affine, deserialize)
        }
        (Some(Structure::BLS12381Gt), scheme) if scheme.as_slice() == BLS12_381_GT_FORMAT.as_slice() => {
            if bytes.len() != 576 {
                return Ok(NativeResult::ok(InternalGas::zero(), smallvec![ Value::bool(false), Value::u64(0)]));
            }
            match <ark_bls12_381::Fq12>::deserialize_uncompressed(bytes) {
                Ok(element) => {
                    if element.pow(BLS12381_R_SCALAR.0) == ark_bls12_381::Fq12::one() {
                        let handle = store_obj!( context , element );
                        Ok(NativeResult::ok(
                            gas_params.deserialize(Structure::BLS12381Gt, scheme.as_slice()),
                            smallvec![Value::bool(true), Value::u64(handle as u64)],
                        ))
                    } else {
                        Ok(NativeResult::ok(
                            gas_params.deserialize(Structure::BLS12381Gt, scheme.as_slice()),
                            smallvec![Value::bool(false), Value::u64(0)],
                        ))
                    }
                }
                _ => {
                    Ok(NativeResult::ok(
                        gas_params.deserialize(Structure::BLS12381Gt, scheme.as_slice()),
                        smallvec![Value::bool(false), Value::u64(0)],
                    ))
                }
            }
        }
        _ => unreachable!()
    }
}

macro_rules! from_u64_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
            let value = pop_arg!($args, u64);
            let element = <$typ>::from(value as u128);
            let handle = store_obj!($context, element);
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
        Some(Structure::BLS12381Fr) =>
            from_u64_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            from_u64_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = pop_arg!($args, u64) as usize;
        let handle_1 = pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element_1.add(element_2);
        let new_handle = store_obj!($context, new_element);
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
        Some(Structure::BLS12381Fr) =>
            ark_field_add_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_add_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_sub_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = pop_arg!($args, u64) as usize;
        let handle_1 = pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element_1.sub(element_2);
        let new_handle = store_obj!($context, new_element);
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
        Some(Structure::BLS12381Fr) =>
            ark_field_sub_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_sub_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_mul_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = pop_arg!($args, u64) as usize;
        let handle_1 = pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element_1.mul(element_2);
        let new_handle = store_obj!($context, new_element);
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
        Some(Structure::BLS12381Fr) =>
            ark_field_mul_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_mul_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_div_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
            let handle_2 = pop_arg!($args, u64) as usize;
            let handle_1 = pop_arg!($args, u64) as usize;
            let element_1_ptr = get_obj_pointer!($context, handle_1);
            let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
            let element_2_ptr = get_obj_pointer!($context, handle_2);
            let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
            if element_2.is_zero() {
                return Ok(NativeResult::ok(
                    InternalGas::zero(),
                    smallvec![Value::bool(false), Value::u64(0_u64)],
                ));
            }
            let new_element = element_1.div(element_2);
            let new_handle = store_obj!($context, new_element);
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
        Some(Structure::BLS12381Fr) =>
            ark_field_div_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_div_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_neg_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element.neg();
        let new_handle = store_obj!($context, new_element);
        Ok(NativeResult::ok(
            $gas_params.field_neg($structure),
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
        Some(Structure::BLS12381Fr) =>
            ark_neg_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_neg_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_inv_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        match element.inverse() {
            Some(new_element) => {
                let new_handle = store_obj!($context, new_element);
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
        Some(Structure::BLS12381Fr) =>
            ark_field_inv_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_inv_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_sqr_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element.square();
        let new_handle = store_obj!($context, new_element);
        Ok(NativeResult::ok(
            $gas_params.field_sqr($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn field_sqr_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) =>
            ark_field_sqr_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_sqr_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_zero_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let new_element = <$typ>::zero();
        let new_handle = store_obj!($context, new_element);
        Ok(NativeResult::ok(
            $gas_params.field_zero($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn field_zero_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) =>
            ark_field_zero_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_zero_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_one_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let new_element = <$typ>::one();
        let new_handle = store_obj!($context, new_element);
        Ok(NativeResult::ok(
            $gas_params.field_one($structure),
            smallvec![Value::u64(new_handle as u64)],
        ))
    }}
}

fn field_one_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) =>
            ark_field_one_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_one_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_is_one_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let result = element.is_one();
        Ok(NativeResult::ok(
            $gas_params.field_is_one($structure),
            smallvec![Value::bool(result)],
        ))
    }}
}

fn field_is_one_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) =>
            ark_field_is_one_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_is_one_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_field_is_zero_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let result = element.is_zero();
        Ok(NativeResult::ok(
            $gas_params.field_is_zero($structure),
            smallvec![Value::bool(result)],
        ))
    }}
}

fn field_is_zero_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) =>
            ark_field_is_zero_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_field_is_zero_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_eq_internal {
    ($gas_params:ident, $context:ident, $args:ident, $structure:expr, $typ:ty) => {{
            let handle_2 = pop_arg!($args, u64) as usize;
            let handle_1 = pop_arg!($args, u64) as usize;
            let element_1_ptr = get_obj_pointer!($context, handle_1);
            let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
            let element_2_ptr = get_obj_pointer!($context, handle_2);
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
        Some(Structure::BLS12381Fr) =>
            ark_eq_internal!(gas_params, context, args, Structure::BLS12381Fr, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_eq_internal!(gas_params, context, args, Structure::BLS12381Fq12, ark_bls12_381::Fq12),
        Some(Structure::BLS12381G1) =>
            ark_eq_internal!(gas_params, context, args, Structure::BLS12381G1, ark_bls12_381::G1Projective),
        Some(Structure::BLS12381G2) =>
            ark_eq_internal!(gas_params, context, args, Structure::BLS12381G2, ark_bls12_381::G2Projective),
        Some(Structure::BLS12381Gt) =>
            ark_eq_internal!(gas_params, context, args, Structure::BLS12381Gt, ark_bls12_381::Fq12),
        _ => unreachable!()
    }
}

macro_rules! ark_group_identity_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $typ:ty, $func:ident) => {{
        let element = <$typ>::$func();
        let handle = store_obj!($context, element);
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
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381G1) =>
            ark_group_identity_internal!(gas_params, context, Structure::BLS12381G1, ark_bls12_381::G1Projective, zero),
        Some(Structure::BLS12381G2) =>
            ark_group_identity_internal!(gas_params, context, Structure::BLS12381G2, ark_bls12_381::G2Projective, zero),
        Some(Structure::BLS12381Gt) =>
            ark_group_identity_internal!(gas_params, context, Structure::BLS12381Gt, ark_bls12_381::Fq12, one),
        _ => unreachable!()
    }
}

static BLS12381_GT_GENERATOR: Lazy<ark_bls12_381::Fq12> = Lazy::new(||{
    let buf = hex::decode("b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f").unwrap();
    ark_bls12_381::Fq12::deserialize(buf.as_slice()).unwrap()
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
        let handle = store_obj!($context, element);
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
        Some(Structure::BLS12381G1) =>
            ark_group_generator_internal!(gas_params, context, Structure::BLS12381G1, ark_bls12_381::G1Projective),
        Some(Structure::BLS12381G2) =>
            ark_group_generator_internal!(gas_params, context, Structure::BLS12381G2, ark_bls12_381::G2Projective),
        Some(Structure::BLS12381Gt) => {
            let element = BLS12381_GT_GENERATOR.add(ark_bls12_381::Fq12::zero());
            let handle = store_obj!(context, element);
            Ok(NativeResult::ok(
                gas_params.group_generator(Structure::BLS12381Gt),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => unreachable!()
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
        Some(Structure::BLS12381G1) | Some(Structure::BLS12381G2) | Some(Structure::BLS12381Gt) => {
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::vector_u8(BLS12381_R_LENDIAN.clone())],
            ))
        }
        _ => unreachable!()
    }
}

macro_rules! ark_insecure_random_element_internal {
    ($context:expr, $typ:ty) => {{
        let element = <$typ>::rand(&mut test_rng());
        let handle = store_obj!($context, element);
        Ok(NativeResult::ok(
            InternalGas::zero(),
            smallvec![Value::u64(handle as u64)],
        ))
    }}
}

#[cfg(feature = "testing")]
fn insecure_random_element_internal(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) =>
            ark_insecure_random_element_internal!(context, ark_bls12_381::Fr),
        Some(Structure::BLS12381Fq12) =>
            ark_insecure_random_element_internal!(context, ark_bls12_381::Fq12),
        Some(Structure::BLS12381G1) =>
            ark_insecure_random_element_internal!(context, ark_bls12_381::G1Projective),
        Some(Structure::BLS12381G2) =>
            ark_insecure_random_element_internal!(context, ark_bls12_381::G2Projective),
        Some(Structure::BLS12381Gt) => {
            let k = ark_bls12_381::Fr::rand(&mut test_rng());
            let element = BLS12381_GT_GENERATOR.pow(k.into_repr());
            let handle = store_obj!(context, element);
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle as u64)],
            ))
        }
        _ => unreachable!()
    }
}

macro_rules! ark_group_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle_2 = pop_arg!($args, u64) as usize;
        let handle_1 = pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element_1.$op(element_2);
        let new_handle = store_obj!($context, new_element);
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
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381G1) =>
            ark_group_add_internal!(gas_params, context, args, Structure::BLS12381G1, ark_bls12_381::G1Projective, add),
        Some(Structure::BLS12381G2) =>
            ark_group_add_internal!(gas_params, context, args, Structure::BLS12381G2, ark_bls12_381::G2Projective, add),
        Some(Structure::BLS12381Gt) =>
            ark_group_add_internal!(gas_params, context, args, Structure::BLS12381Gt, ark_bls12_381::Fq12, mul),
        _ => unreachable!()
    }
}

macro_rules! ark_group_scalar_mul_internal {
    ($gas_params:expr, $context:expr, $args:ident, $group_structure:expr, $scalar_structure:expr, $group_typ:ty, $scalar_typ:ty, $op:ident) => {{
        let scalar_handle = pop_arg!($args, u64) as usize;
        let element_handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, element_handle);
        let element = element_ptr.downcast_ref::<$group_typ>().unwrap();
        let scalar_ptr = get_obj_pointer!($context, scalar_handle);
        let scalar = scalar_ptr.downcast_ref::<$scalar_typ>().unwrap();
        let new_element = element.$op(scalar.into_repr());
        let new_handle = store_obj!($context, new_element);
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
    let group_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let scalar_field_opt = structure_from_ty_arg!(context, &ty_args[1]);
    match (group_opt, scalar_field_opt) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381Fr)) =>
            ark_group_scalar_mul_internal!(gas_params, context, args, Structure::BLS12381G1, Structure::BLS12_381_Fr, ark_bls12_381::G1Projective, ark_bls12_381::Fr, mul),
        (Some(Structure::BLS12381G2), Some(Structure::BLS12381Fr)) =>
            ark_group_scalar_mul_internal!(gas_params, context, args, Structure::BLS12381G2, Structure::BLS12_381_Fr, ark_bls12_381::G2Projective, ark_bls12_381::Fr, mul),
        (Some(Structure::BLS12381Gt), Some(Structure::BLS12381Fr)) =>
            ark_group_scalar_mul_internal!(gas_params, context, args, Structure::BLS12381Gt, Structure::BLS12_381_Fr, ark_bls12_381::Fq12, ark_bls12_381::Fr, pow),
        _ => unreachable!()
    }
}

macro_rules! ark_group_double_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element.$op();
        let new_handle = store_obj!($context, new_element);
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
        Some(Structure::BLS12381G1) =>
            ark_group_double_internal!(gas_params, context, args, Structure::BLS12381G1, ark_bls12_381::G1Projective, double),
        Some(Structure::BLS12381G2) =>
            ark_group_double_internal!(gas_params, context, args, Structure::BLS12381G2, ark_bls12_381::G2Projective, double),
        Some(Structure::BLS12381Gt) =>
            ark_group_double_internal!(gas_params, context, args, Structure::BLS12381Gt, ark_bls12_381::Fq12, square),
        _ => unreachable!()
    }
}

macro_rules! ark_group_neg_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle = pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let new_element = element.$op();
        let new_handle = store_obj!($context, new_element);
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
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381G1) =>
            ark_group_neg_internal!(gas_params, context, args, Structure::BLS12381G1, ark_bls12_381::G1Projective, neg),
        Some(Structure::BLS12381G2) =>
            ark_group_neg_internal!(gas_params, context, args, Structure::BLS12381G2, ark_bls12_381::G2Projective, neg),
        Some(Structure::BLS12381Gt) => {
            let handle = pop_arg!( args , u64 ) as usize;
            let element_ptr = get_obj_pointer!(context, handle);
            let element = element_ptr.downcast_ref::<ark_bls12_381::Fq12>().unwrap();
            let new_element = element.inverse().unwrap();
            let new_handle = store_obj!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.group_neg(Structure::BLS12381Gt),
                smallvec![ Value :: u64 ( new_handle as u64 ) ],
            ))
        }
        _ => unreachable!()
    }
}

fn pairing_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(3, ty_args.len());
    let g1_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let g2_opt = structure_from_ty_arg!(context, &ty_args[1]);
    let gt_opt = structure_from_ty_arg!(context, &ty_args[2]);
    match (g1_opt, g2_opt, gt_opt) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381G2), Some(Structure::BLS12381Gt)) => {
            let g2_element_handle = pop_arg!(args, u64) as usize;
            let g1_element_handle = pop_arg!(args, u64) as usize;
            let g1_element_ptr = get_obj_pointer!(context, g1_element_handle);
            let g2_element_ptr = get_obj_pointer!(context, g2_element_handle);
            let g1_element = g1_element_ptr.downcast_ref::<ark_bls12_381::G1Projective>().unwrap();
            let g2_element = g2_element_ptr.downcast_ref::<ark_bls12_381::G2Projective>().unwrap();
            let g1_element_affine = g1_element.into_affine();
            let g2_element_affine = g2_element.into_affine();
            let new_element = ark_bls12_381::Bls12_381::pairing(g1_element_affine, g2_element_affine);
            let new_handle = store_obj!(context, new_element);
            Ok(NativeResult::ok(
                gas_params.pairing(Structure::BLS12381G1, Structure::BLS12381G2, Structure::BLS12381Gt),
                smallvec![Value::u64(new_handle as u64)],
            ))
        }
        _ => unreachable!()
    }
}

fn downcast_internal(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(2, ty_args.len());
    let parent_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let child_opt = structure_from_ty_arg!(context, &ty_args[1]);
    match (parent_opt, child_opt) {
        (Some(Structure::BLS12381Fq12), Some(Structure::BLS12381Gt)) => {
            let handle = pop_arg!(args, u64) as usize;
            let element_ptr = get_obj_pointer!(context, handle);
            let element = element_ptr.downcast_ref::<ark_bls12_381::Fq12>().unwrap();
            if element.pow(BLS12381_R_SCALAR.0) == ark_bls12_381::Fq12::one() {
                Ok(NativeResult::ok(
                    gas_params.ark_bls12_381_fq12_pow_u256 * NumArgs::one(),
                    smallvec![Value::bool(true), Value::u64(handle as u64)],
                ))
            } else {
                Ok(NativeResult::ok(
                    gas_params.ark_bls12_381_fq12_pow_u256 * NumArgs::one(),
                    smallvec![Value::bool(false), Value::u64(handle as u64)],
                ))
            }
        }
        _ => unreachable!()
    }
}

fn upcast_internal(
    _gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(2, ty_args.len());
    let child_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let parent_opt = structure_from_ty_arg!(context, &ty_args[1]);
    let handle = pop_arg!(args, u64);
    match (child_opt, parent_opt) {
        (Some(Structure::BLS12381Gt), Some(Structure::BLS12381Fq12)) => {
            Ok(NativeResult::ok(
                InternalGas::zero(),
                smallvec![Value::u64(handle)],
            ))
        }
        _ => unreachable!()
    }
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "deserialize_internal",
            make_native_from_func(gas_params.clone(), deserialize_internal),
        ),
        (
            "downcast_internal",
            make_native_from_func(gas_params.clone(), downcast_internal),
        ),
        (
            "eq_internal",
            make_native_from_func(gas_params.clone(), eq_internal),
        ),
        (
            "field_add_internal",
            make_native_from_func(gas_params.clone(), field_add_internal),
        ),
        (
            "field_div_internal",
            make_native_from_func(gas_params.clone(), field_div_internal),
        ),
        (
            "field_inv_internal",
            make_native_from_func(gas_params.clone(), field_inv_internal),
        ),
        (
            "field_is_one_internal",
            make_native_from_func(gas_params.clone(), field_is_one_internal),
        ),
        (
            "field_is_zero_internal",
            make_native_from_func(gas_params.clone(), field_is_zero_internal),
        ),
        (
            "field_mul_internal",
            make_native_from_func(gas_params.clone(), field_mul_internal),
        ),
        (
            "field_neg_internal",
            make_native_from_func(gas_params.clone(), field_neg_internal),
        ),
        (
            "field_one_internal",
            make_native_from_func(gas_params.clone(), field_one_internal),
        ),
        (
            "field_sqr_internal",
            make_native_from_func(gas_params.clone(), field_sqr_internal),
        ),
        (
            "field_sub_internal",
            make_native_from_func(gas_params.clone(), field_sub_internal),
        ),
        (
            "field_zero_internal",
            make_native_from_func(gas_params.clone(), field_zero_internal),
        ),
        (
            "from_u64_internal",
            make_native_from_func(gas_params.clone(), from_u64_internal),
        ),
        (
            "group_add_internal",
            make_native_from_func(gas_params.clone(), group_add_internal),
        ),
        (
            "group_double_internal",
            make_native_from_func(gas_params.clone(), group_double_internal),
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
            "group_neg_internal",
            make_native_from_func(gas_params.clone(), group_neg_internal),
        ),
        (
            "group_order_internal",
            make_native_from_func(gas_params.clone(), group_order_internal),
        ),
        (
            "group_scalar_mul_internal",
            make_native_from_func(gas_params.clone(), group_scalar_mul_internal),
        ),
        (
            "pairing_internal",
            make_native_from_func(gas_params.clone(), pairing_internal),
        ),
        (
            "serialize_internal",
            make_native_from_func(gas_params.clone(), serialize_internal),
        ),
        (
            "upcast_internal",
            make_native_from_func(gas_params, upcast_internal),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![
        (
            "insecure_random_element_internal",
            make_test_only_native_from_func(insecure_random_element_internal),
        ),
    ]);

    crate::natives::helpers::make_module_natives(natives)
}
