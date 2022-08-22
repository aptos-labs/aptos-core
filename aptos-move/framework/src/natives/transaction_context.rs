// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_algebra::InternalGas,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
    },
};
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

/// The native transaction context extension. This needs to be attached to the
/// NativeContextExtensions value which is passed into session functions, so its accessible from
/// natives of this extension.
#[derive(Tid)]
pub struct NativeTransactionContext {
    script_hash: Vec<u8>,
}

impl NativeTransactionContext {
    /// Create a new instance of a native transaction context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new(script_hash: Vec<u8>) -> Self {
        Self { script_hash }
    }
}

/***************************************************************************************************
 * native fun get_script_hash
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Clone, Debug)]
pub struct GetScriptHashGasParameters {
    pub base: InternalGas,
}

fn native_get_script_hash(
    gas_params: &GetScriptHashGasParameters,
    context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let transaction_context = context.extensions().get::<NativeTransactionContext>();

    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::vector_u8(transaction_context.script_hash.clone())],
    ))
}

pub fn make_native_get_script_hash(gas_params: GetScriptHashGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_get_script_hash(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub get_script_hash: GetScriptHashGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "get_script_hash",
        make_native_get_script_hash(gas_params.get_script_hash),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
