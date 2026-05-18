// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_types::{state_store::state_key::StateKey, vm_status::StatusCode};
use aptos_vm_types::resolver::StateStorageView;
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMError;
use move_vm_runtime::{
    native_extensions::{NativeRuntimeRefCheckModelsCompleted, SessionListener},
    native_functions::NativeFunction,
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Exposes the ability to query state storage utilization info to native functions.
#[derive(Tid)]
pub struct NativeStateStorageContext<'a> {
    resolver: &'a dyn StateStorageView<Key = StateKey>,
}

// This extension only carries a reference to pre-block view, so all session-related APIs are
// no-ops.
impl<'a> SessionListener for NativeStateStorageContext<'a> {
    fn start(&mut self, _session_hash: &[u8; 32], _script_hash: &[u8], _session_counter: u8) {}

    fn finish(&mut self) {}

    fn abort(&mut self) {}
}

impl<'a> NativeRuntimeRefCheckModelsCompleted for NativeStateStorageContext<'a> {
    // No native functions in this context return references, so no models to add.
}

impl<'a> NativeStateStorageContext<'a> {
    pub fn new(resolver: &'a dyn StateStorageView<Key = StateKey>) -> Self {
        Self { resolver }
    }
}

/***************************************************************************************************
 * native get_state_storage_usage_only_at_epoch_beginning
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
/// Warning: the result returned is based on the base state view held by the
/// VM for the entire block or chunk of transactions, it's only deterministic
/// if called from the first transaction of the block because the execution layer
/// guarantees a fresh state view then.
fn native_get_usage(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert!(_ty_args.is_empty());
    assert!(_args.is_empty());

    context.charge(STATE_STORAGE_GET_USAGE_BASE_COST)?;

    let ctx = context.extensions().get::<NativeStateStorageContext>();
    let usage = ctx.resolver.get_usage().map_err(|err| {
        PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
            .with_message(format!("Failed to get state storage usage: {}", err))
    })?;

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::u64(usage.items() as u64),
        Value::u64(usage.bytes() as u64),
    ]))])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [(
        "get_state_storage_usage_only_at_epoch_beginning",
        native_get_usage as RawSafeNative,
    )];

    builder.make_named_natives(natives)
}
