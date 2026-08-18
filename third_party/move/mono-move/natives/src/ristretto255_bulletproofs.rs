// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bulletproofs range-proof natives for the `ristretto255_bulletproofs` module
//! (`aptos_std::ristretto255_bulletproofs`) and the batch-verify native
//! re-registered by `aptos_framework::confidential_range_proofs`.
//!
//! Base points cross the boundary as `&RistrettoPoint` handles into the
//! [`RistrettoPointStore`] (see `ristretto255_point.rs`), never as bytes. The
//! Pedersen commitment key is `PedersenGens { B: val_base, B_blinding:
//! rand_base }`, and the transcript label is the caller-supplied domain
//! separation tag, matching V1.

#[cfg(feature = "testing")]
use crate::ristretto255_scalar::scalar_from_bytes;
use crate::{
    monomorphic_natives,
    ristretto255_point::{point_handle, RistrettoPointStore},
    NativeEntry,
};
use aptos_crypto::bulletproofs::MAX_RANGE_BITS;
use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek::ristretto::CompressedRistretto;
use merlin::Transcript;
#[cfg(feature = "testing")]
use mono_move_core::native::native_invariant_violation;
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Ref, Vector},
    VMResult,
};
use std::sync::LazyLock;

/// Abort codes raised by the range-proof natives. Each must match the code in
/// the Move implementation (`ristretto255_bulletproofs.move`).
mod abort_codes {
    /// Deserialization of the range proof failed (`0x01` == INVALID_ARGUMENT).
    pub const NFE_DESERIALIZE_RANGE_PROOF: u64 = 0x01_0001;
    /// The input value for a range proof is too large.
    pub const NFE_VALUE_OUTSIDE_RANGE: u64 = 0x01_0002;
    /// The requested range is larger than the maximum supported one.
    pub const NFE_RANGE_NOT_SUPPORTED: u64 = 0x01_0003;
    /// The requested batch size is larger than the maximum supported one.
    pub const NFE_BATCH_SIZE_NOT_SUPPORTED: u64 = 0x01_0004;
    /// The value and blinding-factor vectors have mismatched lengths.
    pub const NFE_VECTOR_LENGTHS_MISMATCH: u64 = 0x01_0005;
    /// The range proof prover (single) failed internally.
    pub const NFE_PROVER_SINGLE_FAILED: u64 = 0x0A_0007;
    /// The range proof prover (batch) failed internally.
    pub const NFE_PROVER_BATCH_FAILED: u64 = 0x0A_000A;
}

/// The Bulletproofs library only supports proving `[0, 2^num_bits)` ranges for
/// `num_bits` in `{8, 16, 32, 64}`.
fn is_supported_number_of_bits(num_bits: u64) -> bool {
    matches!(num_bits, 8 | 16 | 32 | 64)
}

/// The Bulletproofs library only supports batch sizes in `{1, 2, 4, 8, 16}`.
fn is_supported_batch_size(batch_size: u64) -> bool {
    matches!(batch_size, 1 | 2 | 4 | 8 | 16)
}

/// Public parameters of the Bulletproof range proof system, for both individual
/// and batch proving. The second argument is the max batch size.
static BULLETPROOF_GENERATORS: LazyLock<BulletproofGens> =
    LazyLock::new(|| BulletproofGens::new(MAX_RANGE_BITS, 16));

/// The unsupported-range abort, carrying the same message as V1.
fn range_not_supported(num_bits: u64) -> NativeStatus {
    NativeStatus::Abort {
        code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        message: Some(format!(
            "Range of {} bits is not supported (must be 8, 16, 32, or 64)",
            num_bits
        )),
    }
}

/// The unsupported-batch-size abort, carrying the same message as V1.
fn batch_size_not_supported(batch_size: u64) -> NativeStatus {
    NativeStatus::Abort {
        code: abort_codes::NFE_BATCH_SIZE_NOT_SUPPORTED,
        message: Some(format!(
            "Batch size {} is not supported (must be 1, 2, 4, 8, or 16)",
            batch_size
        )),
    }
}

/// Builds the Pedersen commitment key from the two base-point handles. An
/// out-of-bounds handle is unreachable with well-typed Move, so [`get`] surfaces
/// it as an invariant violation.
///
/// [`get`]: RistrettoPointStore::get
fn pedersen_gens<C: NativeContext>(
    ctx: &C,
    val_base: u64,
    rand_base: u64,
) -> VMResult<PedersenGens> {
    let store = ctx.get_extension::<RistrettoPointStore>()?;
    Ok(PedersenGens {
        B: store.get(val_base)?,
        B_blinding: store.get(rand_base)?,
    })
}

/// `0x1::ristretto255_bulletproofs::verify_range_proof_internal(com:
/// vector<u8>, val_base: &RistrettoPoint, rand_base: &RistrettoPoint, proof:
/// vector<u8>, num_bits: u64, dst: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_verify_range_proof<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`, args 1, 2 are `&RistrettoPoint`, arg 3 is
    // `vector<u8>`, arg 4 is `u64`, arg 5 is `vector<u8>`.
    let comm_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    let val_base = unsafe { ctx.arg::<Ref<u64>>(1)? };
    let rand_base = unsafe { ctx.arg::<Ref<u64>>(2)? };
    let proof_vec: Vector<u8> = unsafe { ctx.arg(3)? };
    let num_bits = unsafe { ctx.arg::<u64>(4)? };
    let dst_vec: Vector<u8> = unsafe { ctx.arg(5)? };

    // Copy the byte args out before touching the store or allocating.
    // SAFETY: each slice is consumed into an owned value immediately.
    let comm_point = CompressedRistretto::from_slice(unsafe { comm_vec.as_bytes() });
    let proof_bytes = unsafe { proof_vec.as_bytes() }.to_vec();
    let dst = unsafe { dst_vec.as_bytes() }.to_vec();
    let val_base = point_handle(&val_base);
    let rand_base = point_handle(&rand_base);

    if !is_supported_number_of_bits(num_bits) {
        return Ok(range_not_supported(num_bits));
    }

    let pc_gens = pedersen_gens(ctx, val_base, rand_base)?;

    let range_proof = match RangeProof::from_bytes(&proof_bytes) {
        Ok(proof) => proof,
        Err(_) => {
            return Ok(NativeStatus::Abort {
                code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
                message: None,
            })
        },
    };

    let mut transcript = Transcript::new(&dst);
    let success = range_proof
        .verify_single(
            &BULLETPROOF_GENERATORS,
            &pc_gens,
            &mut transcript,
            &comm_point,
            num_bits as usize,
        )
        .is_ok();

    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, success)? };
    Ok(NativeStatus::Success)
}

/// ```text
/// 0x1::ristretto255_bulletproofs::verify_batch_range_proof_internal(
///   comms: vector<vector<u8>>,
///   val_base: &RistrettoPoint,
///   rand_base: &RistrettoPoint,
///   proof: vector<u8>,
///   num_bits: u64,
///   dst: vector<u8>,
/// ): bool
/// ```
///
/// Also registered under `0x1::confidential_range_proofs`, which re-exports the
/// same native.
///
/// TODO(metering): charge gas.
pub fn native_verify_batch_range_proof<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<vector<u8>>`, args 1, 2 are `&RistrettoPoint`,
    // arg 3 is `vector<u8>`, arg 4 is `u64`, arg 5 is `vector<u8>`.
    let comms: Vector<Vector<u8>> = unsafe { ctx.arg(0)? };
    let val_base = unsafe { ctx.arg::<Ref<u64>>(1)? };
    let rand_base = unsafe { ctx.arg::<Ref<u64>>(2)? };
    let proof_vec: Vector<u8> = unsafe { ctx.arg(3)? };
    let num_bits = unsafe { ctx.arg::<u64>(4)? };
    let dst_vec: Vector<u8> = unsafe { ctx.arg(5)? };

    // Copy the commitments out before touching the store or allocating.
    let batch_size = comms.len();
    let mut comm_points = Vec::with_capacity(batch_size as usize);
    for i in 0..batch_size {
        let comm = comms.get(i);
        // SAFETY: each inner byte slice is consumed immediately.
        comm_points.push(CompressedRistretto::from_slice(unsafe { comm.as_bytes() }));
    }
    let proof_bytes = unsafe { proof_vec.as_bytes() }.to_vec();
    let dst = unsafe { dst_vec.as_bytes() }.to_vec();
    let val_base = point_handle(&val_base);
    let rand_base = point_handle(&rand_base);

    if !is_supported_number_of_bits(num_bits) {
        return Ok(range_not_supported(num_bits));
    }
    if !is_supported_batch_size(batch_size) {
        return Ok(batch_size_not_supported(batch_size));
    }

    let pc_gens = pedersen_gens(ctx, val_base, rand_base)?;

    let range_proof = match RangeProof::from_bytes(&proof_bytes) {
        Ok(proof) => proof,
        Err(_) => {
            return Ok(NativeStatus::Abort {
                code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
                message: None,
            })
        },
    };

    let mut transcript = Transcript::new(&dst);
    let success = range_proof
        .verify_multiple(
            &BULLETPROOF_GENERATORS,
            &pc_gens,
            &mut transcript,
            &comm_points,
            num_bits as usize,
        )
        .is_ok();

    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, success)? };
    Ok(NativeStatus::Success)
}

/// ```text
/// 0x1::ristretto255_bulletproofs::prove_range_internal(
///   val: vector<u8>, r:
///   vector<u8>,
///   num_bits: u64,
///   dst: vector<u8>,
///   val_base: &RistrettoPoint,
///   rand_base: &RistrettoPoint,
/// ): (vector<u8>, vector<u8>)
/// ```
///
/// Test-only native returning `(proof_bytes, commitment_bytes)`. Proving mixes
/// randomness into the transcript, so the proof bytes are not reproducible;
/// only the deterministic verify path is compared across VMs.
///
/// TODO(metering): charge gas.
#[cfg(feature = "testing")]
pub fn native_prove_range<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0, 1 are `vector<u8>`, arg 2 is `u64`, arg 3 is `vector<u8>`,
    // args 4, 5 are `&RistrettoPoint`.
    let val_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    let r_vec: Vector<u8> = unsafe { ctx.arg(1)? };
    let num_bits = unsafe { ctx.arg::<u64>(2)? };
    let dst_vec: Vector<u8> = unsafe { ctx.arg(3)? };
    let val_base = unsafe { ctx.arg::<Ref<u64>>(4)? };
    let rand_base = unsafe { ctx.arg::<Ref<u64>>(5)? };

    // SAFETY: each slice is consumed into an owned value immediately.
    let v_scalar = scalar_from_bytes(unsafe { val_vec.as_bytes() })?;
    let v_blinding = scalar_from_bytes(unsafe { r_vec.as_bytes() })?;
    let dst = unsafe { dst_vec.as_bytes() }.to_vec();
    let val_base = point_handle(&val_base);
    let rand_base = point_handle(&rand_base);

    if !is_supported_number_of_bits(num_bits) {
        return Ok(range_not_supported(num_bits));
    }
    // Only the low 64 bits of the value may be set.
    if !v_scalar.as_bytes()[8..].iter().all(|&byte| byte == 0u8) {
        return Ok(NativeStatus::Abort {
            code: abort_codes::NFE_VALUE_OUTSIDE_RANGE,
            message: None,
        });
    }
    let v = u64::from_le_bytes(
        v_scalar.as_bytes()[..8]
            .try_into()
            .expect("8-byte prefix of a 32-byte scalar"),
    );

    let pc_gens = pedersen_gens(ctx, val_base, rand_base)?;
    let mut transcript = Transcript::new(&dst);
    let (proof, commitment) = match RangeProof::prove_single(
        &BULLETPROOF_GENERATORS,
        &pc_gens,
        &mut transcript,
        v,
        &v_blinding,
        num_bits as usize,
    ) {
        Ok(result) => result,
        Err(_) => {
            return Ok(NativeStatus::Abort {
                code: abort_codes::NFE_PROVER_SINGLE_FAILED,
                message: Some("Bulletproofs prover single failed".to_string()),
            })
        },
    };

    let proof_out = ctx.new_byte_vector(&proof.to_bytes())?;
    let comm_out = ctx.new_byte_vector(commitment.as_bytes())?;
    // SAFETY: return slots 0 and 1 are `vector<u8>`.
    unsafe {
        ctx.set_return(0, proof_out)?;
        ctx.set_return(1, comm_out)?;
    }
    Ok(NativeStatus::Success)
}

/// ```text
/// 0x1::ristretto255_bulletproofs::prove_batch_range_internal(
///   vals: vector<vector<u8>>,
///   rs: vector<vector<u8>>,
///   num_bits: u64,
///   dst: vector<u8>,
///   val_base: &RistrettoPoint,
///   rand_base: &RistrettoPoint,
/// ): (vector<u8>, vector<vector<u8>>)
/// ```
///
/// Test-only native returning `(proof_bytes, commitments)`. As with the single
/// prover, proving mixes randomness into the transcript, so only the
/// deterministic verify path is compared across VMs.
///
/// TODO(metering): charge gas.
#[cfg(feature = "testing")]
pub fn native_prove_batch_range<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0, 1 are `vector<vector<u8>>`, arg 2 is `u64`, arg 3 is
    // `vector<u8>`, args 4, 5 are `&RistrettoPoint`.
    let vals: Vector<Vector<u8>> = unsafe { ctx.arg(0)? };
    let rs: Vector<Vector<u8>> = unsafe { ctx.arg(1)? };
    let num_bits = unsafe { ctx.arg::<u64>(2)? };
    let dst_vec: Vector<u8> = unsafe { ctx.arg(3)? };
    let val_base = unsafe { ctx.arg::<Ref<u64>>(4)? };
    let rand_base = unsafe { ctx.arg::<Ref<u64>>(5)? };

    // Copy every input out into owned values before touching the store or
    // allocating a return value.
    let mut v_scalars = Vec::with_capacity(vals.len() as usize);
    for i in 0..vals.len() {
        let v = vals.get(i);
        // SAFETY: each inner byte slice is consumed into an owned scalar immediately.
        v_scalars.push(scalar_from_bytes(unsafe { v.as_bytes() })?);
    }
    let mut v_blindings = Vec::with_capacity(rs.len() as usize);
    for i in 0..rs.len() {
        let r = rs.get(i);
        // SAFETY: each inner byte slice is consumed into an owned scalar immediately.
        v_blindings.push(scalar_from_bytes(unsafe { r.as_bytes() })?);
    }
    let dst = unsafe { dst_vec.as_bytes() }.to_vec();
    let val_base = point_handle(&val_base);
    let rand_base = point_handle(&rand_base);

    if !is_supported_number_of_bits(num_bits) {
        return Ok(range_not_supported(num_bits));
    }
    let batch_size = v_scalars.len() as u64;
    if !is_supported_batch_size(batch_size) {
        return Ok(batch_size_not_supported(batch_size));
    }
    if v_scalars.len() != v_blindings.len() {
        return Ok(NativeStatus::Abort {
            code: abort_codes::NFE_VECTOR_LENGTHS_MISMATCH,
            message: Some(format!(
                "Number of committed values ({}) must equal number of blinding factors ({})",
                v_scalars.len(),
                v_blindings.len()
            )),
        });
    }
    // Only the low 64 bits of each value may be set.
    if !v_scalars
        .iter()
        .all(|v| v.as_bytes()[8..].iter().all(|&byte| byte == 0u8))
    {
        return Ok(NativeStatus::Abort {
            code: abort_codes::NFE_VALUE_OUTSIDE_RANGE,
            message: None,
        });
    }
    let values = v_scalars
        .iter()
        .map(|v| {
            u64::from_le_bytes(
                v.as_bytes()[..8]
                    .try_into()
                    .expect("8-byte prefix of a 32-byte scalar"),
            )
        })
        .collect::<Vec<u64>>();

    let pc_gens = pedersen_gens(ctx, val_base, rand_base)?;
    let mut transcript = Transcript::new(&dst);
    let (proof, commitments) = match RangeProof::prove_multiple(
        &BULLETPROOF_GENERATORS,
        &pc_gens,
        &mut transcript,
        &values,
        &v_blindings,
        num_bits as usize,
    ) {
        Ok(result) => result,
        Err(_) => {
            return Ok(NativeStatus::Abort {
                code: abort_codes::NFE_PROVER_BATCH_FAILED,
                message: Some("Bulletproofs prover batch failed".to_string()),
            })
        },
    };

    // The commitments return is a `vector<vector<u8>>`; the specializer
    // publishes its descriptor as the native's first required descriptor.
    let descriptor = ctx.required_descriptor(0).ok_or_else(|| {
        native_invariant_violation("prove_batch_range: missing commitments descriptor".into())
    })?;
    let proof_out = ctx.new_byte_vector(&proof.to_bytes())?;
    let comm_slices = commitments
        .iter()
        .map(|c| c.as_bytes().as_slice())
        .collect::<Vec<_>>();
    let comms_out = ctx.new_byte_vector_vector(descriptor, &comm_slices)?;
    // SAFETY: return slot 0 is `vector<u8>`, return slot 1 is `vector<vector<u8>>`.
    unsafe {
        ctx.set_return(0, proof_out)?;
        ctx.set_return(1, comms_out)?;
    }
    Ok(NativeStatus::Success)
}

/// Production range-proof natives for the `ristretto255_bulletproofs` module,
/// plus the batch-verify native re-registered under `confidential_range_proofs`.
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
        (
            "0x1::confidential_range_proofs::verify_batch_range_proof_internal",
            native_verify_batch_range_proof
        ),
    ]
}

/// Test-only range-proof natives for the `ristretto255_bulletproofs` module:
/// the single-value and batch provers.
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
