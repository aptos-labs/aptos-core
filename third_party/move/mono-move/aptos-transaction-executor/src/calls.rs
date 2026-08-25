// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Helpers for running Move functions inside the transaction's single
//! interpreter context: parameter typing, argument placement, and the call
//! entry points.

use anyhow::{anyhow, bail, Result};
use mono_move_core::{
    interner::InternedIdentifier,
    types::{view_type, view_type_list, InternedType, InternedTypeList, Type},
    Function, Interner, VMInternalError,
};
use mono_move_global_context::{ExecutionGuard, FunctionIrLookup, LoadedModule};
use mono_move_runtime::{
    error::{RuntimeError, RuntimeInvariantViolation},
    InterpreterContext, RuntimeStatus,
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// The parameter types of a loaded module's function, instantiated with
/// `ty_args`.
//
// TODO(perf, cleanup): `Function` does not carry its signature, so this
// re-derives the parameters from the module IR on every call -- which is also
// the only reason callers need the `LoadedModule`.
fn param_types(
    guard: &ExecutionGuard,
    loaded: &LoadedModule,
    function: InternedIdentifier,
    ty_args: InternedTypeList,
) -> Result<Vec<InternedType>> {
    let FunctionIrLookup::Ir(ir) = loaded.get_function_ir(function) else {
        bail!("function has no IR in its loaded module");
    };
    let params = loaded
        .ir()
        .module
        .function_signature_at(ir.handle_idx)
        .params;
    Ok(view_type_list(guard.subst_type_list(params, ty_args)?).to_vec())
}

/// Fills the root frame of `func` from the signer and BCS-argument lists, in
/// parameter order: `signer`/`&signer` parameters from the signer list,
/// everything else deserialized from BCS.
//
// TODO(correctness): rework this part as a whole together with proper argument
// validation.
//
// TODO(perf): don't mandate BCS-encoded arguments: a caller that already holds
// the values (e.g. system function inputs) should be able to place them
// directly, without the BCS round trip.
pub(crate) fn place_args(
    interp: &mut InterpreterContext<'_>,
    func: &Function,
    params: &[InternedType],
    signer_bufs: &[[u8; AccountAddress::LENGTH]],
    args: &[Vec<u8>],
) -> Result<()> {
    if func.param_slots.len() != params.len() {
        bail!(
            "lowered function has {} parameter slots but the signature has {} parameters",
            func.param_slots.len(),
            params.len()
        );
    }
    let mut signers = signer_bufs.iter();
    let mut args = args.iter();
    let missing_signer = || anyhow!("not enough signers for the function");
    // Signers must lead the parameter list, as the legacy VM requires.
    let mut seen_argument = false;
    let mut signer_params = 0;
    for (slot, ty) in func.param_slots.iter().zip(params) {
        let offset = slot.offset.0;
        let view = view_type(*ty);
        let is_signer_param = matches!(view, Type::Signer)
            || matches!(view, Type::ImmutRef { inner } | Type::MutRef { inner }
                if matches!(view_type(*inner), Type::Signer));
        if is_signer_param {
            if seen_argument {
                bail!("a signer parameter follows an argument");
            }
            signer_params += 1;
        } else {
            seen_argument = true;
        }
        match view {
            Type::Signer => {
                let signer = signers.next().ok_or_else(missing_signer)?;
                interp.set_root_arg(offset, signer);
            },
            Type::ImmutRef { inner } | Type::MutRef { inner }
                if matches!(view_type(*inner), Type::Signer) =>
            {
                let signer = signers.next().ok_or_else(missing_signer)?;
                // SAFETY: the signer buffers outlive the call, and sit outside
                // the VM heap so the GC leaves the reference alone.
                unsafe { interp.set_root_ref_arg(offset, signer.as_ptr()) };
            },
            // A reference to anything else cannot be built from a transaction
            // argument, and a function value would let a payload pass something
            // like `minter: || Coin`.
            Type::ImmutRef { .. } | Type::MutRef { .. } => {
                bail!("a reference to a non-signer is not a valid argument")
            },
            Type::Function { .. } => bail!("a function value is not a valid argument"),
            // Substitution has already run, so a type parameter here means it
            // failed.
            Type::TypeParam { .. } => bail!("parameter type is still uninstantiated"),
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address
            | Type::Vector { .. }
            // TODO(security, completeness): only the framework's constructible
            // structs (`String`, `Object`, `Option`, ...) are valid arguments;
            // everything else must be rejected. See the pre-coordinator gates in
            // AGENTS.md.
            | Type::Nominal { .. } => {
                let arg = args
                    .next()
                    .ok_or_else(|| anyhow!("not enough arguments for the function"))?;
                // SAFETY: `offset`/`ty` come from this function's own signature, so
                // the slot is valid for the type's in-memory size.
                unsafe { interp.deserialize_root_arg(offset, *ty, arg) }.map_err(|e| {
                    anyhow!("failed to place argument at frame offset {}: {}", offset, e)
                })?;
            },
        }
    }
    if args.next().is_some() {
        bail!("too many arguments for the function");
    }
    // Like the legacy VM: a function with signer parameters requires exactly
    // that many signers; one without ignores them.
    if signer_params > 0 && signers.next().is_some() {
        bail!("more signers than the function's signer parameters");
    }
    Ok(())
}

/// Loads `module::function<ty_args>`, places arguments, and runs it in the
/// transaction's interpreter context, metered against the context's gas
/// budget.
pub(crate) fn call_function(
    guard: &ExecutionGuard<'_>,
    interp: &mut InterpreterContext<'_>,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    ty_args: InternedTypeList,
    signer_bufs: &[[u8; AccountAddress::LENGTH]],
    args: &[Vec<u8>],
) -> Result<RuntimeStatus, VMInternalError> {
    let module_id = guard.module_id_of(address, module_name);
    let function = guard.identifier_of(function_name);
    let func_ptr = interp.load_function(module_id, function, ty_args)?;
    // SAFETY: the pointer lives in a LoadedModule arena kept alive by the guard.
    let func: &Function = unsafe { func_ptr.as_ref_unchecked() };

    let loaded = interp
        .read_set()
        .get_loaded(guard.arena_ref_for_module_id(module_id))?;
    let params = param_types(guard, loaded, function, ty_args).map_err(invariant_violation)?;

    interp.prepare_call(func);
    // TODO(correctness): rejecting an argument is a user error, not an invariant
    // violation. It needs a real status once argument validation moves into the
    // pre-execution checks.
    place_args(interp, func, &params, signer_bufs, args).map_err(invariant_violation)?;

    interp.run()
}

/// An error that should not be reachable, as a VM error.
pub(crate) fn invariant_violation(err: anyhow::Error) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(err.to_string()),
    ))
}

/// Like `call_function`, but system code never consumes the transaction's gas
/// budget, including its module loads.
pub(crate) fn call_system_function_unmetered(
    guard: &ExecutionGuard<'_>,
    interp: &mut InterpreterContext<'_>,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    ty_args: InternedTypeList,
    signer_bufs: &[[u8; AccountAddress::LENGTH]],
    args: &[Vec<u8>],
) -> Result<RuntimeStatus, VMInternalError> {
    interp.unmetered(|interp| {
        call_function(
            guard,
            interp,
            address,
            module_name,
            function_name,
            ty_args,
            signer_bufs,
            args,
        )
    })
}

/// Reduces a completed call to success or the abort it ended in.
pub(crate) fn into_result(
    status: RuntimeStatus,
) -> Result<(), crate::errors::MoveExecutionFailure> {
    use crate::errors::MoveExecutionFailure;
    match status {
        RuntimeStatus::Success => Ok(()),
        RuntimeStatus::Aborted {
            code,
            message,
            location,
        } => Err(MoveExecutionFailure::Abort {
            code,
            message,
            location,
        }),
    }
}
