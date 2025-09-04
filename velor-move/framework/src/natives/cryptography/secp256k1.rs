// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::gas_algebra::NumArgs;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Abort code when deserialization fails (0x01 == INVALID_ARGUMENT)
/// NOTE: This must match the code in the Move implementation
pub mod abort_codes {
    pub const NFE_DESERIALIZE: u64 = 0x01_0001;
}

/***************************************************************************************************
 * native fun secp256k1_recover
 *
 *   gas cost: base_cost +? ecdsa_recover
 *
 **************************************************************************************************/
fn native_ecdsa_recover(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let signature = safely_pop_arg!(arguments, Vec<u8>);
    let recovery_id = safely_pop_arg!(arguments, u8);
    let msg = safely_pop_arg!(arguments, Vec<u8>);

    context.charge(SECP256K1_BASE)?;

    // NOTE(Gas): O(1) cost
    // (In reality, O(|msg|) deserialization cost, with |msg| < libsecp256k1_core::util::MESSAGE_SIZE
    // which seems to be 32 bytes, so O(1) cost for all intents and purposes.)
    let msg = match libsecp256k1::Message::parse_slice(&msg) {
        Ok(msg) => msg,
        Err(_) => {
            return Err(SafeNativeError::Abort {
                abort_code: abort_codes::NFE_DESERIALIZE,
            });
        },
    };

    // NOTE(Gas): O(1) cost
    let rid = match libsecp256k1::RecoveryId::parse(recovery_id) {
        Ok(rid) => rid,
        Err(_) => {
            return Err(SafeNativeError::Abort {
                abort_code: abort_codes::NFE_DESERIALIZE,
            });
        },
    };

    // NOTE(Gas): O(1) deserialization cost
    // which seems to be 64 bytes, so O(1) cost for all intents and purposes.
    let sig = match libsecp256k1::Signature::parse_standard_slice(&signature) {
        Ok(sig) => sig,
        Err(_) => {
            return Err(SafeNativeError::Abort {
                abort_code: abort_codes::NFE_DESERIALIZE,
            });
        },
    };

    context.charge(SECP256K1_ECDSA_RECOVER * NumArgs::one())?;

    // NOTE(Gas): O(1) cost: a size-2 multi-scalar multiplication
    match libsecp256k1::recover(&msg, &sig, &rid) {
        Ok(pk) => Ok(smallvec![
            Value::vector_u8(pk.serialize()[1..].to_vec()),
            Value::bool(true)
        ]),
        Err(_) => Ok(smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)]),
    }
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [(
        "ecdsa_recover_internal",
        native_ecdsa_recover as RawSafeNative,
    )];

    builder.make_named_natives(natives)
}
