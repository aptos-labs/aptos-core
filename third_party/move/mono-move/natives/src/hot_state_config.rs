// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `hot_state_config` module.

use crate::{monomorphic_natives, NativeEntry};
use aptos_types::on_chain_config::OnChainHotStateConfig;
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};

/// `0x1::hot_state_config::validate_internal(config: vector<u8>): bool`
///
/// Returns whether `config` BCS-decodes to a valid `OnChainHotStateConfig`.
/// Not metered, matching the legacy VM: only the framework signer can reach it.
pub fn native_validate<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`, passed by value.
    let v: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: the bytes are copied immediately, before any allocation.
    let bytes = unsafe { v.as_bytes() }.to_vec();
    let valid = bcs::from_bytes::<OnChainHotStateConfig>(&bytes)
        .is_ok_and(|config| config.validate().is_ok());
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `hot_state_config` module.
pub fn make_all_hot_state_config_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![("0x1::hot_state_config::validate_internal", native_validate)]
}
