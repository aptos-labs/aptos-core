// Copyright © Aptos Foundation

use crate::{
    abort_unless_feature_flag_enabled,
    natives::{
        cryptography::algebra::{
            abort_invariant_violated, gas::GasParameters, AlgebraContext, SerializationFormat,
            Structure, BLS12381_R_SCALAR, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        },
        helpers::{SafeNativeContext, SafeNativeError, SafeNativeResult},
    },
    safe_borrow_element, safely_pop_arg, store_element, structure_from_ty_arg,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ec::CurveGroup;
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use move_core_types::gas_algebra::NumArgs;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use num_traits::One;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

pub fn feature_flag_of_serialization_format(
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

macro_rules! abort_unless_serialization_format_enabled {
    ($context:ident, $structure_opt:expr, $format_opt:expr) => {
        let flag_opt = feature_flag_of_serialization_format($structure_opt, $format_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! format_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ)?;
        SerializationFormat::try_from(type_tag).ok()
    }};
}

/// Macros that implements `serialize_internal()` using arkworks libraries.
macro_rules! ark_serialize_internal {
    ($context:expr, $args:ident, $ark_type:ty, $ark_ser_func:ident, $gas:expr) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $ark_type, element_ptr, element);
        let mut buf = vec![];
        $context.charge($gas)?;
        match element.$ark_ser_func(&mut buf) {
            Ok(_) => {},
            _ => {
                abort_invariant_violated();
                unreachable!()
            },
        }
        buf
    }};
}

macro_rules! ark_ec_point_serialize_internal {
    ($context:expr, $args:ident, $ark_type:ty, $ark_ser_func:ident, $gas:expr) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $ark_type, element_ptr, element);
        let element_affine = element.into_affine();
        let mut buf = Vec::new();
        $context.charge($gas)?;
        match element_affine.$ark_ser_func(&mut buf) {
            Ok(_) => {},
            _ => {
                abort_invariant_violated();
                unreachable!()
            },
        }
        buf
    }};
}

pub fn serialize_internal(
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
                context,
                args,
                ark_bls12_381::Fr,
                serialize_uncompressed,
                gas_params.ark_bls12_381_fr_serialize * NumArgs::one()
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb)) => {
            let mut buf = ark_serialize_internal!(
                context,
                args,
                ark_bls12_381::Fr,
                serialize_uncompressed,
                gas_params.ark_bls12_381_fr_serialize * NumArgs::one()
            );
            buf.reverse();
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb)) => {
            let buf = ark_serialize_internal!(
                context,
                args,
                ark_bls12_381::Fq12,
                serialize_uncompressed,
                gas_params.ark_bls12_381_fq12_serialize * NumArgs::one()
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (
            Some(Structure::BLS12381G1Affine),
            Some(SerializationFormat::BLS12381G1AffineUncompressed),
        ) => {
            let buf = ark_ec_point_serialize_internal!(
                context,
                args,
                ark_bls12_381::G1Projective,
                serialize_uncompressed,
                gas_params.ark_bls12_381_g1_affine_serialize_uncomp * NumArgs::one()
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (
            Some(Structure::BLS12381G1Affine),
            Some(SerializationFormat::BLS12381G1AffineCompressed),
        ) => {
            let buf = ark_ec_point_serialize_internal!(
                context,
                args,
                ark_bls12_381::G1Projective,
                serialize_compressed,
                gas_params.ark_bls12_381_g1_affine_serialize_comp * NumArgs::one()
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (
            Some(Structure::BLS12381G2Affine),
            Some(SerializationFormat::BLS12381G2AffineUncompressed),
        ) => {
            let buf = ark_ec_point_serialize_internal!(
                context,
                args,
                ark_bls12_381::G2Projective,
                serialize_uncompressed,
                gas_params.ark_bls12_381_g2_affine_serialize_uncomp * NumArgs::one()
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (
            Some(Structure::BLS12381G2Affine),
            Some(SerializationFormat::BLS12381G2AffineCompressed),
        ) => {
            let buf = ark_ec_point_serialize_internal!(
                context,
                args,
                ark_bls12_381::G2Projective,
                serialize_compressed,
                gas_params.ark_bls12_381_g2_affine_serialize_comp * NumArgs::one()
            );
            Ok(smallvec![Value::vector_u8(buf)])
        },
        (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            let buf = ark_serialize_internal!(
                context,
                args,
                ark_bls12_381::Fq12,
                serialize_uncompressed,
                gas_params.ark_bls12_381_fq12_serialize * NumArgs::one()
            );
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
                let handle = store_element!($context, element);
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            },
            Err(ark_serialize::SerializationError::InvalidData)
            | Err(ark_serialize::SerializationError::UnexpectedFlags) => {
                Ok(smallvec![Value::bool(false), Value::u64(0)])
            },
            _ => {
                abort_invariant_violated();
                unreachable!()
            },
        }
    }};
}

macro_rules! ark_ec_point_deserialize_internal {
    ($context:expr, $bytes:expr, $typ:ty, $deser_func:ident, $gas:expr) => {{
        $context.charge($gas)?;
        match <$typ>::$deser_func($bytes) {
            Ok(element) => {
                let element_proj = ark_ec::short_weierstrass::Projective::from(element);
                let handle = store_element!($context, element_proj);
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            },
            Err(ark_serialize::SerializationError::InvalidData)
            | Err(ark_serialize::SerializationError::UnexpectedFlags) => {
                Ok(smallvec![Value::bool(false), Value::u64(0)])
            },
            _ => {
                abort_invariant_violated();
                unreachable!()
            },
        }
    }};
}

pub fn deserialize_internal(
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
                context,
                bytes,
                ark_bls12_381::Fr,
                deserialize_uncompressed,
                gas_params.ark_bls12_381_fr_deser * NumArgs::one()
            )
        },
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb)) => {
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
                gas_params.ark_bls12_381_fr_deser * NumArgs::one()
            )
        },
        (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb)) => {
            if bytes.len() != 576 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            ark_deserialize_internal!(
                context,
                bytes,
                ark_bls12_381::Fq12,
                deserialize_uncompressed,
                gas_params.ark_bls12_381_fq12_deser * NumArgs::one()
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
                context,
                bytes,
                ark_bls12_381::G1Affine,
                deserialize_uncompressed,
                gas_params.ark_bls12_381_g1_affine_deser_uncomp * NumArgs::one()
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
                context,
                bytes,
                ark_bls12_381::G1Affine,
                deserialize_compressed,
                gas_params.ark_bls12_381_g1_affine_deser_comp * NumArgs::one()
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
                context,
                bytes,
                ark_bls12_381::G2Affine,
                deserialize_uncompressed,
                gas_params.ark_bls12_381_g2_affine_deser_uncomp * NumArgs::one()
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
                context,
                bytes,
                ark_bls12_381::G2Affine,
                deserialize_compressed,
                gas_params.ark_bls12_381_g2_affine_deser_comp * NumArgs::one()
            )
        },
        (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            if bytes.len() != 576 {
                return Ok(smallvec![Value::bool(false), Value::u64(0)]);
            }
            context.charge(gas_params.ark_bls12_381_fq12_deser * NumArgs::one())?;
            match <ark_bls12_381::Fq12>::deserialize_uncompressed(bytes) {
                Ok(element) => {
                    context.charge(
                        (gas_params.ark_bls12_381_fq12_pow_u256 + gas_params.ark_bls12_381_fq12_eq)
                            * NumArgs::one(),
                    )?;
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
