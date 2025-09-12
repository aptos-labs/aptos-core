// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        abort_invariant_violated, AlgebraContext, SerializationFormat, Structure,
        BLS12381_R_SCALAR, BN254_R_SCALAR, E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES,
        MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, store_element, structure_from_ty_arg,
};
use aptos_gas_schedule::{
    gas_feature_versions::RELEASE_V1_16, gas_params::natives::aptos_framework::*,
};
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ec::CurveGroup;
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use num_traits::One;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

pub fn feature_flag_of_serialization_format(
    format_opt: Option<SerializationFormat>,
) -> Option<FeatureFlag> {
    match format_opt {
        Some(SerializationFormat::BLS12381FrLsb)
        | Some(SerializationFormat::BLS12381FrMsb)
        | Some(SerializationFormat::BLS12381Fq12LscLsb)
        | Some(SerializationFormat::BLS12381G1Uncompressed)
        | Some(SerializationFormat::BLS12381G1Compressed)
        | Some(SerializationFormat::BLS12381G2Uncompressed)
        | Some(SerializationFormat::BLS12381G2Compressed)
        | Some(SerializationFormat::BLS12381Gt) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        Some(SerializationFormat::BN254FrLsb)
        | Some(SerializationFormat::BN254FrMsb)
        | Some(SerializationFormat::BN254FqLsb)
        | Some(SerializationFormat::BN254FqMsb)
        | Some(SerializationFormat::BN254Fq12LscLsb)
        | Some(SerializationFormat::BN254G1Uncompressed)
        | Some(SerializationFormat::BN254G1Compressed)
        | Some(SerializationFormat::BN254G2Uncompressed)
        | Some(SerializationFormat::BN254G2Compressed)
        | Some(SerializationFormat::BN254Gt) => Some(FeatureFlag::BN254_STRUCTURES),
        _ => None,
    }
}

macro_rules! abort_unless_serialization_format_enabled {
    ($context:ident, $format_opt:expr) => {
        let flag_opt = feature_flag_of_serialization_format($format_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! format_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ)?;
        SerializationFormat::try_from(type_tag).ok()
    }};
}

macro_rules! serialize_element {
    (
        $context:expr,
        $args:ident,
        $structure_to_match:expr,
        $format_to_match:expr,
        [$(($field_structure:pat, $field_format:pat, $field_ty:ty, $field_serialization_func:ident,$reverse:expr, $field_serialization_gas:expr)),* $(,)?],
        [$(($curve_structure:pat,$curve_format:pat, $curve_ty:ty, $curve_serialization_func:ident, $curve_serialization_gas:expr, $into_affine_gas:expr)),* $(,)?]
    ) => {
        match ($structure_to_match, $format_to_match) {
        $(
          ($field_structure,$field_format) => {
            let handle = safely_pop_arg!($args, u64) as usize;
            safe_borrow_element!($context, handle, $field_ty, element_ptr, element);
            let mut buf = vec![];
            $context.charge($field_serialization_gas)?;
            element
                .$field_serialization_func(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            if $reverse {
                buf.reverse();
            }
            Ok(smallvec![Value::vector_u8(buf)])
          }
        )*
        $(
          ($curve_structure,$curve_format) => {
            let handle = safely_pop_arg!($args, u64) as usize;
            safe_borrow_element!(
                $context,
                handle,
                $curve_ty,
                element_ptr,
                element
            );
            if $context.gas_feature_version() >= RELEASE_V1_16 {
                $context.charge($into_affine_gas)?;
            }
            let element_affine = element.into_affine();
            let mut buf = Vec::new();
            $context.charge($curve_serialization_gas)?;
            element_affine
                .$curve_serialization_func(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            Ok(smallvec![Value::vector_u8(buf)])
          }
        )*
          _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
          })
        }
    };
}

pub fn serialize_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let format_opt = format_from_ty_arg!(context, &ty_args[1]);
    abort_unless_serialization_format_enabled!(context, format_opt);
    if let (Some(structure), Some(format)) = (structure_opt, format_opt) {
        serialize_element!(
            context,
            args,
            structure,
            format,
            [
                (
                    Structure::BLS12381Fr,
                    SerializationFormat::BLS12381FrLsb,
                    ark_bls12_381::Fr,
                    serialize_uncompressed,
                    false,
                    ALGEBRA_ARK_BLS12_381_FR_SERIALIZE
                ),
                (
                    Structure::BLS12381Fr,
                    SerializationFormat::BLS12381FrMsb,
                    ark_bls12_381::Fr,
                    serialize_uncompressed,
                    true,
                    ALGEBRA_ARK_BLS12_381_FR_SERIALIZE
                ),
                (
                    Structure::BLS12381Fq12,
                    SerializationFormat::BLS12381Fq12LscLsb,
                    ark_bls12_381::Fq12,
                    serialize_uncompressed,
                    false,
                    ALGEBRA_ARK_BLS12_381_FQ12_SERIALIZE
                ),
                (
                    Structure::BLS12381Gt,
                    SerializationFormat::BLS12381Gt,
                    ark_bls12_381::Fq12,
                    serialize_uncompressed,
                    false,
                    ALGEBRA_ARK_BLS12_381_FQ12_SERIALIZE
                ),
                (
                    Structure::BN254Fr,
                    SerializationFormat::BN254FrLsb,
                    ark_bn254::Fr,
                    serialize_uncompressed,
                    false,
                    ALGEBRA_ARK_BN254_FR_SERIALIZE
                ),
                (
                    Structure::BN254Fr,
                    SerializationFormat::BN254FrMsb,
                    ark_bn254::Fr,
                    serialize_uncompressed,
                    true,
                    ALGEBRA_ARK_BN254_FR_SERIALIZE
                ),
                (
                    Structure::BN254Fq,
                    SerializationFormat::BN254FqLsb,
                    ark_bn254::Fq,
                    serialize_uncompressed,
                    false,
                    ALGEBRA_ARK_BN254_FQ_SERIALIZE
                ),
                (
                    Structure::BN254Fq,
                    SerializationFormat::BN254FqMsb,
                    ark_bn254::Fq,
                    serialize_uncompressed,
                    true,
                    ALGEBRA_ARK_BN254_FQ_SERIALIZE
                ),
                (
                    Structure::BN254Fq12,
                    SerializationFormat::BN254Fq12LscLsb,
                    ark_bn254::Fq12,
                    serialize_uncompressed,
                    false,
                    ALGEBRA_ARK_BN254_FQ12_SERIALIZE
                ),
                (
                    Structure::BN254Gt,
                    SerializationFormat::BN254Gt,
                    ark_bn254::Fq12,
                    serialize_uncompressed,
                    false,
                    ALGEBRA_ARK_BN254_FQ12_SERIALIZE
                )
            ],
            [
                (
                    Structure::BLS12381G1,
                    SerializationFormat::BLS12381G1Uncompressed,
                    ark_bls12_381::G1Projective,
                    serialize_uncompressed,
                    ALGEBRA_ARK_BLS12_381_G1_AFFINE_SERIALIZE_UNCOMP,
                    ALGEBRA_ARK_BLS12_381_G1_PROJ_TO_AFFINE
                ),
                (
                    Structure::BLS12381G1,
                    SerializationFormat::BLS12381G1Compressed,
                    ark_bls12_381::G1Projective,
                    serialize_compressed,
                    ALGEBRA_ARK_BLS12_381_G1_AFFINE_SERIALIZE_COMP,
                    ALGEBRA_ARK_BLS12_381_G1_PROJ_TO_AFFINE
                ),
                (
                    Structure::BLS12381G2,
                    SerializationFormat::BLS12381G2Uncompressed,
                    ark_bls12_381::G2Projective,
                    serialize_uncompressed,
                    ALGEBRA_ARK_BLS12_381_G2_AFFINE_SERIALIZE_UNCOMP,
                    ALGEBRA_ARK_BLS12_381_G2_PROJ_TO_AFFINE
                ),
                (
                    Structure::BLS12381G2,
                    SerializationFormat::BLS12381G2Compressed,
                    ark_bls12_381::G2Projective,
                    serialize_compressed,
                    ALGEBRA_ARK_BLS12_381_G2_AFFINE_SERIALIZE_COMP,
                    ALGEBRA_ARK_BLS12_381_G2_PROJ_TO_AFFINE
                ),
                (
                    Structure::BN254G1,
                    SerializationFormat::BN254G1Uncompressed,
                    ark_bn254::G1Projective,
                    serialize_uncompressed,
                    ALGEBRA_ARK_BN254_G1_AFFINE_SERIALIZE_UNCOMP,
                    ALGEBRA_ARK_BN254_G1_PROJ_TO_AFFINE
                ),
                (
                    Structure::BN254G1,
                    SerializationFormat::BN254G1Compressed,
                    ark_bn254::G1Projective,
                    serialize_compressed,
                    ALGEBRA_ARK_BN254_G1_AFFINE_SERIALIZE_COMP,
                    ALGEBRA_ARK_BN254_G1_PROJ_TO_AFFINE
                ),
                (
                    Structure::BN254G2,
                    SerializationFormat::BN254G2Uncompressed,
                    ark_bn254::G2Projective,
                    serialize_uncompressed,
                    ALGEBRA_ARK_BN254_G2_AFFINE_SERIALIZE_UNCOMP,
                    ALGEBRA_ARK_BN254_G2_PROJ_TO_AFFINE
                ),
                (
                    Structure::BN254G2,
                    SerializationFormat::BN254G2Compressed,
                    ark_bn254::G2Projective,
                    serialize_compressed,
                    ALGEBRA_ARK_BN254_G2_AFFINE_SERIALIZE_COMP,
                    ALGEBRA_ARK_BN254_G2_PROJ_TO_AFFINE
                ),
            ]
        )
    } else {
        Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        })
    }
}

/// Macros that implements `deserialize_internal()` using arkworks libraries.
macro_rules! ark_deserialize_internal {
    ($context:expr, $bytes:expr, $ark_typ:ty, $ark_deser_func:ident, $gas:expr) => {{
        $context.charge($gas)?;
        match <$ark_typ>::$ark_deser_func($bytes) {
            Ok(element) => {
                let handle = store_element!($context, element)?;
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            },
            Err(ark_serialize::SerializationError::InvalidData)
            | Err(ark_serialize::SerializationError::UnexpectedFlags) => {
                Ok(smallvec![Value::bool(false), Value::u64(0)])
            },
            _ => Err(SafeNativeError::InvariantViolation(
                abort_invariant_violated(),
            )),
        }
    }};
}

macro_rules! ark_ec_point_deserialize_internal {
    ($context:expr, $bytes:expr, $typ:ty, $deser_func:ident, $gas:expr) => {{
        $context.charge($gas)?;
        match <$typ>::$deser_func($bytes) {
            Ok(element) => {
                let element_proj = ark_ec::short_weierstrass::Projective::from(element);
                let handle = store_element!($context, element_proj)?;
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            },
            Err(ark_serialize::SerializationError::InvalidData)
            | Err(ark_serialize::SerializationError::UnexpectedFlags) => {
                Ok(smallvec![Value::bool(false), Value::u64(0)])
            },
            _ => Err(SafeNativeError::InvariantViolation(
                abort_invariant_violated(),
            )),
        }
    }};
}

pub fn deserialize_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let format_opt = format_from_ty_arg!(context, &ty_args[1]);
    abort_unless_serialization_format_enabled!(context, format_opt);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes = vector_ref.as_bytes_ref();
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
            // Valid BLS12381FrLsb serialization should be 32-byte.
            // NOTE: Arkworks deserialization cost grows as the input size grows.
            // So exit early if the size is incorrect, for gas safety. (Also applied to other cases across this file.)
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bls12_381::Fr,
                deserialize_uncompressed,
                ALGEBRA_ARK_BLS12_381_FR_DESER
            )
        },
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb)) => {
            // Valid BLS12381FrMsb serialization should be 32-byte.
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            let mut bytes_copy: Vec<u8> = bytes.to_vec();
            bytes_copy.reverse();
            let bytes = bytes_copy.as_slice();
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bls12_381::Fr,
                deserialize_uncompressed,
                ALGEBRA_ARK_BLS12_381_FR_DESER
            )
        },
        (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb)) => {
            // Valid BLS12381Fq12LscLsb serialization should be 576-byte.
            if bytes.len() != 576 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bls12_381::Fq12,
                deserialize_uncompressed,
                ALGEBRA_ARK_BLS12_381_FQ12_DESER
            )
        },
        (Some(Structure::BLS12381G1), Some(SerializationFormat::BLS12381G1Uncompressed)) => {
            // Valid BLS12381G1AffineUncompressed serialization should be 96-byte.
            if bytes.len() != 96 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                context,
                bytes,
                ark_bls12_381::G1Affine,
                deserialize_uncompressed,
                ALGEBRA_ARK_BLS12_381_G1_AFFINE_DESER_UNCOMP
            )
        },
        (Some(Structure::BLS12381G1), Some(SerializationFormat::BLS12381G1Compressed)) => {
            // Valid BLS12381G1AffineCompressed serialization should be 48-byte.
            if bytes.len() != 48 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                context,
                bytes,
                ark_bls12_381::G1Affine,
                deserialize_compressed,
                ALGEBRA_ARK_BLS12_381_G1_AFFINE_DESER_COMP
            )
        },
        (Some(Structure::BLS12381G2), Some(SerializationFormat::BLS12381G2Uncompressed)) => {
            // Valid BLS12381G2AffineUncompressed serialization should be 192-byte.
            if bytes.len() != 192 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                context,
                bytes,
                ark_bls12_381::G2Affine,
                deserialize_uncompressed,
                ALGEBRA_ARK_BLS12_381_G2_AFFINE_DESER_UNCOMP
            )
        },
        (Some(Structure::BLS12381G2), Some(SerializationFormat::BLS12381G2Compressed)) => {
            // Valid BLS12381G2AffineCompressed serialization should be 96-byte.
            if bytes.len() != 96 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                context,
                bytes,
                ark_bls12_381::G2Affine,
                deserialize_compressed,
                ALGEBRA_ARK_BLS12_381_G2_AFFINE_DESER_COMP
            )
        },
        (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            // Valid BLS12381Gt serialization should be 576-byte.
            if bytes.len() != 576 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            context.charge(ALGEBRA_ARK_BLS12_381_FQ12_DESER)?;
            match <ark_bls12_381::Fq12>::deserialize_uncompressed(bytes) {
                Ok(element) => {
                    context.charge(
                        ALGEBRA_ARK_BLS12_381_FQ12_POW_U256 + ALGEBRA_ARK_BLS12_381_FQ12_EQ,
                    )?;
                    if element.pow(BLS12381_R_SCALAR.0) == ark_bls12_381::Fq12::one() {
                        let handle = store_element!(context, element)?;
                        Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
                    } else {
                        Ok(smallvec![Value::bool(false), Value::u64(0)])
                    }
                },
                _ => Ok(smallvec![Value::bool(false), Value::u64(0)]),
            }
        },
        (Some(Structure::BN254Fr), Some(SerializationFormat::BN254FrLsb)) => {
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bn254::Fr,
                deserialize_uncompressed,
                ALGEBRA_ARK_BN254_FR_DESER
            )
        },
        (Some(Structure::BN254Fr), Some(SerializationFormat::BN254FrMsb)) => {
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            let mut bytes_copy: Vec<u8> = bytes.to_vec();
            bytes_copy.reverse();
            let bytes = bytes_copy.as_slice();
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bn254::Fr,
                deserialize_uncompressed,
                ALGEBRA_ARK_BN254_FR_DESER
            )
        },
        (Some(Structure::BN254Fq), Some(SerializationFormat::BN254FqLsb)) => {
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bn254::Fq,
                deserialize_uncompressed,
                ALGEBRA_ARK_BN254_FQ_DESER
            )
        },
        (Some(Structure::BN254Fq), Some(SerializationFormat::BN254FqMsb)) => {
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            let mut bytes_copy: Vec<u8> = bytes.to_vec();
            bytes_copy.reverse();
            let bytes = bytes_copy.as_slice();
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bn254::Fq,
                deserialize_uncompressed,
                ALGEBRA_ARK_BN254_FQ_DESER
            )
        },
        (Some(Structure::BN254Fq12), Some(SerializationFormat::BN254Fq12LscLsb)) => {
            // Valid BN254Fq12LscLsb serialization should be 32*12 = 64-byte.
            if bytes.len() != 384 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bn254::Fq12,
                deserialize_uncompressed,
                ALGEBRA_ARK_BN254_FQ12_DESER
            )
        },
        (Some(Structure::BN254G1), Some(SerializationFormat::BN254G1Uncompressed)) => {
            // Valid BN254G1AffineUncompressed serialization should be 64-byte.
            if bytes.len() != 64 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                context,
                bytes,
                ark_bn254::G1Affine,
                deserialize_uncompressed,
                ALGEBRA_ARK_BN254_G1_AFFINE_DESER_UNCOMP
            )
        },
        (Some(Structure::BN254G1), Some(SerializationFormat::BN254G1Compressed)) => {
            // Valid BN254G1AffineCompressed serialization should be 32-byte.
            if bytes.len() != 32 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                context,
                bytes,
                ark_bn254::G1Affine,
                deserialize_compressed,
                ALGEBRA_ARK_BN254_G1_AFFINE_DESER_COMP
            )
        },
        (Some(Structure::BN254G2), Some(SerializationFormat::BN254G2Uncompressed)) => {
            // Valid BN254G2AffineUncompressed serialization should be 128-byte.
            if bytes.len() != 128 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                context,
                bytes,
                ark_bn254::G2Affine,
                deserialize_uncompressed,
                ALGEBRA_ARK_BN254_G2_AFFINE_DESER_UNCOMP
            )
        },
        (Some(Structure::BN254G2), Some(SerializationFormat::BN254G2Compressed)) => {
            // Valid BN254G2AffineCompressed serialization should be 64-byte.
            if bytes.len() != 64 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_ec_point_deserialize_internal!(
                context,
                bytes,
                ark_bn254::G2Affine,
                deserialize_compressed,
                ALGEBRA_ARK_BN254_G2_AFFINE_DESER_COMP
            )
        },
        (Some(Structure::BN254Gt), Some(SerializationFormat::BN254Gt)) => {
            // Valid BN254Gt serialization should be 32*12=384-byte.
            if bytes.len() != 384 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            context.charge(ALGEBRA_ARK_BN254_FQ12_DESER)?;
            match <ark_bn254::Fq12>::deserialize_uncompressed(bytes) {
                Ok(element) => {
                    context.charge(ALGEBRA_ARK_BN254_FQ12_POW_U256 + ALGEBRA_ARK_BN254_FQ12_EQ)?;
                    if element.pow(BN254_R_SCALAR.0) == ark_bn254::Fq12::one() {
                        let handle = store_element!(context, element)?;
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
