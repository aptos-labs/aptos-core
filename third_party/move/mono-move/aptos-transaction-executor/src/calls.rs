// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared plumbing for running a Move function inside the transaction's
//! single interpreter context.

use mono_move_core::{types::InternedTypeList, Function, Interner, VMInternalError};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{CallBuilder, InterpreterContext, RuntimeStatus};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Resolves `module::function<ty_args>` by name, metered against the gas
/// budget.
pub(crate) fn resolve_function_by_name<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    ty_args: InternedTypeList,
) -> Result<&'a Function, VMInternalError> {
    let module_id = guard.module_id_of(address, module_name);
    let function = guard.identifier_of(function_name);
    interp.load_function(module_id, function, ty_args)
}

/// Like `call_function`, but for system code: nothing consumes the
/// transaction's gas budget, `signers` fill the leading signer parameters,
/// and `place` fills the rest.
pub(crate) fn call_system_function_unmetered<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    ty_args: InternedTypeList,
    signers: &[AccountAddress],
    place: impl FnOnce(&mut CallBuilder<'_, '_>) -> Result<(), VMInternalError>,
) -> Result<RuntimeStatus, VMInternalError> {
    interp.unmetered(|interp| {
        let func =
            resolve_function_by_name(guard, interp, address, module_name, function_name, ty_args)?;
        let mut call = interp.build_call(func)?;
        for signer in signers {
            call.signer(signer)?;
        }
        place(&mut call)?;
        call.run()
    })
}
