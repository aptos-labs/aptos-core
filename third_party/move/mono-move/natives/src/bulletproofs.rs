// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bulletproofs range-proof natives for the `ristretto255_bulletproofs` module
//! (`aptos_std::ristretto255_bulletproofs`).
//!
//! The two base points come in as `&RistrettoPoint` handles into the
//! [`RistrettoPointStore`], the same as every other ristretto255 native. An
//! invalid handle is an invariant violation here rather than V1's
//! `NFE_INVALID_POINT_*` abort, matching the sibling point natives; well-typed
//! Move cannot produce one.

#[cfg(feature = "testing")]
use crate::ristretto255_scalar::scalar_from_bytes;
use crate::{
    monomorphic_natives,
    ristretto255_point::{point_handle, RistrettoPointStore},
    NativeEntry,
};
use aptos_crypto::bulletproofs::MAX_RANGE_BITS;
use aptos_types::error;
use bulletproofs::{BulletproofGens, PedersenGens};
#[cfg(feature = "testing")]
use byteorder::{ByteOrder, LittleEndian};
use curve25519_dalek::ristretto::CompressedRistretto;
#[cfg(feature = "testing")]
use curve25519_dalek::scalar::Scalar;
use merlin::Transcript;
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeStatus, Ref, Vector,
    },
    VMResult,
};
use once_cell::sync::Lazy;

// These codes are observable on chain, so they must stay bit-identical to the
// legacy native's. The reasons are the legacy numbering, gaps included: the
// `NFE_INVALID_POINT_*` reasons it skips are invariant violations here.

/// Range proof deserialization failed. Must match the Move implementation.
const NFE_DESERIALIZE_RANGE_PROOF: u64 = error::invalid_argument(1);

/// Input value for a range proof is too large.
#[cfg(feature = "testing")]
const NFE_VALUE_OUTSIDE_RANGE: u64 = error::invalid_argument(2);

/// The requested range is larger than the maximum supported one.
const NFE_RANGE_NOT_SUPPORTED: u64 = error::invalid_argument(3);

/// The requested batch size is larger than the maximum supported one.
const NFE_BATCH_SIZE_NOT_SUPPORTED: u64 = error::invalid_argument(4);

/// The vector lengths of values and blinding factors do not match.
#[cfg(feature = "testing")]
const NFE_VECTOR_LENGTHS_MISMATCH: u64 = error::invalid_argument(5);

/// Range proof prover (single) failed internally.
#[cfg(feature = "testing")]
const NFE_PROVER_SINGLE_FAILED: u64 = error::cancelled(7);

/// Range proof prover (batch) failed internally.
#[cfg(feature = "testing")]
const NFE_PROVER_BATCH_FAILED: u64 = error::cancelled(10);

/// A compressed ristretto point (a Pedersen commitment) is exactly 32 bytes.
const COMPRESSED_POINT_NUM_BYTES: usize = 32;

/// The Bulletproofs library only supports proving `[0, 2^{num_bits})` ranges
/// where `num_bits` is 8, 16, 32, or 64.
fn is_supported_number_of_bits(num_bits: usize) -> bool {
    matches!(num_bits, 8 | 16 | 32 | 64)
}

/// The Bulletproofs library only supports batch sizes of 1, 2, 4, 8, or 16.
fn is_supported_batch_size(batch_size: usize) -> bool {
    matches!(batch_size, 1 | 2 | 4 | 8 | 16)
}

/// Public parameters of the Bulletproof range proof system, for both individual
/// and batch proving. `party_capacity` is the max batch size.
static BULLETPROOF_GENERATORS: Lazy<BulletproofGens> =
    Lazy::new(|| BulletproofGens::new(MAX_RANGE_BITS, 16));

/// Aborts with `code` and V1's message.
fn abort(code: u64, message: impl Into<String>) -> NativeStatus {
    NativeStatus::Abort {
        code,
        message: Some(message.into()),
    }
}

/// Wraps a 32-byte Pedersen commitment. V1 uses `CompressedRistretto::from_slice`,
/// which panics on a wrong length; the Move signatures only ever pass compressed
/// points, so a mismatch is an invariant violation.
fn compressed_point_from_bytes(bytes: &[u8]) -> VMResult<CompressedRistretto> {
    let slice = <[u8; COMPRESSED_POINT_NUM_BYTES]>::try_from(bytes).map_err(|_| {
        native_invariant_violation("bulletproofs commitment must be 32 bytes".into())
    })?;
    Ok(CompressedRistretto(slice))
}

/// Reads the Pedersen generators named by the two base-point handles.
fn pedersen_gens<C: NativeContext>(
    ctx: &C,
    val_base_handle: u64,
    rand_base_handle: u64,
) -> VMResult<PedersenGens> {
    let store = ctx.get_extension::<RistrettoPointStore>()?;
    Ok(PedersenGens {
        B: store.get(val_base_handle)?,
        B_blinding: store.get(rand_base_handle)?,
    })
}

/// `0x1::ristretto255_bulletproofs::verify_range_proof_internal(
///      com: vector<u8>, val_base: &RistrettoPoint, rand_base: &RistrettoPoint,
///      proof: vector<u8>, num_bits: u64, dst: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_verify_range_proof<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: the args are `vector<u8>`, two `&RistrettoPoint`, `vector<u8>`,
    // `u64`, `vector<u8>`, in that order.
    let (comm_bytes, val_base_handle, rand_base_handle, proof_bytes, num_bits, dst) = unsafe {
        let comm = ctx.arg::<Vector<u8>>(0)?;
        let val_base = point_handle(&ctx.arg::<Ref<u64>>(1)?);
        let rand_base = point_handle(&ctx.arg::<Ref<u64>>(2)?);
        let proof = ctx.arg::<Vector<u8>>(3)?;
        let num_bits = ctx.arg::<u64>(4)? as usize;
        let dst = ctx.arg::<Vector<u8>>(5)?;
        // The byte slices are copied out before any allocation.
        (
            comm.as_bytes().to_vec(),
            val_base,
            rand_base,
            proof.as_bytes().to_vec(),
            num_bits,
            dst.as_bytes().to_vec(),
        )
    };

    if !is_supported_number_of_bits(num_bits) {
        return Ok(abort(
            NFE_RANGE_NOT_SUPPORTED,
            format!("Range of {num_bits} bits is not supported (must be 8, 16, 32, or 64)"),
        ));
    }

    let comm_point = compressed_point_from_bytes(&comm_bytes)?;
    let pg = pedersen_gens(ctx, val_base_handle, rand_base_handle)?;

    let Ok(range_proof) = bulletproofs::RangeProof::from_bytes(&proof_bytes) else {
        return Ok(NativeStatus::Abort {
            code: NFE_DESERIALIZE_RANGE_PROOF,
            message: None,
        });
    };

    let mut transcript = Transcript::new(&dst);
    let success = range_proof
        .verify_single(
            &BULLETPROOF_GENERATORS,
            &pg,
            &mut transcript,
            &comm_point,
            num_bits,
        )
        .is_ok();

    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, success)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255_bulletproofs::verify_batch_range_proof_internal(
///      comms: vector<vector<u8>>, val_base: &RistrettoPoint,
///      rand_base: &RistrettoPoint, proof: vector<u8>, num_bits: u64,
///      dst: vector<u8>): bool`
///
/// Also registered under `0x1::confidential_range_proofs`.
///
/// TODO(metering): charge gas.
pub fn native_verify_batch_range_proof<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: the args are `vector<vector<u8>>`, two `&RistrettoPoint`,
    // `vector<u8>`, `u64`, `vector<u8>`, in that order.
    let (comms, val_base_handle, rand_base_handle, proof_bytes, num_bits, dst) = unsafe {
        let comms = ctx.arg::<Vector<Vector<u8>>>(0)?;
        let val_base = point_handle(&ctx.arg::<Ref<u64>>(1)?);
        let rand_base = point_handle(&ctx.arg::<Ref<u64>>(2)?);
        let proof = ctx.arg::<Vector<u8>>(3)?;
        let num_bits = ctx.arg::<u64>(4)? as usize;
        let dst = ctx.arg::<Vector<u8>>(5)?;
        (
            comms,
            val_base,
            rand_base,
            proof.as_bytes().to_vec(),
            num_bits,
            dst.as_bytes().to_vec(),
        )
    };

    let mut comm_points = Vec::with_capacity(comms.len() as usize);
    for i in 0..comms.len() {
        let comm = comms.get_element(i)?;
        // SAFETY: the slice is copied into an owned point before any allocation.
        comm_points.push(compressed_point_from_bytes(unsafe { comm.as_bytes() })?);
    }

    if !is_supported_number_of_bits(num_bits) {
        return Ok(abort(
            NFE_RANGE_NOT_SUPPORTED,
            format!("Range of {num_bits} bits is not supported (must be 8, 16, 32, or 64)"),
        ));
    }
    if !is_supported_batch_size(comm_points.len()) {
        return Ok(abort(
            NFE_BATCH_SIZE_NOT_SUPPORTED,
            format!(
                "Batch size {} is not supported (must be 1, 2, 4, 8, or 16)",
                comm_points.len()
            ),
        ));
    }

    let pg = pedersen_gens(ctx, val_base_handle, rand_base_handle)?;

    let Ok(range_proof) = bulletproofs::RangeProof::from_bytes(&proof_bytes) else {
        return Ok(NativeStatus::Abort {
            code: NFE_DESERIALIZE_RANGE_PROOF,
            message: None,
        });
    };

    let mut transcript = Transcript::new(&dst);
    let success = range_proof
        .verify_multiple(
            &BULLETPROOF_GENERATORS,
            &pg,
            &mut transcript,
            &comm_points,
            num_bits,
        )
        .is_ok();

    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, success)? };
    Ok(NativeStatus::Success)
}

/// Rejects a value whose scalar encoding sets any bit above the low 64, then
/// reads it back as a `u64`.
#[cfg(feature = "testing")]
fn scalar_to_u64(v: &Scalar) -> Option<u64> {
    let bytes = v.as_bytes();
    bytes[8..]
        .iter()
        .all(|&byte| byte == 0u8)
        .then(|| LittleEndian::read_u64(bytes))
}

/// `0x1::ristretto255_bulletproofs::prove_range_internal(
///      val: vector<u8>, r: vector<u8>, num_bits: u64, dst: vector<u8>,
///      val_base: &RistrettoPoint, rand_base: &RistrettoPoint
///  ): (vector<u8>, vector<u8>)`
///
/// Test-only in Move; charges no gas in V1 either.
#[cfg(feature = "testing")]
pub fn native_prove_range<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: the args are two `vector<u8>`, `u64`, `vector<u8>`, and two
    // `&RistrettoPoint`, in that order.
    let (v, v_blinding, num_bits, dst, val_base_handle, rand_base_handle) = unsafe {
        let val = ctx.arg::<Vector<u8>>(0)?;
        let r = ctx.arg::<Vector<u8>>(1)?;
        let num_bits = ctx.arg::<u64>(2)? as usize;
        let dst = ctx.arg::<Vector<u8>>(3)?;
        let val_base = point_handle(&ctx.arg::<Ref<u64>>(4)?);
        let rand_base = point_handle(&ctx.arg::<Ref<u64>>(5)?);
        (
            scalar_from_bytes(val.as_bytes())?,
            scalar_from_bytes(r.as_bytes())?,
            num_bits,
            dst.as_bytes().to_vec(),
            val_base,
            rand_base,
        )
    };

    if !is_supported_number_of_bits(num_bits) {
        return Ok(abort(
            NFE_RANGE_NOT_SUPPORTED,
            format!("Range of {num_bits} bits is not supported (must be 8, 16, 32, or 64)"),
        ));
    }
    let Some(v) = scalar_to_u64(&v) else {
        return Ok(NativeStatus::Abort {
            code: NFE_VALUE_OUTSIDE_RANGE,
            message: None,
        });
    };

    let pg = pedersen_gens(ctx, val_base_handle, rand_base_handle)?;

    let mut transcript = Transcript::new(&dst);
    let Ok((proof, commitment)) = bulletproofs::RangeProof::prove_single(
        &BULLETPROOF_GENERATORS,
        &pg,
        &mut transcript,
        v,
        &v_blinding,
        num_bits,
    ) else {
        return Ok(abort(
            NFE_PROVER_SINGLE_FAILED,
            "Bulletproofs prover single failed",
        ));
    };

    let proof = ctx.new_byte_vector(&proof.to_bytes())?;
    let commitment = ctx.new_byte_vector(commitment.as_bytes())?;

    // SAFETY: return slots 0 and 1 are both `vector<u8>`.
    unsafe {
        ctx.set_return(0, proof)?;
        ctx.set_return(1, commitment)?;
    }
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255_bulletproofs::prove_batch_range_internal(
///      vals: vector<vector<u8>>, rs: vector<vector<u8>>, num_bits: u64,
///      dst: vector<u8>, val_base: &RistrettoPoint, rand_base: &RistrettoPoint
///  ): (vector<u8>, vector<vector<u8>>)`
///
/// Test-only in Move; charges no gas in V1 either.
#[cfg(feature = "testing")]
pub fn native_prove_batch_range<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: the args are two `vector<vector<u8>>`, `u64`, `vector<u8>`, and
    // two `&RistrettoPoint`, in that order.
    let (vals, rs, num_bits, dst, val_base_handle, rand_base_handle) = unsafe {
        let vals = ctx.arg::<Vector<Vector<u8>>>(0)?;
        let rs = ctx.arg::<Vector<Vector<u8>>>(1)?;
        let num_bits = ctx.arg::<u64>(2)? as usize;
        let dst = ctx.arg::<Vector<u8>>(3)?;
        let val_base = point_handle(&ctx.arg::<Ref<u64>>(4)?);
        let rand_base = point_handle(&ctx.arg::<Ref<u64>>(5)?);
        (
            vals,
            rs,
            num_bits,
            dst.as_bytes().to_vec(),
            val_base,
            rand_base,
        )
    };

    let vs = read_scalars(&vals)?;
    let v_blindings = read_scalars(&rs)?;

    if !is_supported_number_of_bits(num_bits) {
        return Ok(abort(
            NFE_RANGE_NOT_SUPPORTED,
            format!("Range of {num_bits} bits is not supported (must be 8, 16, 32, or 64)"),
        ));
    }
    if !is_supported_batch_size(vs.len()) {
        return Ok(abort(
            NFE_BATCH_SIZE_NOT_SUPPORTED,
            format!(
                "Batch size {} is not supported (must be 1, 2, 4, 8, or 16)",
                vs.len()
            ),
        ));
    }
    if vs.len() != v_blindings.len() {
        return Ok(abort(
            NFE_VECTOR_LENGTHS_MISMATCH,
            format!(
                "Number of committed values ({}) must equal number of blinding factors ({})",
                vs.len(),
                v_blindings.len()
            ),
        ));
    }

    let Some(vs) = vs.iter().map(scalar_to_u64).collect::<Option<Vec<_>>>() else {
        return Ok(NativeStatus::Abort {
            code: NFE_VALUE_OUTSIDE_RANGE,
            message: None,
        });
    };

    let pg = pedersen_gens(ctx, val_base_handle, rand_base_handle)?;

    let mut transcript = Transcript::new(&dst);
    let Ok((proof, commitments)) = bulletproofs::RangeProof::prove_multiple(
        &BULLETPROOF_GENERATORS,
        &pg,
        &mut transcript,
        &vs,
        &v_blindings,
        num_bits,
    ) else {
        return Ok(abort(
            NFE_PROVER_BATCH_FAILED,
            "Bulletproofs prover batch failed",
        ));
    };

    let proof = ctx.new_byte_vector(&proof.to_bytes())?;
    let commitments = commitments
        .iter()
        .map(|c| c.as_bytes().as_slice())
        .collect::<Vec<_>>();
    let commitments = ctx.new_byte_vector_vector(&commitments)?;

    // SAFETY: return slot 0 is `vector<u8>`, slot 1 is `vector<vector<u8>>`.
    unsafe {
        ctx.set_return(0, proof)?;
        ctx.set_return(1, commitments)?;
    }
    Ok(NativeStatus::Success)
}

/// Reads a `vector<vector<u8>>` of 32-byte scalar encodings.
#[cfg(feature = "testing")]
fn read_scalars<'a>(vec: &Vector<'a, Vector<'a, u8>>) -> VMResult<Vec<Scalar>> {
    (0..vec.len())
        .map(|i| {
            let elem = vec.get_element(i)?;
            // SAFETY: the slice is consumed into an owned scalar before any
            // allocation.
            scalar_from_bytes(unsafe { elem.as_bytes() })
        })
        .collect()
}

/// Production bulletproofs natives.
pub fn make_all_bulletproofs_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::ristretto255_bulletproofs::verify_range_proof_internal",
            native_verify_range_proof
        ),
        (
            "0x1::ristretto255_bulletproofs::verify_batch_range_proof_internal",
            native_verify_batch_range_proof
        ),
        // The confidential-asset framework declares its own native binding to
        // the same implementation.
        (
            "0x1::confidential_range_proofs::verify_batch_range_proof_internal",
            native_verify_batch_range_proof
        ),
    ]
}

/// Test-only bulletproofs natives (the provers).
#[cfg(feature = "testing")]
pub fn make_all_bulletproofs_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::ristretto255_bulletproofs::prove_range_internal",
            native_prove_range
        ),
        (
            "0x1::ristretto255_bulletproofs::prove_batch_range_internal",
            native_prove_batch_range
        ),
    ]
}
