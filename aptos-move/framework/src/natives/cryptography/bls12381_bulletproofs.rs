// Copyright (c) 2024 Supra.

use aptos_crypto::bulletproofs::MAX_RANGE_BITS;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use bulletproofs_bls12381::{BulletproofGens, PedersenGens, RangeProof};
use merlin::Transcript;
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value},
};
use once_cell::sync::Lazy;
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;
use blsttc::G1Projective;
use crate::natives::cryptography::bulletproofs::abort_codes;

/// The Bulletproofs library only seems to support proving [0, 2^{num_bits}) ranges where num_bits is
/// either 8, 16, 32 or 64.
fn is_supported_number_of_bits(num_bits: usize) -> bool {
    matches!(num_bits, 8 | 16 | 32 | 64)
}

fn deserialize_g1(vec: Vec<u8>) -> Result<G1Projective, ()> {
    if vec.len() != 48 {
        return Err(());
    }
    let array: [u8; 48] = vec
        .try_into()
        .map_err(|_| ())?;

    let g1_option = G1Projective::from_compressed(&array);

    if g1_option.is_some().unwrap_u8() == 1 {
        Ok(g1_option.unwrap())
    } else {
        Err(())
    }
}

/// Public parameters of the Bulletproof range proof system
static BULLETPROOF_GENERATORS: Lazy<BulletproofGens> =
    Lazy::new(|| BulletproofGens::new(MAX_RANGE_BITS, 1));

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
    let rand_base_bytes = safely_pop_arg!(args, Vec<u8>);
    let val_base_bytes = safely_pop_arg!(args, Vec<u8>);
    let comm_bytes = safely_pop_arg!(args, Vec<u8>);

    if !is_supported_number_of_bits(num_bits) {
        return Err(SafeNativeError::Abort {
            abort_code: abort_codes::NFE_RANGE_NOT_SUPPORTED,
        });
    }

    let comm_point = deserialize_g1(comm_bytes).map_err(|_| SafeNativeError::Abort {
        abort_code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
    })?;

    let pg = {

        let rand_base = deserialize_g1(rand_base_bytes).map_err(|_| SafeNativeError::Abort {
            abort_code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
        })?;
        let val_base = deserialize_g1(val_base_bytes).map_err(|_| SafeNativeError::Abort {
            abort_code: abort_codes::NFE_DESERIALIZE_RANGE_PROOF,
        })?;

        PedersenGens {
            B: val_base,
            B_blinding: rand_base,
        }
    };

    verify_range_proof(context, &comm_point, &pg, &proof_bytes[..], num_bits, dst)
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
/// Helper function to gas meter and verify a single Bulletproof range proof for a Pedersen
/// commitment with `pc_gens` as its commitment key.
fn verify_range_proof(
    context: &mut SafeNativeContext,
    comm_point: &G1Projective,
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

    let range_proof = match RangeProof::from_bytes(proof_bytes) {
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

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([(
        "verify_range_proof_internal",
        native_verify_range_proof as RawSafeNative,
    )]);

    builder.make_named_natives(natives)
}
