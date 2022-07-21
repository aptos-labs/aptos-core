// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{MoveResolverExt, SessionExt};
use aptos_types::vm_status::StatusCode;
use better_any::{Tid, TidAble};
use move_deps::move_binary_format::errors::{PartialVMError, VMResult};
use move_deps::move_vm_types::gas_schedule::GasStatus;
use move_deps::move_vm_types::pop_arg;
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

/// Abort code when code publishing is requested twice (0x03 == INVALID_STATE)
const EALREADY_REQUESTED: u64 = 0x03_0000;

/// The native code context.
#[derive(Tid, Default)]
pub struct NativeCodeContext {
    /// Remembers whether the publishing of a module bundle was requested during transaction
    /// execution.
    requested_module_bundle: Option<Vec<Vec<u8>>>,
}

/// Returns all natives for code module.
pub fn code_natives(addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(addr, &[("code", "request_publish", native_request_publish)])
}

/// `native fun request_publish(bundle: vector<vector<u8>>)`
fn native_request_publish(
    context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let mut modules = vec![];
    for module in pop_arg!(args, Vec<Value>) {
        modules.push(module.value_as::<Vec<u8>>()?);
    }
    let code_context = context.extensions_mut().get_mut::<NativeCodeContext>();
    if code_context.requested_module_bundle.is_some() {
        // Can't request second time.
        return Err(PartialVMError::new(StatusCode::ABORTED).with_sub_status(EALREADY_REQUESTED));
    }
    code_context.requested_module_bundle = Some(modules);
    // TODO: charge gas for code loading here? Or charge it in publish_module_bundle?
    let cost = native_gas(context.cost_table(), NativeCostIndex::EMPTY, 0);
    Ok(NativeResult::ok(cost, smallvec![]))
}

impl NativeCodeContext {
    /// Process pending code publishing requests.
    pub fn resolve_pending_requests<S: MoveResolverExt>(
        session: &mut SessionExt<S>,
        sender: &AccountAddress,
        gas_status: &mut GasStatus,
    ) -> VMResult<()> {
        let ctx = session
            .get_native_extensions()
            .get_mut::<NativeCodeContext>();
        if let Some(bundle) = ctx.requested_module_bundle.take() {
            session.publish_module_bundle(bundle, *sender, gas_status)
        } else {
            Ok(())
        }
    }
}
