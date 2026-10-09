// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared plumbing for running a Move function inside the transaction's
//! single interpreter context.

use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    types::InternedTypeList,
    VMInternalError,
};
use mono_move_runtime::{CallBuilder, InterpreterContext, RuntimeStatus};
use move_core_types::account_address::AccountAddress;

/// Calls `module::function<ty_args>` as system code: `signers` fill the
/// leading signer parameters and `place` fills the rest. Metered against the
/// transaction's gas budget.
pub(crate) fn call_system_function<'a>(
    interp: &mut InterpreterContext<'a>,
    module: InternedModuleId,
    function: InternedIdentifier,
    ty_args: InternedTypeList,
    signers: &[AccountAddress],
    place: impl FnOnce(&mut CallBuilder<'_, '_>) -> Result<(), VMInternalError>,
) -> Result<RuntimeStatus, VMInternalError> {
    let func = interp.load_function(module, function, ty_args)?;
    let mut call = interp.build_call(func)?;
    for signer in signers {
        call.signer(signer)?;
    }
    place(&mut call)?;
    Ok(call.run()?.into_status())
}

/// Like [`call_system_function`], but nothing consumes the transaction's gas
/// budget.
pub(crate) fn call_system_function_unmetered<'a>(
    interp: &mut InterpreterContext<'a>,
    module: InternedModuleId,
    function: InternedIdentifier,
    ty_args: InternedTypeList,
    signers: &[AccountAddress],
    place: impl FnOnce(&mut CallBuilder<'_, '_>) -> Result<(), VMInternalError>,
) -> Result<RuntimeStatus, VMInternalError> {
    interp
        .unmetered(|interp| call_system_function(interp, module, function, ty_args, signers, place))
}
