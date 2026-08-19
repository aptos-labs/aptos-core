// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `consensus_config` module.

use crate::{monomorphic_natives, NativeEntry};
use aptos_types::on_chain_config::OnChainConsensusConfig;
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};

/// `0x1::consensus_config::validator_txn_enabled_internal(config_bytes: vector<u8>): bool`
///
/// BCS-decodes an `OnChainConsensusConfig` from `config_bytes` and returns
/// whether validator transactions are enabled. A malformed encoding falls back
/// to the default config, matching the legacy VM's `unwrap_or_default()`.
//
// TODO(metering): charge gas.
pub fn native_validator_txn_enabled<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`, passed by value.
    let v: Vector<u8> = unsafe { ctx.arg(0)? };
    // Copy off the VM heap first: deserialization allocates and may relocate it.
    // SAFETY: the bytes are copied immediately, before any allocation.
    let bytes = unsafe { v.as_bytes() }.to_vec();
    let config = bcs::from_bytes::<OnChainConsensusConfig>(&bytes).unwrap_or_default();
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, config.is_vtxn_enabled())? };
    Ok(NativeStatus::Success)
}

/// Natives for the `consensus_config` module.
pub fn make_all_consensus_config_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![(
        "0x1::consensus_config::validator_txn_enabled_internal",
        native_validator_txn_enabled
    )]
}
