// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `secp256k1` module (`aptos_std::secp256k1`).

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};

/// Abort code on deserialization failure (0x01 == INVALID_ARGUMENT). Must match
/// the V1 native and the Move `E_DESERIALIZE` code.
const NFE_DESERIALIZE: u64 = 0x01_0001;

/// `0x1::secp256k1::ecdsa_recover_internal(message: vector<u8>, recovery_id: u8,
/// signature: vector<u8>): (vector<u8>, bool)`
///
/// Recovers the signer's 64-byte public key from an ECDSA signature. Returns
/// `(public_key, true)` on success and `([], false)` when recovery fails.
/// Aborts with `NFE_DESERIALIZE` when an input cannot be deserialized.
//
// TODO(metering): charge gas.
pub fn native_ecdsa_recover<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let message_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: arg 1 is `u8`.
    let recovery_id: u8 = unsafe { ctx.arg(1)? };
    // SAFETY: arg 2 is `vector<u8>`.
    let signature_vec: Vector<u8> = unsafe { ctx.arg(2)? };

    // SAFETY: bytes reference dropped before any allocation.
    let msg = match libsecp256k1::Message::parse_slice(unsafe { message_vec.as_bytes() }) {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(NativeStatus::Abort {
                code: NFE_DESERIALIZE,
                message: Some("Message must be exactly 32 bytes".to_string()),
            })
        },
    };
    let rid = match libsecp256k1::RecoveryId::parse(recovery_id) {
        Ok(rid) => rid,
        Err(_) => {
            return Ok(NativeStatus::Abort {
                code: NFE_DESERIALIZE,
                message: Some("Recovery ID must be 0, 1, 2, or 3".to_string()),
            })
        },
    };
    // SAFETY: bytes reference dropped before any allocation.
    let sig =
        match libsecp256k1::Signature::parse_standard_slice(unsafe { signature_vec.as_bytes() }) {
            Ok(sig) => sig,
            Err(_) => {
                return Ok(NativeStatus::Abort {
                    code: NFE_DESERIALIZE,
                    message: Some("Signature must be exactly 64 bytes".to_string()),
                })
            },
        };

    match libsecp256k1::recover(&msg, &sig, &rid) {
        Ok(pk) => {
            // Drop the leading 0x04 tag byte to keep the 64-byte raw key.
            let out = ctx.new_byte_vector(&pk.serialize()[1..])?;
            // SAFETY: return 0 is `vector<u8>`.
            unsafe { ctx.set_return(0, out)? };
            // SAFETY: return 1 is `bool`.
            unsafe { ctx.set_return(1, true)? };
        },
        Err(_) => {
            let out = ctx.new_byte_vector(&[])?;
            // SAFETY: return 0 is `vector<u8>`.
            unsafe { ctx.set_return(0, out)? };
            // SAFETY: return 1 is `bool`.
            unsafe { ctx.set_return(1, false)? };
        },
    }
    Ok(NativeStatus::Success)
}

/// Natives for the `secp256k1` module.
pub fn make_all_secp256k1_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![(
        "0x1::secp256k1::ecdsa_recover_internal",
        native_ecdsa_recover
    ),]
}
