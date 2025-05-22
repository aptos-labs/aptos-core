// Copyright (c) 2024 Supra.

use std::collections::VecDeque;
use smallvec::{smallvec, SmallVec};
use blst::blst_scalar;
use aptos_gas_schedule::gas_params::natives::aptos_framework::{HASH_KECCAK256_BASE, HASH_KECCAK256_PER_BYTE};
use aptos_native_interface::{safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult};
use move_core_types::gas_algebra::{NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::values::Value;
use blsttc::Fr;
use blsttc::group::ff::Field;

/// Native function for computing hash to scalar for BLS12-381.
///
/// # Arguments
///
///   1. `dst`: Domain String (`Vec<u8>`)
///   2. `msg`: Input data (`Vec<u8>`)
///
/// Returns a vector<u8> representing the byte representation of bls12381_scalar
/// In case an error occurs, an empty vector is returned
///
fn native_hash_to_scalar(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {

    let msg: Vec<u8> = safely_pop_arg!(arguments, Vec<u8>);
    let dst: Vec<u8> = safely_pop_arg!(arguments, Vec<u8>);

    let cost = HASH_KECCAK256_BASE
        + HASH_KECCAK256_PER_BYTE * (NumBytes::new(msg.len() as u64) + NumBytes::new(dst.len() as u64));

    context.charge(cost)?;

    let scalar_fr: Fr;
    if let Some(scalar) = blst_scalar::hash_to(&msg, &dst){
        if let Ok(fr_scalar) = scalar.try_into(){
            scalar_fr = fr_scalar;
        }
        else {
            scalar_fr = Fr::zero();
        }
    }
    else {
        scalar_fr = Fr::zero();
    }

    Ok(smallvec![Value::vector_u8(scalar_fr.to_bytes_le())])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([
        (
            "native_hash_to_scalar",
            native_hash_to_scalar as RawSafeNative,
        ),
    ]);

    builder.make_named_natives(natives)
}
