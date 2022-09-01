// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::vm_status::StatusCode;
use better_any::{Tid, TidAble};
use move_deps::move_binary_format::errors::PartialVMError;
use move_deps::move_core_types::gas_algebra::InternalGas;
use move_deps::move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    values::{Struct, Value},
};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
};
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

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
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(_ty_args.is_empty());
    assert!(_args.is_empty());

    let ctx = context.extensions().get::<NativeStateStorageContext>();
    let usage = ctx.resolver.get_state_storage_usage().map_err(|err| {
        PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
            .with_message(format!("Failed to get state storage usage: {}", err))
    })?;

    Ok(NativeResult::ok(
        gas_params.base_cost,
        smallvec![Value::struct_(Struct::pack(vec![
            Value::u64(usage.items() as u64),
            Value::u64(usage.bytes() as u64),
        ]))],
    ))
}

pub fn make_native_get_usage(gas_params: GetUsageGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| native_get_usage(&gas_params, context, ty_args, args))
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub get_usage: GetUsageGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "get_state_storage_usage_only_at_epoch_beginning",
        make_native_get_usage(gas_params.get_usage),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
