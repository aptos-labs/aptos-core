// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Running an entry-function payload.

use super::args::run_user_txn_call;
use crate::errors::{InvalidArguments, MoveExecutionFailure};
use mono_move_core::{
    interner::InternedIdentifier, types::InternedTypeList, Interner, PreparedModule,
};
use mono_move_global_context::{ExecutionGuard, LoadedModule};
use mono_move_natives::RandomnessContext;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FunctionDefinitionIndex, Visibility},
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Checks what sets an entry function apart from a script's `main`, based on
/// info from its definition. The checks the two share follow in
/// [`run_user_txn_call`].
/// - It must not be a native.
/// - It must be an entry function.
fn check_callable_definition(
    module: &PreparedModule,
    def_idx: FunctionDefinitionIndex,
) -> Result<(), InvalidArguments> {
    let def = module.function_def_at(def_idx);
    if def.is_native() {
        return Err(InvalidArguments::NativeEntryFunction);
    }
    if !def.is_entry {
        return Err(InvalidArguments::NotEntryFunction);
    }
    Ok(())
}

/// Whether the function `def_idx` of `module`, named `name`, may call the
/// randomness API: a private or friend function carrying the `#[randomness]`
/// annotation.
fn is_unbiasable_entry_function(
    module: &LoadedModule,
    def_idx: FunctionDefinitionIndex,
    name: InternedIdentifier,
) -> bool {
    module.ir().module.function_def_at(def_idx).visibility != Visibility::Public
        && module.has_randomness_annotation(&name)
}

/// Runs the transaction's entry function, metered against the transaction's gas budget.
pub(crate) fn call_entry_function<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    ty_args: InternedTypeList,
    sender: &AccountAddress,
    secondary_signers: &[AccountAddress],
    args: &[Vec<u8>],
) -> Result<RuntimeStatus, MoveExecutionFailure> {
    let module_id = guard.module_id_of(address, module_name);
    let function_name = guard.identifier_of(function_name);
    let module = interp
        .load_module(module_id)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    // A function the module does not define is reported by the loader below.
    if let Some(def_idx) = module.function_def_idx(function_name) {
        check_callable_definition(&module.ir().module, def_idx)
            .map_err(MoveExecutionFailure::InvalidArguments)?;
        if is_unbiasable_entry_function(module, def_idx, function_name) {
            interp
                .extensions()
                .get_mut::<RandomnessContext>()
                .map_err(MoveExecutionFailure::RuntimeError)?
                .mark_unbiasable();
        }
    }
    let func = interp
        .load_function(module_id, function_name, ty_args)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    run_user_txn_call(
        guard,
        interp,
        module,
        function_name,
        func,
        sender,
        secondary_signers,
        args,
    )
}
