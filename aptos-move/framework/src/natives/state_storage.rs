// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeResult};
use aptos_types::{
    on_chain_config::{Features, TimedFeatures},
    state_store::state_storage_usage::StateStorageUsage,
    vm_status::StatusCode,
};
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMError;
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, sync::Arc};

/// Ability to reveal the state storage utilization info.
pub trait StateStorageUsageResolver {
    fn get_state_storage_usage(&self) -> anyhow::Result<StateStorageUsage>;
}

/// Exposes the ability to query state storage utilization info to native functions.
#[derive(Tid)]
pub struct NativeStateStorageContext<'a> {
    resolver: &'a dyn StateStorageUsageResolver,
}

impl<'a> NativeStateStorageContext<'a> {
    pub fn new(resolver: &'a dyn StateStorageUsageResolver) -> Self {
        Self { resolver }
    }
}

/***************************************************************************************************
 * native get_state_storage_usage_only_at_eopch_beginning
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Clone, Debug)]
pub struct GetUsageGasParameters {
    pub base_cost: InternalGas,
}

/// Warning: the result returned is based on the base state view held by the
/// VM for the entire block or chunk of transactions, it's only deterministic
/// if called from the first transaction of the block because the execution layer
/// guarantees a fresh state view then.
fn native_get_usage(
    gas_params: &GetUsageGasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert!(_ty_args.is_empty());
    assert!(_args.is_empty());

    context.charge(gas_params.base_cost)?;

    let ctx = context.extensions().get::<NativeStateStorageContext>();
    let usage = ctx.resolver.get_state_storage_usage().map_err(|err| {
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
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub get_usage: GetUsageGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "get_state_storage_usage_only_at_epoch_beginning",
        make_safe_native(
            gas_params.get_usage,
            timed_features,
            features,
            native_get_usage,
        ),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
