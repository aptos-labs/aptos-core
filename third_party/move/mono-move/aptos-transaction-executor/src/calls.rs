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
use mono_move_global_context::{ExecutionGuard, LoadedModule};
use mono_move_runtime::{
    error::{RuntimeError, RuntimeInvariantViolation},
    InterpreterContext, RuntimeStatus,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, vm_status::AbortLocation,
};

/// The parameter types of a loaded module's function, instantiated with
/// `ty_args`.
fn param_types(
    guard: &ExecutionGuard,
    loaded: &LoadedModule,
    function: InternedIdentifier,
    ty_args: InternedTypeList,
) -> Result<Vec<InternedType>> {
    let Some(Some(ir)) = loaded.get_function_ir(function) else {
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
    for (slot, ty) in func.param_slots.iter().zip(params) {
        let offset = slot.offset.0;
        match view_type(*ty) {
            Type::Signer => {
                let signer = signers.next().ok_or_else(missing_signer)?;
                interp.set_root_arg(offset, signer);
            },
            Type::ImmutRef { inner } | Type::MutRef { inner }
                if matches!(view_type(*inner), Type::Signer) =>
            {
                // A reference is a 16-byte (base, byte_offset) fat pointer at the
                // signer buffer. The base is outside the VM heap, so the GC
                // leaves it alone.
                let signer = signers.next().ok_or_else(missing_signer)?;
                let mut fat = [0u8; 16];
                fat[..8].copy_from_slice(&(signer.as_ptr() as u64).to_le_bytes());
                interp.set_root_arg(offset, &fat);
            },
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
            | Type::ImmutRef { .. }
            | Type::MutRef { .. }
            | Type::Vector { .. }
            | Type::Nominal { .. }
            | Type::Function { .. }
            | Type::TypeParam { .. } => {
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
    if signers.next().is_some() {
        bail!("too many signers for the function");
    }
    Ok(())
}

/// How a function call concluded. VM errors are reported through the `Err`
/// channel instead.
pub(crate) enum CallStatus {
    Success,
    Abort {
        code: u64,
        message: Option<String>,
        location: AbortLocation,
    },
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
) -> Result<CallStatus, VMInternalError> {
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
    place_args(interp, func, &params, signer_bufs, args).map_err(invariant_violation)?;

    Ok(match interp.run()? {
        RuntimeStatus::Success => CallStatus::Success,
        RuntimeStatus::Aborted {
            code,
            message,
            location,
        } => CallStatus::Abort {
            code,
            message,
            location,
        },
    })
}

fn invariant_violation(err: anyhow::Error) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(err.to_string()),
    ))
}
