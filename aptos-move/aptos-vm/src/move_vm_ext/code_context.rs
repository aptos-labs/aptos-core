// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{MoveResolverExt, SessionExt};
use aptos_types::transaction::ModuleBundle;
use aptos_types::vm_status::StatusCode;
use better_any::{Tid, TidAble};
use move_deps::move_binary_format::errors::PartialVMError;
use move_deps::move_vm_types::pop_arg;
use move_deps::move_vm_types::values::Struct;
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
use std::collections::{BTreeSet, VecDeque};

/// Abort code when code publishing is requested twice (0x03 == INVALID_STATE)
const EALREADY_REQUESTED: u64 = 0x03_0000;

/// The native code context.
#[derive(Tid, Default)]
pub struct NativeCodeContext {
    /// Remembers whether the publishing of a module bundle was requested during transaction
    /// execution.
    requested_module_bundle: Option<PublishRequest>,
}

pub struct PublishRequest {
    pub destination: AccountAddress,
    pub bundle: ModuleBundle,
    pub expected_modules: BTreeSet<String>,
    pub check_compat: bool,
}

/// Returns all natives for code module.
pub fn code_natives(addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(
        addr,
        &[("code", "native_request_publish", native_request_publish)],
    )
}

/// `native fun request_publish(bundle: vector<vector<u8>>)`
fn native_request_publish(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert_eq!(args.len(), 4);
    let check_compat = pop_arg!(args, bool);
    let mut expected_modules = BTreeSet::new();
    for name in pop_arg!(args, Vec<Value>) {
        expected_modules.insert(get_move_string(name)?);
    }
    let mut code = vec![];
    for module in pop_arg!(args, Vec<Value>) {
        code.push(module.value_as::<Vec<u8>>()?);
    }
    let destination = pop_arg!(args, AccountAddress);
    let code_context = context.extensions_mut().get_mut::<NativeCodeContext>();
    if code_context.requested_module_bundle.is_some() {
        // Can't request second time.
        return Err(PartialVMError::new(StatusCode::ABORTED).with_sub_status(EALREADY_REQUESTED));
    }
    code_context.requested_module_bundle = Some(PublishRequest {
        destination,
        bundle: ModuleBundle::new(code),
        expected_modules,
        check_compat,
    });
    // TODO: charge gas for requesting code load (charge for actual code loading done elsewhere)
    let cost = native_gas(context.cost_table(), NativeCostIndex::EMPTY, 0);
    Ok(NativeResult::ok(cost, smallvec![]))
}

impl NativeCodeContext {
    /// Extracts any pending publish request from the session.
    pub fn extract_publish_request<S: MoveResolverExt>(
        session: &mut SessionExt<S>,
    ) -> Option<PublishRequest> {
        let ctx = session
            .get_native_extensions()
            .get_mut::<NativeCodeContext>();
        ctx.requested_module_bundle.take()
    }
}

/// Gets the string value embedded in a Move `string::String` struct.
fn get_move_string(v: Value) -> PartialVMResult<String> {
    let bytes = v
        .value_as::<Struct>()?
        .unpack()?
        .next()
        .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?
        .value_as::<Vec<u8>>()?;
    String::from_utf8(bytes).map_err(|_| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
}
