// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
use crate::natives::cryptography::ristretto255::{pop_scalar_from_bytes, pop_scalars_from_bytes};
use crate::natives::cryptography::ristretto255_point::{
    get_point_handle, NativeRistrettoPointContext,
};
use aptos_crypto::bulletproofs::MAX_RANGE_BITS;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, safely_pop_vec_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeError, SafeNativeResult,
};
use bulletproofs::{BulletproofGens, PedersenGens};
#[cfg(feature = "testing")]
use byteorder::{ByteOrder, LittleEndian};
use curve25519_dalek::ristretto::CompressedRistretto;
use merlin::Transcript;
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{StructRef, Value},
};
use once_cell::sync::Lazy;
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

pub mod abort_codes {
    /// Abort code when input value for a range proof is too large.
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_VALUE_OUTSIDE_RANGE: u64 = 0x01_0001;

    /// Abort code when the requested range is larger than the maximum supported one.
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_RANGE_NOT_SUPPORTED: u64 = 0x01_0002;

    /// Abort code when the requested batch size is larger than the maximum supported one.
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_BATCH_SIZE_NOT_SUPPORTED: u64 = 0x01_0003;

    /// Abort code when the vector lengths of values and blinding factors do not match.
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_VECTOR_LENGTHS_MISMATCH: u64 = 0x01_0004;
}

/// The Bulletproofs library only seems to support proving [0, 2^{num_bits}) ranges where num_bits is
/// either 8, 16, 32 or 64.
fn is_supported_number_of_bits(num_bits: usize) -> bool {
    matches!(num_bits, 8 | 16 | 32 | 64)
}

/// The Bulletproofs library only supports batch sizes of 1, 2, 4, 8, or 16.
fn is_supported_batch_size(batch_size: usize) -> bool {
    matches!(batch_size, 1 | 2 | 4 | 8 | 16)
}

/// Public parameters of the Bulletproof range proof system
static BULLETPROOF_GENERATORS: Lazy<BulletproofGens> =
    Lazy::new(|| BulletproofGens::new(MAX_RANGE_BITS, 16));

fn native_verify_range_proof(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 6);

    let dst = safely_pop_arg!(args, Vec<u8>);
    let num_bits = safely_pop_arg!(args, u64) as usize;
    let proof_bytes = safely_pop_arg!(args, Vec<u8>);
    let rand_base_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let val_base_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let comm_bytes = safely_pop_arg!(args, Vec<u8>);

    let comm_point = CompressedRistretto::from_slice(comm_bytes.as_slice());

    if !is_supported_number_of_bits(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }

    let pg = {
        let point_context = context.extensions().get::<NativeRistrettoPointContext>();
        let point_data = point_context.point_data.borrow_mut();

        let rand_base = point_data.get_point(&rand_base_handle);
        let val_base = point_data.get_point(&val_base_handle);

        // TODO(Perf): Is there a way to avoid this unnecessary cloning here?
        PedersenGens {
            B: *val_base,
            B_blinding: *rand_base,
        }
    };

    verify_range_proof(context, &comm_point, &pg, &proof_bytes[..], num_bits, dst)
}

fn native_verify_batch_range_proof(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 6);

    let dst = safely_pop_arg!(args, Vec<u8>);
    let num_bits = safely_pop_arg!(args, u64) as usize;
    let proof_bytes = safely_pop_arg!(args, Vec<u8>);
    let rand_base_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let val_base_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let comm_bytes = safely_pop_vec_arg!(args, Vec<u8>);

    let comm_points = comm_bytes
        .iter()
        .map(|comm_bytes| CompressedRistretto::from_slice(comm_bytes.as_slice()))
        .collect::<Vec<_>>();

    if !is_supported_number_of_bits(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }
    if !is_supported_batch_size(comm_points.len()) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_BATCH_SIZE_NOT_SUPPORTED,
        });
    }

    let pg = {
        let point_context = context.extensions().get::<NativeRistrettoPointContext>();
        let point_data = point_context.point_data.borrow_mut();

        let rand_base = point_data.get_point(&rand_base_handle);
        let val_base = point_data.get_point(&val_base_handle);

        // TODO(Perf): Is there a way to avoid this unnecessary cloning here?
        PedersenGens {
            B: *val_base,
            B_blinding: *rand_base,
        }
    };

    verify_batch_range_proof(context, &comm_points, &pg, &proof_bytes[..], num_bits, dst)
}

#[cfg(feature = "testing")]
/// This is a test-only native that charges zero gas. It is only exported in testing mode.
fn native_test_only_prove_range(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 6);

    let rand_base_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let val_base_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let dst = safely_pop_arg!(args, Vec<u8>);
    let num_bits = safely_pop_arg!(args, u64) as usize;
    let v_blinding = pop_scalar_from_bytes(&mut args)?;
    let v = pop_scalar_from_bytes(&mut args)?;

    if !is_supported_number_of_bits(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }

    // Make sure only the first 64 bits are set.
    if !v.as_bytes()[8..].iter().all(|&byte| byte == 0u8) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_VALUE_OUTSIDE_RANGE,
        });
    }

    // Convert Scalar to u64.
    let v = LittleEndian::read_u64(v.as_bytes());

    let mut t = Transcript::new(dst.as_slice());

    let pg = {
        let point_context = context.extensions().get::<NativeRistrettoPointContext>();
        let point_data = point_context.point_data.borrow_mut();

        let rand_base = point_data.get_point(&rand_base_handle);
        let val_base = point_data.get_point(&val_base_handle);

        // TODO(Perf): Is there a way to avoid this unnecessary cloning here?
        PedersenGens {
            B: *val_base,
            B_blinding: *rand_base,
        }
    };

    // Construct a range proof.
    let (proof, commitment) = bulletproofs::RangeProof::prove_single(
        &BULLETPROOF_GENERATORS,
        &pg,
        &mut t,
        v,
        &v_blinding,
        num_bits,
    )
    .expect("Bulletproofs prover failed unexpectedly");

    Ok(smallvec![
        Value::vector_u8(proof.to_bytes()),
        Value::vector_u8(commitment.as_bytes().to_vec())
    ])
}

#[cfg(feature = "testing")]
/// This is a test-only native that charges zero gas. It is only exported in testing mode.
fn native_test_only_batch_prove_range(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 6);

    let rand_base_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let val_base_handle = get_point_handle(&safely_pop_arg!(args, StructRef))?;
    let dst = safely_pop_arg!(args, Vec<u8>);
    let num_bits = safely_pop_arg!(args, u64) as usize;
    let v_blindings = pop_scalars_from_bytes(&mut args)?;
    let vs = pop_scalars_from_bytes(&mut args)?;

    if !is_supported_number_of_bits(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }
    if !is_supported_batch_size(vs.len()) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_BATCH_SIZE_NOT_SUPPORTED,
        });
    }
    if vs.len() != v_blindings.len() {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_VECTOR_LENGTHS_MISMATCH,
        });
    }

    // Make sure only the first 64 bits are set for each Scalar.
    if !vs
        .iter()
        .all(|v| v.as_bytes()[8..].iter().all(|&byte| byte == 0u8))
    {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_VALUE_OUTSIDE_RANGE,
        });
    }

    // Convert each Scalar to u64.
    let vs = vs
        .iter()
        .map(|v| LittleEndian::read_u64(v.as_bytes()))
        .collect::<Vec<_>>();

    let mut t = Transcript::new(dst.as_slice());

    let pg = {
        let point_context = context.extensions().get::<NativeRistrettoPointContext>();
        let point_data = point_context.point_data.borrow_mut();

        let rand_base = point_data.get_point(&rand_base_handle);
        let val_base = point_data.get_point(&val_base_handle);

        // TODO(Perf): Is there a way to avoid this unnecessary cloning here?
        PedersenGens {
            B: *val_base,
            B_blinding: *rand_base,
        }
    };

    // Construct a range proof.
    let (proof, commitments) = bulletproofs::RangeProof::prove_multiple(
        &BULLETPROOF_GENERATORS,
        &pg,
        &mut t,
        &vs,
        &v_blindings,
        num_bits,
    )
    .expect("Bulletproofs prover failed unexpectedly");

    Ok(smallvec![
        Value::vector_u8(proof.to_bytes()),
        Value::vector_for_testing_only(
            commitments
                .iter()
                .map(|commitment| Value::vector_u8(commitment.as_bytes().to_vec()))
                .collect::<Vec<_>>()
        )
    ])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
/// Helper function to gas meter and verify a single Bulletproof range proof for a Pedersen
/// commitment with `pc_gens` as its commitment key.
fn verify_range_proof(
    context: &mut SafeNativeContext,
    comm_point: &CompressedRistretto,
    pc_gens: &PedersenGens,
    proof_bytes: &[u8],
    bit_length: usize,
    dst: Vec<u8>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // Batch size of 1 corresponds to the first element in the array.
    charge_gas_for_deserialization(context, proof_bytes, 1)?;

    let range_proof = match bulletproofs::RangeProof::from_bytes(proof_bytes) {
        Ok(proof) => proof,
        Err(_) => return Ok(smallvec![Value::bool(false)]),
    };

    // The (Bullet)proof size is $\log_2(num_bits)$ and its verification time is $O(num_bits)$
    charge_gas_for_verification(context, bit_length, 1)?;

    let mut ver_trans = Transcript::new(dst.as_slice());

    let success = range_proof
        .verify_single(
            &BULLETPROOF_GENERATORS,
            pc_gens,
            &mut ver_trans,
            comm_point,
            bit_length,
        )
        .is_ok();

    Ok(smallvec![Value::bool(success)])
}

/// Helper function to gas meter and verify a batch Bulletproof range proof for Pedersen
/// commitments with `pc_gens` as their commitment keys.
fn verify_batch_range_proof(
    context: &mut SafeNativeContext,
    comm_points: &[CompressedRistretto],
    pc_gens: &PedersenGens,
    proof_bytes: &[u8],
    bit_length: usize,
    dst: Vec<u8>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    charge_gas_for_deserialization(context, proof_bytes, comm_points.len())?;

    let range_proof = match bulletproofs::RangeProof::from_bytes(proof_bytes) {
        Ok(proof) => proof,
        Err(_) => return Ok(smallvec![Value::bool(false)]),
    };

    // The (Bullet)proof size is $\log_2(num_bits)$ and its verification time is $O(num_bits)$
    charge_gas_for_verification(context, bit_length, comm_points.len())?;

    let mut ver_trans = Transcript::new(dst.as_slice());

    let success = range_proof
        .verify_multiple(
            &BULLETPROOF_GENERATORS,
            pc_gens,
            &mut ver_trans,
            comm_points,
            bit_length,
        )
        .is_ok();

    Ok(smallvec![Value::bool(success)])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    #[cfg(feature = "testing")]
    natives.extend([
        (
            "prove_range_internal",
            native_test_only_prove_range as RawSafeNative,
        ),
        (
            "prove_batch_range_internal",
            native_test_only_batch_prove_range,
        ),
    ]);

    natives.extend([
        (
            "verify_range_proof_internal",
            native_verify_range_proof as RawSafeNative,
        ),
        (
            "verify_batch_range_proof_internal",
            native_verify_batch_range_proof,
        ),
    ]);

    builder.make_named_natives(natives)
}

/// Charges gas for deserializing a Bulletproof range proof.
fn charge_gas_for_deserialization(
    context: &mut SafeNativeContext,
    proof_bytes: &[u8],
    batch_size: usize,
) -> SafeNativeResult<()> {
    let proof_bytes_len = NumBytes::new(proof_bytes.len() as u64);

    match batch_size {
        1 => context.charge(
            BULLETPROOFS_DESERIALIZE_BASE_1 + BULLETPROOFS_DESERIALIZE_PER_BYTE_1 * proof_bytes_len,
        ),
        2 => context.charge(
            BULLETPROOFS_DESERIALIZE_BASE_2 + BULLETPROOFS_DESERIALIZE_PER_BYTE_2 * proof_bytes_len,
        ),
        4 => context.charge(
            BULLETPROOFS_DESERIALIZE_BASE_4 + BULLETPROOFS_DESERIALIZE_PER_BYTE_4 * proof_bytes_len,
        ),
        8 => context.charge(
            BULLETPROOFS_DESERIALIZE_BASE_8 + BULLETPROOFS_DESERIALIZE_PER_BYTE_8 * proof_bytes_len,
        ),
        16 => context.charge(
            BULLETPROOFS_DESERIALIZE_BASE_16
                + BULLETPROOFS_DESERIALIZE_PER_BYTE_16 * proof_bytes_len,
        ),
        _ => unreachable!(),
    }
}

/// Charges gas for verifying a Bulletproof range proof.
fn charge_gas_for_verification(
    context: &mut SafeNativeContext,
    bit_length: usize,
    batch_size: usize,
) -> SafeNativeResult<()> {
    let bit_length = NumArgs::new(bit_length as u64);

    match batch_size {
        1 => {
            context.charge(BULLETPROOFS_VERIFY_BASE_1 + BULLETPROOFS_VERIFY_PER_BIT_1 * bit_length)
        },
        2 => {
            context.charge(BULLETPROOFS_VERIFY_BASE_2 + BULLETPROOFS_VERIFY_PER_BIT_2 * bit_length)
        },
        4 => {
            context.charge(BULLETPROOFS_VERIFY_BASE_4 + BULLETPROOFS_VERIFY_PER_BIT_4 * bit_length)
        },
        8 => {
            context.charge(BULLETPROOFS_VERIFY_BASE_8 + BULLETPROOFS_VERIFY_PER_BIT_8 * bit_length)
        },
        16 => context
            .charge(BULLETPROOFS_VERIFY_BASE_16 + BULLETPROOFS_VERIFY_PER_BIT_16 * bit_length),
        _ => unreachable!(),
    }
}
