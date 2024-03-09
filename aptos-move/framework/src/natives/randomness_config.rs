// Copyright © Aptos Foundation

use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

pub fn enabled_internal(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let config_bytes = safely_pop_arg!(args, Vec<u8>);
    let config = bcs::from_bytes::<OnChainRandomnessConfig>(&config_bytes)
        .unwrap_or_else(|_| OnChainRandomnessConfig::default_disabled());
    Ok(smallvec![Value::bool(config.randomness_enabled())])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = vec![("enabled_internal", enabled_internal as RawSafeNative)];

    builder.make_named_natives(natives)
}
