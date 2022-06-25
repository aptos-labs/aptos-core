// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::account_address::AccountAddress,
    move_vm_runtime::{
        native_functions,
        native_functions::{NativeContext, NativeFunctionTable},
    },
    move_vm_types::{
        gas_schedule::NativeCostIndex,
        loaded_data::runtime_types::Type,
        natives::function::{native_gas, NativeResult},
        values::Value,
    },
};
use smallvec::smallvec;
use std::collections::VecDeque;

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

/// Returns all natives for transaction context.
pub fn transaction_context_natives(table_addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(
        table_addr,
        &[(
            "TransactionContext",
            "get_script_hash",
            native_get_script_hash,
        )],
    )
}

fn native_get_script_hash(
    context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let transaction_context = context.extensions().get::<NativeTransactionContext>();
    let cost = native_gas(context.cost_table(), NativeCostIndex::SHA3_256, 0);

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::vector_u8(transaction_context.script_hash.clone())],
    ))
}
