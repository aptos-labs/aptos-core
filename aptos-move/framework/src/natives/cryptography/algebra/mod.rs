// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
use crate::natives::helpers::make_test_only_native_from_func;
use crate::{
    natives::{
        cryptography::algebra::gas::GasParameters,
        helpers::{make_safe_native, SafeNativeContext, SafeNativeError, SafeNativeResult},
    },
    safely_pop_arg,
};
use aptos_types::on_chain_config::{FeatureFlag, Features, TimedFeatures};
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
#[cfg(feature = "testing")]
use ark_std::{test_rng, UniformRand};
use better_any::{Tid, TidAble};
#[cfg(feature = "testing")]
use move_binary_format::errors::PartialVMResult;
#[cfg(feature = "testing")]
use move_core_types::gas_algebra::InternalGas;
use move_core_types::language_storage::TypeTag;
#[cfg(feature = "testing")]
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::NativeFunction;
#[cfg(feature = "testing")]
use move_vm_types::natives::function::NativeResult;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use num_traits::{One, Zero};
use smallvec::{smallvec, SmallVec};
use std::{
    any::Any,
    collections::VecDeque,
    hash::Hash,
    ops::{Add, Div, Mul, Neg, Sub},
    rc::Rc,
    sync::Arc,
};

pub mod gas;

/// Equivalent to `std::error::not_implemented(0)` in Move.
const MOVE_ABORT_CODE_NOT_IMPLEMENTED: u64 = 0x0C0000;

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12381Fq12,
    BLS12381G1Affine,
    BLS12381G2Affine,
    BLS12381Gt,
    BLS12381Fr,
}

impl Structure {
    pub fn from_type_tag(type_tag: &TypeTag) -> Option<Structure> {
        match type_tag.to_string().as_str() {
            "0x1::algebra_bls12381::Fr" => Some(Structure::BLS12381Fr),
            "0x1::algebra_bls12381::Fq12" => Some(Structure::BLS12381Fq12),
            "0x1::algebra_bls12381::G1Affine" => Some(Structure::BLS12381G1Affine),
            "0x1::algebra_bls12381::G2Affine" => Some(Structure::BLS12381G2Affine),
            "0x1::algebra_bls12381::Gt" => Some(Structure::BLS12381Gt),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum SerializationFormat {
    BLS12381Fq12LscLscLscLsb,
    BLS12381G1AffineCompressed,
    BLS12381G1AffineUncompressed,
    BLS12381G2AffineCompressed,
    BLS12381G2AffineUnompressed,
    BLS12381Gt,
    BLS12381FrLsb,
    BLS12381FrMsb,
}

impl TryFrom<u64> for SerializationFormat {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0x0400000000000000 => Ok(SerializationFormat::BLS12381Fq12LscLscLscLsb),
            0x0600000000000000 => Ok(SerializationFormat::BLS12381G1AffineUncompressed),
            0x0601000000000000 => Ok(SerializationFormat::BLS12381G1AffineCompressed),
            0x0800000000000000 => Ok(SerializationFormat::BLS12381G2AffineUnompressed),
            0x0801000000000000 => Ok(SerializationFormat::BLS12381G2AffineCompressed),
            0x0900000000000000 => Ok(SerializationFormat::BLS12381Gt),
            0x0A00000000000000 => Ok(SerializationFormat::BLS12381FrLsb),
            0x0A01000000000000 => Ok(SerializationFormat::BLS12381FrMsb),
            _ => Err(()),
        }
    }
}

/// Hash-to-structure suites.
pub enum HashToStructureSuite {
    Bls12381g1XmdSha256SswuRo,
    Bls12381g2XmdSha256SswuRo,
}

impl TryFrom<u64> for HashToStructureSuite {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0x0001000000000000 => Ok(HashToStructureSuite::Bls12381g1XmdSha256SswuRo),
            0x0002000000000000 => Ok(HashToStructureSuite::Bls12381g2XmdSha256SswuRo),
            _ => Err(()),
        }
    }
}

#[derive(Tid, Default)]
pub struct AlgebraContext {
    objs: Vec<Rc<dyn Any>>,
}

impl AlgebraContext {
    pub fn new() -> Self {
        Self { objs: Vec::new() }
    }
}

macro_rules! structure_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        Structure::from_type_tag(&type_tag)
    }};
}

macro_rules! store_obj {
    ($context:expr, $obj:expr) => {{
        let target_vec = &mut $context.extensions_mut().get_mut::<AlgebraContext>().objs;
        let ret = target_vec.len();
        target_vec.push(Rc::new($obj));
        ret
    }};
}

macro_rules! abort_unless_feature_enabled {
    ($context:ident, $feature:expr) => {
        if !$context.get_feature_flags().is_enabled($feature) {
            return Err(SafeNativeError::Abort {
                abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
            });
        }
    };
}

macro_rules! get_obj_pointer {
    ($context:expr, $handle:expr) => {{
        $context.extensions_mut().get_mut::<AlgebraContext>().objs[$handle].clone()
    }};
}

macro_rules! ark_serialize_internal {
    (
        $gas_params:expr,
        $context:expr,
        $structure:expr,
        $handle:expr,
        $format:expr,
        $typ:ty,
        $ser_func:ident
    ) => {{
        let element_ptr = get_obj_pointer!($context, $handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        let mut buf = Vec::new();
        $context.charge($gas_params.serialize($structure, $format))?;
        element.$ser_func(&mut buf).unwrap();
        buf
    }};
}

fn serialize_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let handle = safely_pop_arg!(args, u64) as usize;
    let format_opt = SerializationFormat::try_from(safely_pop_arg!(args, u64));
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Ok(SerializationFormat::BLS12381FrLsb)) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let buf = ark_serialize_internal!(
                gas_params,
                context,
                Structure::BLS12381Fr,
                handle,
                SerializationFormat::BLS12381FrLsb,
                ark_bls12_381::Fr,
                serialize_uncompressed
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Fr), Ok(SerializationFormat::BLS12381FrMsb)) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            let mut buf = ark_serialize_internal!(
                gas_params,
                context,
                Structure::BLS12381Fr,
                handle,
                SerializationFormat::BLS12381FrMsb,
                ark_bls12_381::Fr,
                serialize_uncompressed
            );
            buf.reverse();
            Ok(smallvec![Value::vector_u8(buf)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_deserialize_internal {
    (
        $gas_params:expr,
        $context:expr,
        $bytes:expr,
        $structure:expr,
        $format:expr,
        $typ:ty,
        $deser_func:ident
    ) => {{
        $context.charge($gas_params.deserialize($structure, $format))?;
        match <$typ>::$deser_func($bytes) {
            Ok(element) => {
                let handle = store_obj!($context, element);
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            },
            _ => Ok(smallvec![Value::bool(false), Value::u64(0)]),
        }
    }};
}

fn deserialize_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure = structure_from_ty_arg!(context, &ty_args[0]);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let bytes = bytes_ref.as_slice();
    let format_opt = SerializationFormat::try_from(safely_pop_arg!(args, u64));
    match (structure, format_opt) {
        (Some(Structure::BLS12381Fr), Ok(SerializationFormat::BLS12381FrLsb)) => {
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                gas_params,
                context,
                bytes,
                Structure::BLS12381Fr,
                SerializationFormat::BLS12381FrLsb,
                ark_bls12_381::Fr,
                deserialize_uncompressed
            )
        },
        (Some(Structure::BLS12381Fr), Ok(SerializationFormat::BLS12381FrMsb)) => {
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            let mut lendian: Vec<u8> = bytes.to_vec();
            lendian.reverse();
            let bytes = lendian.as_slice();
            ark_deserialize_internal!(
                gas_params,
                context,
                bytes,
                Structure::BLS12381Fr,
                SerializationFormat::BLS12381FrMsb,
                ark_bls12_381::Fr,
                deserialize_uncompressed
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! from_u64_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let value = safely_pop_arg!($args, u64);
        $context.charge($gas_params.from_u128($structure))?;
        let element = <$typ>::from(value as u128);
        let handle = store_obj!($context, element);
        Ok(smallvec![Value::u64(handle as u64)])
    }};
}

fn from_u64_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            from_u64_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.field_add($structure))?;
        let new_element = element_1.add(element_2);
        let new_handle = store_obj!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_add_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_add_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_sub_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.field_sub($structure))?;
        let new_element = element_1.sub(element_2);
        let new_handle = store_obj!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_sub_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_sub_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_mul_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.field_mul($structure))?;
        let new_element = element_1.mul(element_2);
        let new_handle = store_obj!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_mul_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_mul_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_div_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        if element_2.is_zero() {
            return Ok(smallvec![Value::bool(false), Value::u64(0_u64)]);
        }
        $context.charge($gas_params.field_div($structure))?;
        let new_element = element_1.div(element_2);
        let new_handle = store_obj!($context, new_element);
        Ok(smallvec![Value::bool(true), Value::u64(new_handle as u64)])
    }};
}

fn field_div_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_div_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_neg_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.field_neg($structure))?;
        let new_element = element.neg();
        let new_handle = store_obj!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_neg_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_neg_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_inv_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.field_inv($structure))?;
        match element.inverse() {
            Some(new_element) => {
                let new_handle = store_obj!($context, new_element);
                Ok(smallvec![Value::bool(true), Value::u64(new_handle as u64)])
            },
            None => Ok(smallvec![Value::bool(false), Value::u64(0)]),
        }
    }};
}

fn field_inv_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_inv_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_sqr_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.field_sqr($structure))?;
        let new_element = element.square();
        let new_handle = store_obj!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_sqr_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_sqr_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_zero_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        $context.charge($gas_params.field_zero($structure))?;
        let new_element = <$typ>::zero();
        let new_handle = store_obj!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_zero_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_zero_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_one_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        $context.charge($gas_params.field_one($structure))?;
        let new_element = <$typ>::one();
        let new_handle = store_obj!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_one_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_one_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_is_one_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.field_is_one($structure))?;
        let result = element.is_one();
        Ok(smallvec![Value::bool(result)])
    }};
}

fn field_is_one_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_is_one_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_is_zero_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        let element_ptr = get_obj_pointer!($context, handle);
        let element = element_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.field_is_zero($structure))?;
        let result = element.is_zero();
        Ok(smallvec![Value::bool(result)])
    }};
}

fn field_is_zero_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_field_is_zero_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_eq_internal {
    ($gas_params:ident, $context:ident, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        let element_1_ptr = get_obj_pointer!($context, handle_1);
        let element_1 = element_1_ptr.downcast_ref::<$typ>().unwrap();
        let element_2_ptr = get_obj_pointer!($context, handle_2);
        let element_2 = element_2_ptr.downcast_ref::<$typ>().unwrap();
        $context.charge($gas_params.eq($structure))?;
        let result = element_1 == element_2;
        Ok(smallvec![Value::bool(result)])
    }};
}

fn eq_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    if !context
        .get_feature_flags()
        .is_enabled(FeatureFlag::BLS12_381_STRUCTURES)
    {
        return Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        });
    }
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            abort_unless_feature_enabled!(context, FeatureFlag::BLS12_381_STRUCTURES);
            ark_eq_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

#[cfg(feature = "testing")]
macro_rules! ark_insecure_random_element_internal {
    ($context:expr, $typ:ty) => {{
        let element = <$typ>::rand(&mut test_rng());
        let handle = store_obj!($context, element);
        Ok(NativeResult::ok(InternalGas::zero(), smallvec![
            Value::u64(handle as u64)
        ]))
    }};
}

#[cfg(feature = "testing")]
fn insecure_random_element_internal(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    match structure_from_ty_arg!(context, &ty_args[0]) {
        Some(Structure::BLS12381Fr) => {
            ark_insecure_random_element_internal!(context, ark_bls12_381::Fr)
        },
        _ => unreachable!(),
    }
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "deserialize_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                deserialize_internal,
            ),
        ),
        (
            "eq_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                eq_internal,
            ),
        ),
        (
            "field_add_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_add_internal,
            ),
        ),
        (
            "field_div_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_div_internal,
            ),
        ),
        (
            "field_inv_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_inv_internal,
            ),
        ),
        (
            "field_is_one_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_is_one_internal,
            ),
        ),
        (
            "field_is_zero_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_is_zero_internal,
            ),
        ),
        (
            "field_mul_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_mul_internal,
            ),
        ),
        (
            "field_neg_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_neg_internal,
            ),
        ),
        (
            "field_one_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_one_internal,
            ),
        ),
        (
            "field_sqr_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_sqr_internal,
            ),
        ),
        (
            "field_sub_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_sub_internal,
            ),
        ),
        (
            "field_zero_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                field_zero_internal,
            ),
        ),
        (
            "from_u64_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                from_u64_internal,
            ),
        ),
        (
            "serialize_internal",
            make_safe_native(gas_params, timed_features, features, serialize_internal),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![(
        "insecure_random_element_internal",
        make_test_only_native_from_func(insecure_random_element_internal),
    )]);

    crate::natives::helpers::make_module_natives(natives)
}
