// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `init` module, backing lazy module initialization.

use crate::{monomorphic_natives, NativeEntry};
use aptos_types::error;
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus},
    types::view_name,
    view_module_id, VMResult,
};
use sha3::{Digest, Sha3_256};

/// Matches the Move-side `EINVALID_INITIALIZE_CALLER` in `0x1::init.move`.
const EINVALID_INITIALIZE_CALLER: u64 = error::invalid_argument(1);

/// `0x1::init::get_caller_address_and_module_id(): (address, ModuleId)`
///
/// Returns the address and module id of the module that directly called the
/// function invoking this native. Aborts if there is no such module (e.g. a
/// script). The module id is the sha3-256 of the module name, trimmed to 16
/// bytes. INVARIANT: Must match implemention in `0x1::init.move`.
//
// TODO(metering): charge gas.
pub fn native_get_caller_address_and_module_id<C: NativeContext>(
    ctx: &C,
) -> VMResult<NativeStatus> {
    let Some(module_id) = ctx.caller_module() else {
        return Ok(NativeStatus::Abort {
            code: EINVALID_INITIALIZE_CALLER,
            message: Some("caller has no associated module (e.g. a script)".into()),
        });
    };
    let module_id = view_module_id(module_id);
    let address = *module_id.address();
    let name = view_name(module_id.name());
    let hash = Sha3_256::digest(name.as_bytes());
    let module_id_hash = u128::from_le_bytes(
        hash[..16]
            .try_into()
            .expect("sha3-256 digest has at least 16 bytes"),
    );

    // SAFETY: return 0 is `address`; return 1 is `ModuleId { hash: u128 }`,
    // which has same representation as u128.
    unsafe { ctx.set_return(0, address)? };
    unsafe { ctx.set_return(1, module_id_hash)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `init` module.
pub fn make_all_init_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![(
        "0x1::init::get_caller_address_and_module_id",
        native_get_caller_address_and_module_id
    ),]
}
