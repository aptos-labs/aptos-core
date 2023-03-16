// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::{
        cryptography::algebra::gas::GasParameters,
        helpers::{
            make_safe_native, make_test_only_native_from_func, SafeNativeContext, SafeNativeError,
            SafeNativeResult,
        },
    },
    safely_pop_arg,
};
use aptos_types::on_chain_config::{FeatureFlag, Features, TimedFeatures};
use ark_ec::{
    hashing::HashToCurve, pairing::Pairing, short_weierstrass::Projective, CurveGroup, Group,
};
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
#[cfg(feature = "testing")]
use ark_std::{test_rng, UniformRand};
use better_any::{Tid, TidAble};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    gas_algebra::{InternalGas, NumArgs},
    language_storage::TypeTag,
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    values::{Value, VectorRef},
};
use num_traits::{One, Zero};
use once_cell::sync::Lazy;
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

/// Equivalent to `std::error::invalid_argument(0)` in Move.
const MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING: u64 = 0x01_0000;

/// Equivalent to `std::error::not_implemented(0)` in Move.
const MOVE_ABORT_CODE_NOT_IMPLEMENTED: u64 = 0x0C_0000;

/// This encodes an algebraic structure defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12381Fq12,
    BLS12381G1Affine,
    BLS12381G2Affine,
    BLS12381Gt,
    BLS12381Fr,
}

impl TryFrom<TypeTag> for Structure {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            "0x1::algebra_bls12381::Fr" => Ok(Structure::BLS12381Fr),
            "0x1::algebra_bls12381::Fq12" => Ok(Structure::BLS12381Fq12),
            "0x1::algebra_bls12381::G1Affine" => Ok(Structure::BLS12381G1Affine),
            "0x1::algebra_bls12381::G2Affine" => Ok(Structure::BLS12381G2Affine),
            "0x1::algebra_bls12381::Gt" => Ok(Structure::BLS12381Gt),
            _ => Err(()),
        }
    }
}

/// This encodes a supported serialization format defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum SerializationFormat {
    BLS12381Fq12LscLsb,
    BLS12381G1AffineCompressed,
    BLS12381G1AffineUncompressed,
    BLS12381G2AffineCompressed,
    BLS12381G2AffineUncompressed,
    BLS12381Gt,
    BLS12381FrLsb,
    BLS12381FrMsb,
}

impl TryFrom<TypeTag> for SerializationFormat {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            "0x1::algebra_bls12381::Fq12FormatLscLsb" => {
                Ok(SerializationFormat::BLS12381Fq12LscLsb)
            },
            "0x1::algebra_bls12381::G1AffineFormatUncompressed" => {
                Ok(SerializationFormat::BLS12381G1AffineUncompressed)
            },
            "0x1::algebra_bls12381::G1AffineFormatCompressed" => {
                Ok(SerializationFormat::BLS12381G1AffineCompressed)
            },
            "0x1::algebra_bls12381::G2AffineFormatUncompressed" => {
                Ok(SerializationFormat::BLS12381G2AffineUncompressed)
            },
            "0x1::algebra_bls12381::G2AffineFormatCompressed" => {
                Ok(SerializationFormat::BLS12381G2AffineCompressed)
            },
            "0x1::algebra_bls12381::GtFormat" => Ok(SerializationFormat::BLS12381Gt),
            "0x1::algebra_bls12381::FrFormatLsb" => Ok(SerializationFormat::BLS12381FrLsb),
            "0x1::algebra_bls12381::FrFormatMsb" => Ok(SerializationFormat::BLS12381FrMsb),
            _ => Err(()),
        }
    }
}

/// This encodes a supported hash-to-structure suite defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum HashToStructureSuite {
    Bls12381g1XmdSha256SswuRo,
    Bls12381g2XmdSha256SswuRo,
}

impl TryFrom<TypeTag> for HashToStructureSuite {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            "0x1::algebra_bls12381::H2SSuiteBls12381g1XmdSha256SswuRo" => {
                Ok(HashToStructureSuite::Bls12381g1XmdSha256SswuRo)
            },
            "0x1::algebra_bls12381::H2SSuiteBls12381g2XmdSha256SswuRo" => {
                Ok(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)
            },
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
        Structure::try_from(type_tag).ok()
    }};
}

macro_rules! format_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        SerializationFormat::try_from(type_tag).ok()
    }};
}

macro_rules! suite_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        HashToStructureSuite::try_from(type_tag).ok()
    }};
}

macro_rules! store_element {
    ($context:expr, $obj:expr) => {{
        let target_vec = &mut $context.extensions_mut().get_mut::<AlgebraContext>().objs;
        let ret = target_vec.len();
        target_vec.push(Rc::new($obj));
        ret
    }};
}

macro_rules! abort_unless_feature_flag_enabled {
    ($context:ident, $flag_opt:expr) => {
        match $flag_opt {
            Some(flag) if $context.get_feature_flags().is_enabled(flag) => {
                // Continue.
            },
            _ => {
                return Err(SafeNativeError::Abort {
                    abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
                });
            },
        }
    };
}

fn feature_flag_of_single_type_basic_op(structure_opt: Option<Structure>) -> Option<FeatureFlag> {
    match structure_opt {
        Some(Structure::BLS12381Fr)
        | Some(Structure::BLS12381Fq12)
        | Some(Structure::BLS12381G1Affine)
        | Some(Structure::BLS12381G2Affine)
        | Some(Structure::BLS12381Gt) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        _ => None,
    }
}

fn feature_flag_of_serialization_format(
    structure_opt: Option<Structure>,
    format_opt: Option<SerializationFormat>,
) -> Option<FeatureFlag> {
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb))
        | (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb))
        | (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb))
        | (
            Some(Structure::BLS12381G1Affine),
            Some(SerializationFormat::BLS12381G1AffineUncompressed),
        )
        | (
            Some(Structure::BLS12381G1Affine),
            Some(SerializationFormat::BLS12381G1AffineCompressed),
        )
        | (
            Some(Structure::BLS12381G2Affine),
            Some(SerializationFormat::BLS12381G2AffineUncompressed),
        )
        | (
            Some(Structure::BLS12381G2Affine),
            Some(SerializationFormat::BLS12381G2AffineCompressed),
        )
        | (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        _ => None,
    }
}

fn feature_flag_of_hash_to_structure(
    structure_opt: Option<Structure>,
    suite_opt: Option<HashToStructureSuite>,
) -> Option<FeatureFlag> {
    match (structure_opt, suite_opt) {
        (
            Some(Structure::BLS12381G1Affine),
            Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo),
        )
        | (
            Some(Structure::BLS12381G2Affine),
            Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo),
        ) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        _ => None,
    }
}

fn feature_flag_of_group_scalar_mul(
    group_opt: Option<Structure>,
    scalar_field_opt: Option<Structure>,
) -> Option<FeatureFlag> {
    match (group_opt, scalar_field_opt) {
        (Some(Structure::BLS12381G1Affine), Some(Structure::BLS12381Fr))
        | (Some(Structure::BLS12381G2Affine), Some(Structure::BLS12381Fr))
        | (Some(Structure::BLS12381Gt), Some(Structure::BLS12381Fr)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        _ => None,
    }
}

fn feature_flag_of_pairing(
    g1_opt: Option<Structure>,
    g2_opt: Option<Structure>,
    gt_opt: Option<Structure>,
) -> Option<FeatureFlag> {
    match (g1_opt, g2_opt, gt_opt) {
        (
            Some(Structure::BLS12381G1Affine),
            Some(Structure::BLS12381G2Affine),
            Some(Structure::BLS12381Gt),
        ) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        _ => None,
    }
}

fn feature_flag_of_casting(
    super_opt: Option<Structure>,
    sub_opt: Option<Structure>,
) -> Option<FeatureFlag> {
    match (super_opt, sub_opt) {
        (Some(Structure::BLS12381Fq12), Some(Structure::BLS12381Gt)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        _ => None,
    }
}

macro_rules! abort_unless_single_type_basic_op_enabled {
    ($context:ident, $structure_opt:expr) => {
        let flag_opt = feature_flag_of_single_type_basic_op($structure_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! abort_unless_hash_to_structure_enabled {
    ($context:ident, $structure_opt:expr, $suite_opt:expr) => {
        let flag_opt = feature_flag_of_hash_to_structure($structure_opt, $suite_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! abort_unless_serialization_format_enabled {
    ($context:ident, $structure_opt:expr, $format_opt:expr) => {
        let flag_opt = feature_flag_of_serialization_format($structure_opt, $format_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! abort_unless_group_scalar_mul_enabled {
    ($context:ident, $group_opt:expr, $scalar_field_opt:expr) => {
        let flag_opt = feature_flag_of_group_scalar_mul($group_opt, $scalar_field_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! abort_unless_pairing_enabled {
    ($context:ident, $g1_opt:expr, $g2_opt:expr, $gt_opt:expr) => {
        let flag_opt = feature_flag_of_pairing($g1_opt, $g2_opt, $gt_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! abort_unless_casting_enabled {
    ($context:ident, $super_opt:expr, $sub_opt:expr) => {
        let flag_opt = feature_flag_of_casting($super_opt, $sub_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

fn abort_invariant_violated() -> PartialVMError {
    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
}

/// Try getting a pointer to the `handle`-th elements in `context` and assign it to a local variable `ptr_out`.
/// Then try casting it to a reference of `typ` and assign it in a local variable `ref_out`.
/// Abort the VM execution with invariant violation if anything above fails.
macro_rules! safe_borrow_element {
    ($context:expr, $handle:expr, $typ:ty, $ptr_out:ident, $ref_out:ident) => {
        let $ptr_out = $context
            .extensions()
            .get::<AlgebraContext>()
            .objs
            .get($handle)
            .ok_or_else(abort_invariant_violated)?
            .clone();
        let $ref_out = $ptr_out
            .downcast_ref::<$typ>()
            .ok_or_else(abort_invariant_violated)?;
    };
}

macro_rules! ark_serialize_internal {
    (
        $gas_params:expr,
        $context:expr,
        $args:ident,
        $structure:expr,
        $format:expr,
        $ark_type:ty,
        $ark_ser_func:ident
    ) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $ark_type, element_ptr, element);
        let mut buf = vec![];
        $context.charge($gas_params.serialize($structure, $format))?;
        element.$ark_ser_func(&mut buf).unwrap();
        buf
    }};
}

macro_rules! ark_ec_point_serialize_internal {
    (
        $gas_params:expr,
        $context:expr,
        $args:ident,
        $structure:expr,
        $format:expr,
        $ark_type:ty,
        $ark_ser_func:ident
    ) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $ark_type, element_ptr, element);
        let element_affine = element.into_affine();
        let mut buf = Vec::new();
        $context.charge($gas_params.serialize($structure, $format))?;
        element_affine.$ark_ser_func(&mut buf).unwrap();
        buf
    }};
}

fn serialize_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let format_opt = format_from_ty_arg!(context, &ty_args[1]);
    abort_unless_serialization_format_enabled!(context, structure_opt, format_opt);
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
            let buf = ark_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                SerializationFormat::BLS12381FrLsb,
                ark_bls12_381::Fr,
                serialize_uncompressed
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb)) => {
            let mut buf = ark_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fr,
                SerializationFormat::BLS12381FrMsb,
                ark_bls12_381::Fr,
                serialize_uncompressed
            );
            buf.reverse();
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb)) => {
            let buf = ark_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Fq12,
                SerializationFormat::BLS12381Fq12LscLsb,
                ark_bls12_381::Fq12,
                serialize_uncompressed
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (
            Some(Structure::BLS12381G1Affine),
            Some(SerializationFormat::BLS12381G1AffineUncompressed),
        ) => {
            let buf = ark_ec_point_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381G1Affine,
                SerializationFormat::BLS12381G1AffineUncompressed,
                ark_bls12_381::G1Projective,
                serialize_uncompressed
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (
            Some(Structure::BLS12381G1Affine),
            Some(SerializationFormat::BLS12381G1AffineCompressed),
        ) => {
            let buf = ark_ec_point_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381G1Affine,
                SerializationFormat::BLS12381G1AffineCompressed,
                ark_bls12_381::G1Projective,
                serialize_compressed
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (
            Some(Structure::BLS12381G2Affine),
            Some(SerializationFormat::BLS12381G2AffineUncompressed),
        ) => {
            let buf = ark_ec_point_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381G2Affine,
                SerializationFormat::BLS12381G2AffineUncompressed,
                ark_bls12_381::G2Projective,
                serialize_uncompressed
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (
            Some(Structure::BLS12381G2Affine),
            Some(SerializationFormat::BLS12381G2AffineCompressed),
        ) => {
            let buf = ark_ec_point_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381G2Affine,
                SerializationFormat::BLS12381G2AffineCompressed,
                ark_bls12_381::G2Projective,
                serialize_compressed
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            let buf = ark_serialize_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381Gt,
                SerializationFormat::BLS12381Gt,
                ark_bls12_381::Fq12,
                serialize_uncompressed
            );
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
                let handle = store_element!($context, element);
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            },
            _ => Ok(smallvec![Value::bool(false), Value::u64(0)]),
        }
    }};
}

macro_rules! ark_ec_point_deserialize_internal {
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
                let element_proj = Projective::from(element);
                let handle = store_element!($context, element_proj);
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
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let format_opt = format_from_ty_arg!(context, &ty_args[1]);
    abort_unless_serialization_format_enabled!(context, structure_opt, format_opt);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let bytes = bytes_ref.as_slice();
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
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
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb)) => {
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
        (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb)) => {
            if bytes.len() != 576 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                gas_params,
                context,
                bytes,
                Structure::BLS12381Fq12,
                SerializationFormat::BLS12381Fq12LscLsb,
                ark_bls12_381::Fq12,
                deserialize_uncompressed
            )
        },
        (
            Some(Structure::BLS12381G1Affine),
            Some(SerializationFormat::BLS12381G1AffineUncompressed),
        ) => {
            if bytes.len() != 96 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                gas_params,
                context,
                bytes,
                Structure::BLS12381G1Affine,
                SerializationFormat::BLS12381G1AffineUncompressed,
                ark_bls12_381::G1Affine,
                deserialize_uncompressed
            )
        },
        (
            Some(Structure::BLS12381G1Affine),
            Some(SerializationFormat::BLS12381G1AffineCompressed),
        ) => {
            if bytes.len() != 48 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                gas_params,
                context,
                bytes,
                Structure::BLS12381G1Affine,
                SerializationFormat::BLS12381G1AffineCompressed,
                ark_bls12_381::G1Affine,
                deserialize_compressed
            )
        },
        (
            Some(Structure::BLS12381G2Affine),
            Some(SerializationFormat::BLS12381G2AffineUncompressed),
        ) => {
            if bytes.len() != 192 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                gas_params,
                context,
                bytes,
                Structure::BLS12381G2Affine,
                SerializationFormat::BLS12381G2AffineUncompressed,
                ark_bls12_381::G2Affine,
                deserialize_uncompressed
            )
        },
        (
            Some(Structure::BLS12381G2Affine),
            Some(SerializationFormat::BLS12381G2AffineCompressed),
        ) => {
            if bytes.len() != 96 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                gas_params,
                context,
                bytes,
                Structure::BLS12381G2Affine,
                SerializationFormat::BLS12381G2AffineCompressed,
                ark_bls12_381::G2Affine,
                deserialize_compressed
            )
        },
        (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            if bytes.len() != 576 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            context.charge(
                gas_params.deserialize(Structure::BLS12381Gt, SerializationFormat::BLS12381Gt),
            )?;
            match <ark_bls12_381::Fq12>::deserialize_uncompressed(bytes) {
                Ok(element) => {
                    if element.pow(BLS12381_R_SCALAR.0) == ark_bls12_381::Fq12::one() {
                        let handle = store_element!(context, element);
                        Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
                    } else {
                        Ok(smallvec![Value::bool(false), Value::u64(0)])
                    }
                },
                _ => Ok(smallvec![Value::bool(false), Value::u64(0)]),
            }
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! from_u64_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let value = safely_pop_arg!($args, u64);
        $context.charge($gas_params.from_u64($structure))?;
        let element = <$typ>::from(value as u64);
        let handle = store_element!($context, element);
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
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => from_u64_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => from_u64_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $typ, element_2_ptr, element_2);
        $context.charge($gas_params.field_add($structure))?;
        let new_element = element_1.add(element_2);
        let new_handle = store_element!($context, new_element);
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
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_field_add_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_field_add_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_sub_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $typ, element_2_ptr, element_2);
        $context.charge($gas_params.field_sub($structure))?;
        let new_element = element_1.sub(element_2);
        let new_handle = store_element!($context, new_element);
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
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_field_sub_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_field_sub_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_mul_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $typ, element_2_ptr, element_2);
        $context.charge($gas_params.field_mul($structure))?;
        let new_element = element_1.mul(element_2);
        let new_handle = store_element!($context, new_element);
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
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_field_mul_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_field_mul_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_div_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $typ, element_2_ptr, element_2);
        if element_2.is_zero() {
            return Ok(smallvec![Value::bool(false), Value::u64(0_u64)]);
        }
        $context.charge($gas_params.field_div($structure))?;
        let new_element = element_1.div(element_2);
        let new_handle = store_element!($context, new_element);
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
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_field_div_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_field_div_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_neg_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $typ, element_ptr, element);
        $context.charge($gas_params.field_neg($structure))?;
        let new_element = element.neg();
        let new_handle = store_element!($context, new_element);
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
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_neg_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_neg_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_inv_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $typ, element_ptr, element);
        $context.charge($gas_params.field_inv($structure))?;
        match element.inverse() {
            Some(new_element) => {
                let new_handle = store_element!($context, new_element);
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
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_field_inv_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_field_inv_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_sqr_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $typ, element_ptr, element);
        $context.charge($gas_params.field_sqr($structure))?;
        let new_element = element.square();
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_sqr_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_field_sqr_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_field_sqr_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_zero_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        $context.charge($gas_params.field_zero($structure))?;
        let new_element = <$typ>::zero();
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_zero_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_field_zero_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_field_zero_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_field_one_internal {
    ($gas_params:ident, $context:expr, $args:ident, $structure:expr, $typ:ty) => {{
        $context.charge($gas_params.field_one($structure))?;
        let new_element = <$typ>::one();
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn field_one_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_field_one_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_field_one_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_eq_internal {
    ($gas_params:ident, $context:ident, $args:ident, $structure:expr, $typ:ty) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $typ, element_2_ptr, element_2);
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
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_eq_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fr,
            ark_bls12_381::Fr
        ),
        Some(Structure::BLS12381Fq12) => ark_eq_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Fq12,
            ark_bls12_381::Fq12
        ),
        Some(Structure::BLS12381G1Affine) => ark_eq_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G1Affine,
            ark_bls12_381::G1Projective
        ),
        Some(Structure::BLS12381G2Affine) => ark_eq_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G2Affine,
            ark_bls12_381::G2Projective
        ),
        Some(Structure::BLS12381Gt) => ark_eq_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Gt,
            ark_bls12_381::Fq12
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_group_identity_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $typ:ty, $func:ident) => {{
        $context.charge($gas_params.group_identity($structure))?;
        let element = <$typ>::$func();
        let handle = store_element!($context, element);
        Ok(smallvec![Value::u64(handle as u64)])
    }};
}

fn group_identity_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381G1Affine) => ark_group_identity_internal!(
            gas_params,
            context,
            Structure::BLS12381G1Affine,
            ark_bls12_381::G1Projective,
            zero
        ),
        Some(Structure::BLS12381G2Affine) => ark_group_identity_internal!(
            gas_params,
            context,
            Structure::BLS12381G2Affine,
            ark_bls12_381::G2Projective,
            zero
        ),
        Some(Structure::BLS12381Gt) => ark_group_identity_internal!(
            gas_params,
            context,
            Structure::BLS12381Gt,
            ark_bls12_381::Fq12,
            one
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_multi_scalar_mul_internal {
    (
        $gas_params:expr,
        $context:expr,
        $args:ident,
        $structure:expr,
        $element_typ:ty,
        $scalar_typ:ty
    ) => {{
        let scalar_handles = safely_pop_arg!($args, Vec<u64>);
        let element_handles = safely_pop_arg!($args, Vec<u64>);
        let num_elements = element_handles.len();
        let num_scalars = scalar_handles.len();
        if num_elements != num_scalars {
            return Err(SafeNativeError::Abort {
                abort_code: MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING,
            });
        }
        let mut bases = Vec::with_capacity(num_elements);
        for handle in element_handles {
            safe_borrow_element!(
                $context,
                handle as usize,
                $element_typ,
                element_ptr,
                element
            );
            bases.push(element.into_affine());
        }
        let mut scalars = Vec::with_capacity(num_scalars);
        for handle in scalar_handles {
            safe_borrow_element!($context, handle as usize, $scalar_typ, scalar_ptr, scalar);
            scalars.push(scalar.clone());
        }
        $context.charge($gas_params.group_multi_scalar_mul($structure, num_elements))?;
        let new_element: $element_typ =
            ark_ec::VariableBaseMSM::msm(bases.as_slice(), scalars.as_slice()).unwrap();
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn group_multi_scalar_mul_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let group_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let scalar_opt = structure_from_ty_arg!(context, &ty_args[1]);
    abort_unless_group_scalar_mul_enabled!(context, group_opt, scalar_opt);
    match (group_opt, scalar_opt) {
        (Some(Structure::BLS12381G1Affine), Some(Structure::BLS12381Fr)) => {
            ark_multi_scalar_mul_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381G1Affine,
                ark_bls12_381::G1Projective,
                ark_bls12_381::Fr
            )
        },
        (Some(Structure::BLS12381G2Affine), Some(Structure::BLS12381Fr)) => {
            ark_multi_scalar_mul_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381G2Affine,
                ark_bls12_381::G2Projective,
                ark_bls12_381::Fr
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

static BLS12381_GT_GENERATOR: Lazy<ark_bls12_381::Fq12> = Lazy::new(|| {
    let buf = hex::decode("b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f").unwrap();
    ark_bls12_381::Fq12::deserialize_uncompressed(buf.as_slice()).unwrap()
});

static BLS12381_R_LENDIAN: Lazy<Vec<u8>> = Lazy::new(|| {
    hex::decode("01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73").unwrap()
});

static BLS12381_R_SCALAR: Lazy<ark_ff::BigInteger256> = Lazy::new(|| {
    ark_ff::BigInteger256::deserialize_uncompressed(BLS12381_R_LENDIAN.as_slice()).unwrap()
});

macro_rules! ark_group_generator_internal {
    ($gas_params:expr, $context:expr, $structure:expr, $typ:ty) => {{
        $context.charge($gas_params.group_generator($structure))?;
        let element = <$typ>::generator();
        let handle = store_element!($context, element);
        Ok(smallvec![Value::u64(handle as u64)])
    }};
}

fn group_generator_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381G1Affine) => ark_group_generator_internal!(
            gas_params,
            context,
            Structure::BLS12381G1Affine,
            ark_bls12_381::G1Projective
        ),
        Some(Structure::BLS12381G2Affine) => ark_group_generator_internal!(
            gas_params,
            context,
            Structure::BLS12381G2Affine,
            ark_bls12_381::G2Projective
        ),
        Some(Structure::BLS12381Gt) => {
            context.charge(gas_params.group_generator(Structure::BLS12381Gt))?;
            let element = BLS12381_GT_GENERATOR.add(ark_bls12_381::Fq12::zero());
            let handle = store_element!(context, element);
            Ok(smallvec![Value::u64(handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

fn group_order_internal(
    _gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381G1Affine)
        | Some(Structure::BLS12381G2Affine)
        | Some(Structure::BLS12381Gt) => {
            Ok(smallvec![Value::vector_u8(BLS12381_R_LENDIAN.clone())])
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
        let handle = store_element!($context, element);
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
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    match structure_opt {
        Some(Structure::BLS12381Fr) => {
            ark_insecure_random_element_internal!(context, ark_bls12_381::Fr)
        },
        Some(Structure::BLS12381Fq12) => {
            ark_insecure_random_element_internal!(context, ark_bls12_381::Fq12)
        },
        Some(Structure::BLS12381G1Affine) => {
            ark_insecure_random_element_internal!(context, ark_bls12_381::G1Projective)
        },
        Some(Structure::BLS12381G2Affine) => {
            ark_insecure_random_element_internal!(context, ark_bls12_381::G2Projective)
        },
        Some(Structure::BLS12381Gt) => {
            let k = ark_bls12_381::Fr::rand(&mut test_rng());
            let k_bigint: ark_ff::BigInteger256 = k.into();
            let element = BLS12381_GT_GENERATOR.pow(k_bigint);
            let handle = store_element!(context, element);
            Ok(NativeResult::ok(InternalGas::zero(), smallvec![
                Value::u64(handle as u64)
            ]))
        },
        _ => unreachable!(),
    }
}

macro_rules! ark_group_add_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $typ, element_2_ptr, element_2);
        $context.charge($gas_params.group_add($structure))?;

        let new_element = element_1.$op(element_2);
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn group_add_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381G1Affine) => ark_group_add_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G1Affine,
            ark_bls12_381::G1Projective,
            add
        ),
        Some(Structure::BLS12381G2Affine) => ark_group_add_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G2Affine,
            ark_bls12_381::G2Projective,
            add
        ),
        Some(Structure::BLS12381Gt) => ark_group_add_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Gt,
            ark_bls12_381::Fq12,
            mul
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_group_sub_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $typ, element_2_ptr, element_2);
        $context.charge($gas_params.group_sub($structure))?;
        let new_element = element_1.$op(element_2);
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn group_sub_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381G1Affine) => ark_group_sub_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G1Affine,
            ark_bls12_381::G1Projective,
            sub
        ),
        Some(Structure::BLS12381G2Affine) => ark_group_sub_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G2Affine,
            ark_bls12_381::G2Projective,
            sub
        ),
        Some(Structure::BLS12381Gt) => ark_group_sub_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Gt,
            ark_bls12_381::Fq12,
            div
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_group_scalar_mul_internal {
    (
        $gas_params:expr,
        $context:expr,
        $args:ident,
        $group_structure:expr,
        $scalar_structure:expr,
        $group_typ:ty,
        $scalar_typ:ty,
        $op:ident
    ) => {{
        let scalar_handle = safely_pop_arg!($args, u64) as usize;
        let element_handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, element_handle, $group_typ, element_ptr, element);
        safe_borrow_element!($context, scalar_handle, $scalar_typ, scalar_ptr, scalar);
        let scalar_bigint: ark_ff::BigInteger256 = (*scalar).into();
        $context.charge($gas_params.group_scalar_mul($group_structure))?;
        let new_element = element.$op(scalar_bigint);
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn group_scalar_mul_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let group_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let scalar_field_opt = structure_from_ty_arg!(context, &ty_args[1]);
    abort_unless_group_scalar_mul_enabled!(context, group_opt, scalar_field_opt);
    match (group_opt, scalar_field_opt) {
        (Some(Structure::BLS12381G1Affine), Some(Structure::BLS12381Fr)) => {
            ark_group_scalar_mul_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381G1Affine,
                Structure::BLS12381Fr,
                ark_bls12_381::G1Projective,
                ark_bls12_381::Fr,
                mul_bigint
            )
        },
        (Some(Structure::BLS12381G2Affine), Some(Structure::BLS12381Fr)) => {
            ark_group_scalar_mul_internal!(
                gas_params,
                context,
                args,
                Structure::BLS12381G2Affine,
                Structure::BLS12381Fr,
                ark_bls12_381::G2Projective,
                ark_bls12_381::Fr,
                mul_bigint
            )
        },
        (Some(Structure::BLS12381Gt), Some(Structure::BLS12381Fr)) => {
            let scalar_handle = safely_pop_arg!(args, u64) as usize;
            let element_handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(
                context,
                element_handle,
                ark_bls12_381::Fq12,
                element_ptr,
                element
            );
            safe_borrow_element!(
                context,
                scalar_handle,
                ark_bls12_381::Fr,
                scalar_ptr,
                scalar
            );
            let scalar_bigint: ark_ff::BigInteger256 = (*scalar).into();
            context.charge(gas_params.group_scalar_mul(Structure::BLS12381Gt))?;
            let new_element = element.pow(scalar_bigint);
            let new_handle = store_element!(context, new_element);
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_bls12381gx_xmd_sha_256_sswu_ro_internal {
    (
        $gas_params:expr,
        $context:expr,
        $dst:expr,
        $msg:expr,
        $h2s_suite:expr,
        $target_type:ty,
        $config_type:ty
    ) => {{
        $context.charge($gas_params.hash_to($h2s_suite, $dst.len(), $msg.len()))?;
        let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
            ark_ec::models::short_weierstrass::Projective<$config_type>,
            ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
            ark_ec::hashing::curve_maps::wb::WBMap<$config_type>,
        >::new($dst)
        .unwrap();
        let new_element = <$target_type>::from(mapper.hash($msg).unwrap());
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn hash_to_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let suite_opt = suite_from_ty_arg!(context, &ty_args[1]);
    abort_unless_hash_to_structure_enabled!(context, structure_opt, suite_opt);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let msg = bytes_ref.as_slice();
    let tag_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = tag_ref.as_bytes_ref();
    let dst = bytes_ref.as_slice();
    match (structure_opt, suite_opt) {
        (
            Some(Structure::BLS12381G1Affine),
            Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo),
        ) => ark_bls12381gx_xmd_sha_256_sswu_ro_internal!(
            gas_params,
            context,
            dst,
            msg,
            HashToStructureSuite::Bls12381g1XmdSha256SswuRo,
            ark_bls12_381::G1Projective,
            ark_bls12_381::g1::Config
        ),
        (
            Some(Structure::BLS12381G2Affine),
            Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo),
        ) => ark_bls12381gx_xmd_sha_256_sswu_ro_internal!(
            gas_params,
            context,
            dst,
            msg,
            HashToStructureSuite::Bls12381g2XmdSha256SswuRo,
            ark_bls12_381::G2Projective,
            ark_bls12_381::g2::Config
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_group_double_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $typ, element_ptr, element);
        $context.charge($gas_params.group_double($structure))?;
        let new_element = element.$op();
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn group_double_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381G1Affine) => ark_group_double_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G1Affine,
            ark_bls12_381::G1Projective,
            double
        ),
        Some(Structure::BLS12381G2Affine) => ark_group_double_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G2Affine,
            ark_bls12_381::G2Projective,
            double
        ),
        Some(Structure::BLS12381Gt) => ark_group_double_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381Gt,
            ark_bls12_381::Fq12,
            square
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

macro_rules! ark_group_neg_internal {
    ($gas_params:expr, $context:expr, $args:ident, $structure:expr, $typ:ty, $op:ident) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $typ, element_ptr, element);
        $context.charge($gas_params.group_neg($structure))?;
        let new_element = element.$op();
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

fn group_neg_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_single_type_basic_op_enabled!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381G1Affine) => ark_group_neg_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G1Affine,
            ark_bls12_381::G1Projective,
            neg
        ),
        Some(Structure::BLS12381G2Affine) => ark_group_neg_internal!(
            gas_params,
            context,
            args,
            Structure::BLS12381G2Affine,
            ark_bls12_381::G2Projective,
            neg
        ),
        Some(Structure::BLS12381Gt) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bls12_381::Fq12, element_ptr, element);
            context.charge(gas_params.group_neg(Structure::BLS12381Gt))?;
            let new_element = element.inverse().unwrap();
            let new_handle = store_element!(context, new_element);
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

fn multi_pairing_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(3, ty_args.len());
    let g1_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let g2_opt = structure_from_ty_arg!(context, &ty_args[1]);
    let gt_opt = structure_from_ty_arg!(context, &ty_args[2]);
    abort_unless_pairing_enabled!(context, g1_opt, g2_opt, gt_opt);
    match (g1_opt, g2_opt, gt_opt) {
        (
            Some(Structure::BLS12381G1Affine),
            Some(Structure::BLS12381G2Affine),
            Some(Structure::BLS12381Gt),
        ) => {
            let g2_element_handles = safely_pop_arg!(args, Vec<u64>);
            let g1_element_handles = safely_pop_arg!(args, Vec<u64>);
            let num_entries = g1_element_handles.len();
            if num_entries != g2_element_handles.len() {
                return Err(SafeNativeError::Abort {
                    abort_code: MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING,
                });
            }
            context.charge(gas_params.multi_pairing(
                Structure::BLS12381G1Affine,
                Structure::BLS12381G2Affine,
                Structure::BLS12381Gt,
                g1_element_handles.len(),
            ))?;
            let mut g1_elements_affine = Vec::with_capacity(num_entries);
            for handle in g1_element_handles {
                safe_borrow_element!(
                    context,
                    handle as usize,
                    ark_bls12_381::G1Projective,
                    ptr,
                    element
                );
                g1_elements_affine.push(element.into_affine());
            }
            let mut g2_elements_affine = Vec::with_capacity(num_entries);
            for handle in g2_element_handles {
                safe_borrow_element!(
                    context,
                    handle as usize,
                    ark_bls12_381::G2Projective,
                    ptr,
                    element
                );
                g2_elements_affine.push(element.into_affine());
            }
            let new_element =
                ark_bls12_381::Bls12_381::multi_pairing(g1_elements_affine, g2_elements_affine).0;
            let new_handle = store_element!(context, new_element);
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

fn pairing_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(3, ty_args.len());
    let g1_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let g2_opt = structure_from_ty_arg!(context, &ty_args[1]);
    let gt_opt = structure_from_ty_arg!(context, &ty_args[2]);
    abort_unless_pairing_enabled!(context, g1_opt, g2_opt, gt_opt);
    match (g1_opt, g2_opt, gt_opt) {
        (
            Some(Structure::BLS12381G1Affine),
            Some(Structure::BLS12381G2Affine),
            Some(Structure::BLS12381Gt),
        ) => {
            let g2_element_handle = safely_pop_arg!(args, u64) as usize;
            let g1_element_handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(
                context,
                g1_element_handle,
                ark_bls12_381::G1Projective,
                g1_element_ptr,
                g1_element
            );
            let g1_element_affine = g1_element.into_affine();
            safe_borrow_element!(
                context,
                g2_element_handle,
                ark_bls12_381::G2Projective,
                g2_element_ptr,
                g2_element
            );
            let g2_element_affine = g2_element.into_affine();
            context.charge(gas_params.pairing(
                Structure::BLS12381G1Affine,
                Structure::BLS12381G2Affine,
                Structure::BLS12381Gt,
            ))?;
            let new_element =
                ark_bls12_381::Bls12_381::pairing(g1_element_affine, g2_element_affine).0;
            let new_handle = store_element!(context, new_element);
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

fn downcast_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let super_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let sub_opt = structure_from_ty_arg!(context, &ty_args[1]);
    abort_unless_casting_enabled!(context, super_opt, sub_opt);
    match (super_opt, sub_opt) {
        (Some(Structure::BLS12381Fq12), Some(Structure::BLS12381Gt)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bls12_381::Fq12, element_ptr, element);
            context.charge(gas_params.ark_bls12_381_fq12_pow_u256 * NumArgs::one())?;
            if element.pow(BLS12381_R_SCALAR.0) == ark_bls12_381::Fq12::one() {
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            } else {
                Ok(smallvec![Value::bool(false), Value::u64(handle as u64)])
            }
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

fn upcast_internal(
    _gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let sub_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let super_opt = structure_from_ty_arg!(context, &ty_args[1]);
    abort_unless_casting_enabled!(context, super_opt, sub_opt);
    match (sub_opt, super_opt) {
        (Some(Structure::BLS12381Gt), Some(Structure::BLS12381Fq12)) => {
            let handle = safely_pop_arg!(args, u64);
            Ok(smallvec![Value::u64(handle)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
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
            "downcast_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                downcast_internal,
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
            "group_add_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_add_internal,
            ),
        ),
        (
            "group_double_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_double_internal,
            ),
        ),
        (
            "group_generator_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_generator_internal,
            ),
        ),
        (
            "group_identity_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_identity_internal,
            ),
        ),
        (
            "group_multi_scalar_mul_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_multi_scalar_mul_internal,
            ),
        ),
        (
            "group_neg_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_neg_internal,
            ),
        ),
        (
            "group_order_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_order_internal,
            ),
        ),
        (
            "group_scalar_mul_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_scalar_mul_internal,
            ),
        ),
        (
            "group_sub_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                group_sub_internal,
            ),
        ),
        (
            "hash_to_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                hash_to_internal,
            ),
        ),
        (
            "multi_pairing_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                multi_pairing_internal,
            ),
        ),
        (
            "pairing_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                pairing_internal,
            ),
        ),
        (
            "serialize_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                serialize_internal,
            ),
        ),
        (
            "upcast_internal",
            make_safe_native(gas_params, timed_features, features, upcast_internal),
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
