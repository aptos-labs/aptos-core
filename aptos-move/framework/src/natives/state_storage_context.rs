// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::vm_status::StatusCode;
use better_any::{Tid, TidAble};
use move_deps::move_binary_format::errors::PartialVMError;
use move_deps::move_vm_types::pop_arg;
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

/// FIXME(aldenhu): doc
pub struct StateStorageUsage {
    items: usize,
    bytes: usize,
}

/// FIXME(aldenhu): doc
/// A table resolver which needs to be provided by the environment. This allows to lookup
/// data in remote storage, as well as retrieve cost of table operations.
pub trait StateStorageUsageResolver {
    fn get_state_storage_usage_at_epoch_ending(
        &self,
        epoch: u64,
    ) -> Result<StateStorageUsage, anyhow::Error>;
}

/// FIXME(aldenhu)
/// The native transaction context extension. This needs to be attached to the
/// NativeContextExtensions value which is passed into session functions, so its accessible from
/// natives of this extension.
#[derive(Tid)]
pub struct NativeStateStorageContext<'a> {
    resolver: &'a dyn StateStorageUsageResolver,
}

impl<'a> NativeStateStorageContext<'a> {
    /// FIXME(aldenhu)
    /// Create a new instance of a native transaction context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new(resolver: &'a dyn StateStorageUsageResolver) -> Self {
        Self { resolver }
    }
}

/// FIXME(aldenhu)
/***************************************************************************************************
 * native fun get_script_hash
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Clone, Debug)]
pub struct GetStateStorageUsageAtEpochEndingGasParameters {
    pub base_cost: u64,
}

fn native_get_state_storage_usage_at_epoch_ending(
    gas_params: &GetStateStorageUsageAtEpochEndingGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(_ty_args.is_empty());
    assert_eq!(args.len(), 1);

    let ctx = context.extensions().get::<NativeStateStorageContext>();
    let epoch = pop_arg!(args, u64);

    let usage = ctx
        .resolver
        .get_state_storage_usage_at_epoch_ending(epoch)
        .map_err(|err| {
            PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
                .with_message(format!("Failed to get state storage usage: {}", err))
        })?;

    Ok(NativeResult::ok(
        gas_params.base_cost,
        smallvec![Value::struct_(Struct::pack(vec![
            Value::u64(usage.items as u64),
            Value::u64(usage.bytes as u64),
        ]))],
    ))
}

pub fn make_native_get_state_storage_usage_at_epoch_ending(
    gas_params: GetStateStorageUsageAtEpochEndingGasParameters,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_get_state_storage_usage_at_epoch_ending(&gas_params, context, ty_args, args)
    })
}

/// FIXME(aldenhu)
/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub get_state_storage_usage_at_epoch_ending: GetStateStorageUsageAtEpochEndingGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "get_state_storage_usage_at_epoch_ending",
        make_native_get_state_storage_usage_at_epoch_ending(
            gas_params.get_state_storage_usage_at_epoch_ending,
        ),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
