// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_gas_schedule::gas_params::natives::{
    aptos_framework::*,
    move_stdlib::{HASH_SHA3_256_BASE, HASH_SHA3_256_PER_BYTE},
};
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_core_types::gas_algebra::NumBytes;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use sha3::Digest;
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Equals `error::invalid_argument(EINVALID_INITIALIZE_CALLER)` for the constant
/// declared in `init.move`.
const EINVALID_INITIALIZE_CALLER: u64 = 0x1_0001;

/***************************************************************************************************
 * native fun get_caller_address_and_module_id
 *
 *   Returns the address and module id of the module that directly called the Move function
 *   invoking this native. Aborts with EINVALID_INITIALIZE_CALLER if the caller has no
 *   associated module (e.g. a script).
 *
 *   gas cost: base_cost + sha3-256 cost of the module name (charged via the move-stdlib
 *             hash parameters, so it stays aligned with `std::hash::sha3_256`)
 *
 **************************************************************************************************/
fn native_get_caller_address_and_module_id(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(INIT_GET_CALLER_ADDRESS_AND_MODULE_ID_BASE)?;

    // Check the runtime call site as well as publication: old bytecode and stored closures can
    // predate the verifier's restriction on function values over `internal_maybe_initialize`.
    let caller_module_id = context.direct_caller_module().ok_or_else(|| {
        SafeNativeError::abort_with_message(
            EINVALID_INITIALIZE_CALLER,
            "initializer must be invoked by a direct module call",
        )
    })?;

    let name_bytes = caller_module_id.name().as_bytes();
    context.charge(
        HASH_SHA3_256_BASE + HASH_SHA3_256_PER_BYTE * NumBytes::new(name_bytes.len() as u64),
    )?;

    // Must produce the same value as `init::module_id_from_name`: the sha3-256 of the
    // module name bytes, trimmed to 16 bytes, read as a (BCS) little-endian u128.
    let hash = sha3::Sha3_256::digest(name_bytes);
    let module_id_hash = u128::from_le_bytes(
        hash[..16]
            .try_into()
            .expect("sha3-256 digest has at least 16 bytes"),
    );

    Ok(smallvec![
        Value::address(*caller_module_id.address()),
        Value::struct_(Struct::pack(vec![Value::u128(module_id_hash)]))
    ])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [(
        "get_caller_address_and_module_id",
        native_get_caller_address_and_module_id as RawSafeNative,
    )];

    builder.make_named_natives(natives)
}
