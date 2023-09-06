// Copyright © Aptos Foundation

use crate::{
    abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        abort_invariant_violated, AlgebraContext, SerializationFormat, Structure,
        BLS12381_R_SCALAR, E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES,
        MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, store_element, structure_from_ty_arg,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
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

pub fn serialize_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let format_opt = format_from_ty_arg!(context, &ty_args[1]);
    abort_unless_serialization_format_enabled!(context, format_opt);
    match (structure_opt, format_opt) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bls12_381::Fr, element_ptr, element);
            let mut buf = vec![];
            context.charge(ALGEBRA_ARK_BLS12_381_FR_SERIALIZE)?;
            element
                .serialize_uncompressed(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bls12_381::Fr, element_ptr, element);
            let mut buf = vec![];
            context.charge(ALGEBRA_ARK_BLS12_381_FR_SERIALIZE)?;
            element
                .serialize_uncompressed(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            buf.reverse();
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bls12_381::Fq12, element_ptr, element);
            let mut buf = vec![];
            context.charge(ALGEBRA_ARK_BLS12_381_FQ12_SERIALIZE)?;
            element
                .serialize_uncompressed(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381G1), Some(SerializationFormat::BLS12381G1Uncompressed)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(
                context,
                handle,
                ark_bls12_381::G1Projective,
                element_ptr,
                element
            );
            let element_affine = element.into_affine();
            let mut buf = Vec::new();
            context.charge(ALGEBRA_ARK_BLS12_381_G1_AFFINE_SERIALIZE_UNCOMP)?;
            element_affine
                .serialize_uncompressed(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381G1), Some(SerializationFormat::BLS12381G1Compressed)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(
                context,
                handle,
                ark_bls12_381::G1Projective,
                element_ptr,
                element
            );
            let element_affine = element.into_affine();
            let mut buf = Vec::new();
            context.charge(ALGEBRA_ARK_BLS12_381_G1_AFFINE_SERIALIZE_COMP)?;
            element_affine
                .serialize_compressed(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381G2), Some(SerializationFormat::BLS12381G2Uncompressed)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(
                context,
                handle,
                ark_bls12_381::G2Projective,
                element_ptr,
                element
            );
            let element_affine = element.into_affine();
            let mut buf = Vec::new();
            context.charge(ALGEBRA_ARK_BLS12_381_G2_AFFINE_SERIALIZE_UNCOMP)?;
            element_affine
                .serialize_uncompressed(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381G2), Some(SerializationFormat::BLS12381G2Compressed)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(
                context,
                handle,
                ark_bls12_381::G2Projective,
                element_ptr,
                element
            );
            let element_affine = element.into_affine();
            let mut buf = Vec::new();
            context.charge(ALGEBRA_ARK_BLS12_381_G2_AFFINE_SERIALIZE_COMP)?;
            element_affine
                .serialize_compressed(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bls12_381::Fq12, element_ptr, element);
            let mut buf = vec![];
            context.charge(ALGEBRA_ARK_BLS12_381_FQ12_SERIALIZE)?;
            element
                .serialize_uncompressed(&mut buf)
                .map_err(|_e| abort_invariant_violated())?;
            Ok(smallvec![Value::vector_u8(buf)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
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
    let bytes_ref = vector_ref.as_bytes_ref();
    let bytes = bytes_ref.as_slice();
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
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
