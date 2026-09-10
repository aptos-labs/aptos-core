// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Serialization and deserialization of elements.
//!
//! Deserialization checks the exact encoding length before handing the bytes to
//! arkworks: its cost grows with the input, so a wrong length must not be paid
//! for. A wrong length, or a well-formed encoding of a value outside the
//! structure, is `(false, 0)` rather than an abort.

use super::{
    algebra_invariant_violation, format_of, not_implemented, structure_of, AlgebraStore,
    SerializationFormat, Structure, BLS12381_R_SCALAR, BN254_R_SCALAR,
    E_SERIALIZATION_BLS12381GT_CONST_LOADING_FAILED,
};
use ark_ec::{short_weierstrass::Projective, CurveGroup};
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use mono_move_core::{
    native::{NativeContext, NativeStatus, Ref, Vector},
    VMResult,
};
use num_traits::One;
use std::any::Any;

/// Serializes a field element, optionally most-significant byte first.
fn serialize_field<C: NativeContext, T: Any + Copy + CanonicalSerialize>(
    ctx: &C,
    handle: u64,
    reverse: bool,
) -> VMResult<NativeStatus> {
    let mut buf = vec![];
    {
        let store = ctx.get_extension::<AlgebraStore>()?;
        if store
            .get::<T>(handle)?
            .serialize_uncompressed(&mut buf)
            .is_err()
        {
            return algebra_invariant_violation();
        }
    }
    if reverse {
        buf.reverse();
    }
    let out = ctx.new_byte_vector(&buf)?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// Serializes a curve point through its affine representation.
fn serialize_curve<C: NativeContext, G: Any + Copy + CurveGroup>(
    ctx: &C,
    handle: u64,
    compressed: bool,
) -> VMResult<NativeStatus> {
    let mut buf = vec![];
    {
        let store = ctx.get_extension::<AlgebraStore>()?;
        let affine = store.get::<G>(handle)?.into_affine();
        let written = if compressed {
            affine.serialize_compressed(&mut buf)
        } else {
            affine.serialize_uncompressed(&mut buf)
        };
        if written.is_err() {
            return algebra_invariant_violation();
        }
    }
    let out = ctx.new_byte_vector(&buf)?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::serialize_internal<S, F>(handle: u64): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_serialize<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    let format = format_of(ctx.ty_arg(1)?);
    // SAFETY: arg 0 is a `u64` handle.
    let handle = unsafe { ctx.arg::<u64>(0)? };

    match (structure, format) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
            serialize_field::<_, ark_bls12_381::Fr>(ctx, handle, false)
        },
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb)) => {
            serialize_field::<_, ark_bls12_381::Fr>(ctx, handle, true)
        },
        (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb)) => {
            serialize_field::<_, ark_bls12_381::Fq12>(ctx, handle, false)
        },
        (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            serialize_field::<_, ark_bls12_381::Fq12>(ctx, handle, false)
        },
        (Some(Structure::BN254Fr), Some(SerializationFormat::BN254FrLsb)) => {
            serialize_field::<_, ark_bn254::Fr>(ctx, handle, false)
        },
        (Some(Structure::BN254Fr), Some(SerializationFormat::BN254FrMsb)) => {
            serialize_field::<_, ark_bn254::Fr>(ctx, handle, true)
        },
        (Some(Structure::BN254Fq), Some(SerializationFormat::BN254FqLsb)) => {
            serialize_field::<_, ark_bn254::Fq>(ctx, handle, false)
        },
        (Some(Structure::BN254Fq), Some(SerializationFormat::BN254FqMsb)) => {
            serialize_field::<_, ark_bn254::Fq>(ctx, handle, true)
        },
        (Some(Structure::BN254Fq12), Some(SerializationFormat::BN254Fq12LscLsb)) => {
            serialize_field::<_, ark_bn254::Fq12>(ctx, handle, false)
        },
        (Some(Structure::BN254Gt), Some(SerializationFormat::BN254Gt)) => {
            serialize_field::<_, ark_bn254::Fq12>(ctx, handle, false)
        },
        (Some(Structure::BLS12381G1), Some(SerializationFormat::BLS12381G1Uncompressed)) => {
            serialize_curve::<_, ark_bls12_381::G1Projective>(ctx, handle, false)
        },
        (Some(Structure::BLS12381G1), Some(SerializationFormat::BLS12381G1Compressed)) => {
            serialize_curve::<_, ark_bls12_381::G1Projective>(ctx, handle, true)
        },
        (Some(Structure::BLS12381G2), Some(SerializationFormat::BLS12381G2Uncompressed)) => {
            serialize_curve::<_, ark_bls12_381::G2Projective>(ctx, handle, false)
        },
        (Some(Structure::BLS12381G2), Some(SerializationFormat::BLS12381G2Compressed)) => {
            serialize_curve::<_, ark_bls12_381::G2Projective>(ctx, handle, true)
        },
        (Some(Structure::BN254G1), Some(SerializationFormat::BN254G1Uncompressed)) => {
            serialize_curve::<_, ark_bn254::G1Projective>(ctx, handle, false)
        },
        (Some(Structure::BN254G1), Some(SerializationFormat::BN254G1Compressed)) => {
            serialize_curve::<_, ark_bn254::G1Projective>(ctx, handle, true)
        },
        (Some(Structure::BN254G2), Some(SerializationFormat::BN254G2Uncompressed)) => {
            serialize_curve::<_, ark_bn254::G2Projective>(ctx, handle, false)
        },
        (Some(Structure::BN254G2), Some(SerializationFormat::BN254G2Compressed)) => {
            serialize_curve::<_, ark_bn254::G2Projective>(ctx, handle, true)
        },
        _ => Ok(not_implemented()),
    }
}

/// Returns `(false, 0)`: the bytes are not a valid encoding of an element.
fn not_an_element<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: return slots 0 and 1 are `bool` and `u64`.
    unsafe {
        ctx.set_return(0, false)?;
        ctx.set_return(1, 0u64)?;
    }
    Ok(NativeStatus::Success)
}

/// Stores `convert(element)` and returns `(true, handle)`.
fn store_deserialized<C: NativeContext, T: Any>(ctx: &C, element: T) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let handle = match store.add(element) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slots 0 and 1 are `bool` and `u64`.
    unsafe {
        ctx.set_return(0, true)?;
        ctx.set_return(1, handle)?;
    }
    Ok(NativeStatus::Success)
}

/// Turns a deserialization result into `(true, handle)` or `(false, 0)`.
///
/// Only a malformed encoding is `(false, 0)`. Any other arkworks error means the
/// length check above let through something it should not have.
fn deserialized<C: NativeContext, T, U: Any>(
    ctx: &C,
    result: Result<T, SerializationError>,
    convert: impl FnOnce(T) -> U,
) -> VMResult<NativeStatus> {
    match result {
        Ok(element) => store_deserialized(ctx, convert(element)),
        Err(SerializationError::InvalidData) | Err(SerializationError::UnexpectedFlags) => {
            not_an_element(ctx)
        },
        Err(SerializationError::NotEnoughSpace) | Err(SerializationError::IoError(_)) => {
            algebra_invariant_violation()
        },
    }
}

/// `0x1::crypto_algebra::deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64)`
///
/// TODO(metering): charge gas.
pub fn native_deserialize<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    let format = format_of(ctx.ty_arg(1)?);
    // SAFETY: arg 0 is `&vector<u8>`.
    let bytes_ref = unsafe { ctx.arg::<Ref<Vector<u8>>>(0)? };
    let bytes_vec = bytes_ref.borrow();
    // SAFETY: the slice is copied out before any allocation.
    let bytes = unsafe { bytes_vec.as_bytes() }.to_vec();

    match (structure, format) {
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrLsb)) => {
            deserialize_field::<_, ark_bls12_381::Fr>(ctx, &bytes, 32, false)
        },
        (Some(Structure::BLS12381Fr), Some(SerializationFormat::BLS12381FrMsb)) => {
            deserialize_field::<_, ark_bls12_381::Fr>(ctx, &bytes, 32, true)
        },
        (Some(Structure::BLS12381Fq12), Some(SerializationFormat::BLS12381Fq12LscLsb)) => {
            deserialize_field::<_, ark_bls12_381::Fq12>(ctx, &bytes, 576, false)
        },
        (Some(Structure::BN254Fr), Some(SerializationFormat::BN254FrLsb)) => {
            deserialize_field::<_, ark_bn254::Fr>(ctx, &bytes, 32, false)
        },
        (Some(Structure::BN254Fr), Some(SerializationFormat::BN254FrMsb)) => {
            deserialize_field::<_, ark_bn254::Fr>(ctx, &bytes, 32, true)
        },
        (Some(Structure::BN254Fq), Some(SerializationFormat::BN254FqLsb)) => {
            deserialize_field::<_, ark_bn254::Fq>(ctx, &bytes, 32, false)
        },
        (Some(Structure::BN254Fq), Some(SerializationFormat::BN254FqMsb)) => {
            deserialize_field::<_, ark_bn254::Fq>(ctx, &bytes, 32, true)
        },
        (Some(Structure::BN254Fq12), Some(SerializationFormat::BN254Fq12LscLsb)) => {
            deserialize_field::<_, ark_bn254::Fq12>(ctx, &bytes, 384, false)
        },
        (Some(Structure::BLS12381G1), Some(SerializationFormat::BLS12381G1Uncompressed)) => {
            deserialize_curve::<_, ark_bls12_381::g1::Config>(ctx, &bytes, 96, false)
        },
        (Some(Structure::BLS12381G1), Some(SerializationFormat::BLS12381G1Compressed)) => {
            deserialize_curve::<_, ark_bls12_381::g1::Config>(ctx, &bytes, 48, true)
        },
        (Some(Structure::BLS12381G2), Some(SerializationFormat::BLS12381G2Uncompressed)) => {
            deserialize_curve::<_, ark_bls12_381::g2::Config>(ctx, &bytes, 192, false)
        },
        (Some(Structure::BLS12381G2), Some(SerializationFormat::BLS12381G2Compressed)) => {
            deserialize_curve::<_, ark_bls12_381::g2::Config>(ctx, &bytes, 96, true)
        },
        (Some(Structure::BN254G1), Some(SerializationFormat::BN254G1Uncompressed)) => {
            deserialize_curve::<_, ark_bn254::g1::Config>(ctx, &bytes, 64, false)
        },
        (Some(Structure::BN254G1), Some(SerializationFormat::BN254G1Compressed)) => {
            deserialize_curve::<_, ark_bn254::g1::Config>(ctx, &bytes, 32, true)
        },
        (Some(Structure::BN254G2), Some(SerializationFormat::BN254G2Uncompressed)) => {
            deserialize_curve::<_, ark_bn254::g2::Config>(ctx, &bytes, 128, false)
        },
        (Some(Structure::BN254G2), Some(SerializationFormat::BN254G2Compressed)) => {
            deserialize_curve::<_, ark_bn254::g2::Config>(ctx, &bytes, 64, true)
        },
        (Some(Structure::BLS12381Gt), Some(SerializationFormat::BLS12381Gt)) => {
            if bytes.len() != 576 {
                return not_an_element(ctx);
            }
            let Some(r_scalar) = BLS12381_R_SCALAR.as_ref() else {
                return Ok(NativeStatus::Abort {
                    code: E_SERIALIZATION_BLS12381GT_CONST_LOADING_FAILED,
                    message: Some("BLS12381 GT constant loading failed".to_string()),
                });
            };
            match ark_bls12_381::Fq12::deserialize_uncompressed(bytes.as_slice()) {
                Ok(element) if element.pow(r_scalar.0) == ark_bls12_381::Fq12::one() => {
                    store_deserialized(ctx, element)
                },
                Ok(_) | Err(_) => not_an_element(ctx),
            }
        },
        (Some(Structure::BN254Gt), Some(SerializationFormat::BN254Gt)) => {
            if bytes.len() != 384 {
                return not_an_element(ctx);
            }
            match ark_bn254::Fq12::deserialize_uncompressed(bytes.as_slice()) {
                Ok(element) if element.pow(BN254_R_SCALAR.0) == ark_bn254::Fq12::one() => {
                    store_deserialized(ctx, element)
                },
                Ok(_) | Err(_) => not_an_element(ctx),
            }
        },
        _ => Ok(not_implemented()),
    }
}

/// Deserializes a field element of exactly `len` bytes, optionally
/// most-significant byte first.
fn deserialize_field<C: NativeContext, T: Any + CanonicalDeserialize>(
    ctx: &C,
    bytes: &[u8],
    len: usize,
    reverse: bool,
) -> VMResult<NativeStatus> {
    if bytes.len() != len {
        return not_an_element(ctx);
    }
    let reversed;
    let bytes = if reverse {
        reversed = bytes.iter().rev().copied().collect::<Vec<u8>>();
        reversed.as_slice()
    } else {
        bytes
    };
    deserialized(ctx, T::deserialize_uncompressed(bytes), |element| element)
}

/// Deserializes a curve point of exactly `len` bytes and stores it in
/// projective form.
fn deserialize_curve<C: NativeContext, P: ark_ec::short_weierstrass::SWCurveConfig>(
    ctx: &C,
    bytes: &[u8],
    len: usize,
    compressed: bool,
) -> VMResult<NativeStatus>
where
    Projective<P>: Any,
{
    if bytes.len() != len {
        return not_an_element(ctx);
    }
    let result = if compressed {
        ark_ec::short_weierstrass::Affine::<P>::deserialize_compressed(bytes)
    } else {
        ark_ec::short_weierstrass::Affine::<P>::deserialize_uncompressed(bytes)
    };
    deserialized(ctx, result, Projective::<P>::from)
}
