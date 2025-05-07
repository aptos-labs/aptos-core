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
use better_any::{Tid, TidAble};
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
    /// Abort code when deserialization fails (leading 0x01 == INVALID_ARGUMENT)
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_DESERIALIZE_RANGE_PROOF: u64 = 0x01_0001;

    /// Abort code when input value for a range proof is too large.
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_VALUE_OUTSIDE_RANGE: u64 = 0x01_0002;

    /// Abort code when the requested range is larger than the maximum supported one.
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_RANGE_NOT_SUPPORTED: u64 = 0x01_0003;

    /// Abort code when the requested batch size is larger than the maximum supported one.
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_BATCH_SIZE_NOT_SUPPORTED: u64 = 0x01_0004;

    /// Abort code when the vector lengths of values and blinding factors do not match.
    /// NOTE: This must match the code in the Move implementation
    pub const NFE_VECTOR_LENGTHS_MISMATCH: u64 = 0x01_0005;

    /// Abort code when configured restriction of invoking `native_verify_range_proof` is violated.
    pub const NFE_INVOCATION_RESTRICTED: u64 = 0x03_0008;
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

/// Public parameters of the Bulletproof range proof system, for both individual and batch proving
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

    let ctx = context.extensions().get::<BulletproofContext>();
    if ctx.verify_batch_restricted && !ctx.called_from_system_entry_function {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_INVOCATION_RESTRICTED,
        });
    }

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
    context.charge(
        BULLETPROOFS_BASE
            + BULLETPROOFS_PER_BYTE_RANGEPROOF_DESERIALIZE
                * NumBytes::new(proof_bytes.len() as u64),
    )?;

    let range_proof = match bulletproofs::RangeProof::from_bytes(proof_bytes) {
        Ok(proof) => proof,
        Err(_) => {
            return Err(SafeNativeError::Abort {
                abort_code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
            })
        },
    };

    // The (Bullet)proof size is $\log_2(num_bits)$ and its verification time is $O(num_bits)$
    context.charge(BULLETPROOFS_PER_BIT_RANGEPROOF_VERIFY * NumArgs::new(bit_length as u64))?;

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
    charge_gas(context, comm_points.len(), bit_length)?;

    let range_proof = match bulletproofs::RangeProof::from_bytes(proof_bytes) {
        Ok(proof) => proof,
        Err(_) => {
            return Err(SafeNativeError::Abort {
                abort_code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
            })
        },
    };

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

/// Charges base gas fee for verifying and deserializing a Bulletproof range proof.
fn charge_gas(
    context: &mut SafeNativeContext,
    batch_size: usize,
    bit_length: usize,
) -> SafeNativeResult<()> {
    match (batch_size, bit_length) {
        (1, 8) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_1_BITS_8),
        (1, 16) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_1_BITS_16),
        (1, 32) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_1_BITS_32),
        (1, 64) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_1_BITS_64),
        (2, 8) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_2_BITS_8),
        (2, 16) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_2_BITS_16),
        (2, 32) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_2_BITS_32),
        (2, 64) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_2_BITS_64),
        (4, 8) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_4_BITS_8),
        (4, 16) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_4_BITS_16),
        (4, 32) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_4_BITS_32),
        (4, 64) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_4_BITS_64),
        (8, 8) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_8_BITS_8),
        (8, 16) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_8_BITS_16),
        (8, 32) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_8_BITS_32),
        (8, 64) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_8_BITS_64),
        (16, 8) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_16_BITS_8),
        (16, 16) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_16_BITS_16),
        (16, 32) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_16_BITS_32),
        (16, 64) => context.charge(BULLETPROOFS_VERIFY_BASE_BATCH_16_BITS_64),
        _ => unreachable!(),
    }
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

#[derive(Tid, Default)]
pub struct BulletproofContext {
    /// If true, `verify_batch_range_proof_internal` is only allowed
    /// in transactions that calls a system entry function (module address in 0x0 - 0xa).
    verify_batch_restricted: bool,
    pub called_from_system_entry_function: bool,
}

impl BulletproofContext {
    pub fn new(verify_batch_restricted: bool) -> Self {
        Self {
            verify_batch_restricted,
            called_from_system_entry_function: false,
        }
    }
}
