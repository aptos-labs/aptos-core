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
    pub const NFE_DESERIALIZE_RANGE_PROOF: u64 = 0x01_0001;
    pub const NFE_VALUE_OUTSIDE_RANGE: u64 = 0x01_0002;
    pub const NFE_RANGE_NOT_SUPPORTED: u64 = 0x01_0003;
    pub const NFE_BATCH_SIZE_NOT_SUPPORTED: u64 = 0x01_0004;
    pub const NFE_VECTOR_LENGTHS_MISMATCH: u64 = 0x01_0005;
}

fn bit_length_is_valid(bits: usize) -> bool {
    matches!(bits, 8 | 16 | 32 | 64)
}

fn batch_count_is_valid(n: usize) -> bool {
    matches!(n, 1 | 2 | 4 | 8 | 16)
}

static BP_GENS: Lazy<BulletproofGens> =
    Lazy::new(|| BulletproofGens::new(MAX_RANGE_BITS, 16));

fn resolve_pedersen_bases(
    ctx: &SafeNativeContext,
    val_base_handle: &crate::natives::cryptography::ristretto255_point::RistrettoPointHandle,
    rand_base_handle: &crate::natives::cryptography::ristretto255_point::RistrettoPointHandle,
) -> Result<PedersenGens, SafeNativeError> {
    let pt_ctx = ctx.extensions().get::<NativeRistrettoPointContext>();
    let store = pt_ctx.point_data.borrow_mut();

    let val_base = store.get_point(val_base_handle)?;
    let rand_base = store.get_point(rand_base_handle)?;

    Ok(PedersenGens {
        B: *val_base,
        B_blinding: *rand_base,
    })
}

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

    if !bit_length_is_valid(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }

    let commitment = CompressedRistretto::from_slice(comm_bytes.as_slice());
    let pg = resolve_pedersen_bases(context, &val_base_handle, &rand_base_handle)?;

    execute_single_verify(context, &commitment, &pg, &proof_bytes, num_bits, dst)
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
    let raw_commitments = safely_pop_vec_arg!(args, Vec<u8>);

    if !bit_length_is_valid(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }
    if !batch_count_is_valid(raw_commitments.len()) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_BATCH_SIZE_NOT_SUPPORTED,
        });
    }

    let commitments: Vec<CompressedRistretto> = raw_commitments
        .iter()
        .map(|bytes| CompressedRistretto::from_slice(bytes.as_slice()))
        .collect();

    let pg = resolve_pedersen_bases(context, &val_base_handle, &rand_base_handle)?;

    execute_batch_verify(context, &commitments, &pg, &proof_bytes, num_bits, dst)
}

#[cfg(feature = "testing")]
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
    let blinding = pop_scalar_from_bytes(&mut args)?;
    let value_scalar = pop_scalar_from_bytes(&mut args)?;

    if !bit_length_is_valid(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }

    if value_scalar.as_bytes()[8..].iter().any(|&b| b != 0) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_VALUE_OUTSIDE_RANGE,
        });
    }

    let value_u64 = LittleEndian::read_u64(value_scalar.as_bytes());
    let pg = resolve_pedersen_bases(context, &val_base_handle, &rand_base_handle)?;
    let mut transcript = Transcript::new(dst.as_slice());

    let (proof, commitment) = bulletproofs::RangeProof::prove_single(
        &BP_GENS,
        &pg,
        &mut transcript,
        value_u64,
        &blinding,
        num_bits,
    )
    .map_err(|_| SafeNativeError::Abort {
        abort_code: abort_codes::NFE_VALUE_OUTSIDE_RANGE,
    })?;

    Ok(smallvec![
        Value::vector_u8(proof.to_bytes()),
        Value::vector_u8(commitment.as_bytes().to_vec())
    ])
}

#[cfg(feature = "testing")]
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
    let blindings = pop_scalars_from_bytes(&mut args)?;
    let value_scalars = pop_scalars_from_bytes(&mut args)?;

    if !bit_length_is_valid(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }
    if !batch_count_is_valid(value_scalars.len()) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_BATCH_SIZE_NOT_SUPPORTED,
        });
    }
    if value_scalars.len() != blindings.len() {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_VECTOR_LENGTHS_MISMATCH,
        });
    }

    if value_scalars
        .iter()
        .any(|s| s.as_bytes()[8..].iter().any(|&b| b != 0))
    {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_VALUE_OUTSIDE_RANGE,
        });
    }

    let values_u64: Vec<u64> = value_scalars
        .iter()
        .map(|s| LittleEndian::read_u64(s.as_bytes()))
        .collect();

    let pg = resolve_pedersen_bases(context, &val_base_handle, &rand_base_handle)?;
    let mut transcript = Transcript::new(dst.as_slice());

    let (proof, commitments) = bulletproofs::RangeProof::prove_multiple(
        &BP_GENS,
        &pg,
        &mut transcript,
        &values_u64,
        &blindings,
        num_bits,
    )
    .map_err(|_| SafeNativeError::Abort {
        abort_code: abort_codes::NFE_VALUE_OUTSIDE_RANGE,
    })?;

    Ok(smallvec![
        Value::vector_u8(proof.to_bytes()),
        Value::vector_for_testing_only(
            commitments
                .iter()
                .map(|c| Value::vector_u8(c.as_bytes().to_vec()))
                .collect::<Vec<_>>()
        )
    ])
}

fn execute_single_verify(
    context: &mut SafeNativeContext,
    commitment: &CompressedRistretto,
    pedersen: &PedersenGens,
    proof_bytes: &[u8],
    bits: usize,
    dst: Vec<u8>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(
        BULLETPROOFS_BASE
            + BULLETPROOFS_PER_BYTE_RANGEPROOF_DESERIALIZE
                * NumBytes::new(proof_bytes.len() as u64),
    )?;

    let range_proof = bulletproofs::RangeProof::from_bytes(proof_bytes).map_err(|_| {
        SafeNativeError::Abort {
            abort_code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
        }
    })?;

    context.charge(BULLETPROOFS_PER_BIT_RANGEPROOF_VERIFY * NumArgs::new(bits as u64))?;

    let mut transcript = Transcript::new(dst.as_slice());
    let ok = range_proof
        .verify_single(&BP_GENS, pedersen, &mut transcript, commitment, bits)
        .is_ok();

    Ok(smallvec![Value::bool(ok)])
}

fn execute_batch_verify(
    context: &mut SafeNativeContext,
    commitments: &[CompressedRistretto],
    pedersen: &PedersenGens,
    proof_bytes: &[u8],
    bits: usize,
    dst: Vec<u8>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    charge_batch_gas(context, commitments.len(), bits)?;

    let range_proof = bulletproofs::RangeProof::from_bytes(proof_bytes).map_err(|_| {
        SafeNativeError::Abort {
            abort_code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
        }
    })?;

    let mut transcript = Transcript::new(dst.as_slice());
    let ok = range_proof
        .verify_multiple(&BP_GENS, pedersen, &mut transcript, commitments, bits)
        .is_ok();

    Ok(smallvec![Value::bool(ok)])
}

fn charge_batch_gas(
    context: &mut SafeNativeContext,
    batch_size: usize,
    bits: usize,
) -> SafeNativeResult<()> {
    match (batch_size, bits) {
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
        _ => Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_BATCH_SIZE_NOT_SUPPORTED,
        }),
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
