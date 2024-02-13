// Copyright © Aptos Foundation

use std::collections::VecDeque;
use smallvec::{SmallVec, smallvec};
use aptos_native_interface::{RawSafeNative, safely_pop_arg, SafeNativeBuilder, SafeNativeContext, SafeNativeResult};
use aptos_types::on_chain_config::OnChainConsensusConfig;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::values::Value;

pub fn validator_txn_enabled(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let config_bytes = safely_pop_arg!(args, Vec<u8>);
    let config = bcs::from_bytes::<OnChainConsensusConfig>(&config_bytes).unwrap_or_default();
    Ok(smallvec![Value::bool(config.is_vtxn_enabled())])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = vec![
        (
            "validator_txn_enabled_internal",
            validator_txn_enabled as RawSafeNative,
        ),
    ];

    builder.make_named_natives(natives)
}
