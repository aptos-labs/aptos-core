// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    calls::resolve_function_by_name,
    errors::{InvalidArguments, MoveExecutionFailure},
};
use mono_move_core::{
    interner::{view_module_id, InternedIdentifier, InternedModuleId},
    types::{
        is_signer_or_signer_immut_ref, view_name, view_type, InternedType, InternedTypeList, Type,
    },
    Function, PreparedModule,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::LoaderError;
use mono_move_runtime::{CallBuilder, InterpreterContext, RuntimeError, RuntimeStatus};
use move_binary_format::access::ModuleAccess;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Checks that `func` is allowed to be called by a user transaction, returning
/// the number of leading signer parameters.
/// - It must be an entry function.
/// - It must not return values.
/// - All signers must be in leading positions.
/// - All other parameters after the leading signers must be of the allowed types.
fn check_callable_by_user_txn(
    func: &Function,
    module: &PreparedModule,
) -> Result<usize, InvalidArguments> {
    let def = module.function_def_at(func.def_idx);
    if !def.is_entry {
        return Err(InvalidArguments::NotEntryFunction);
    }
    let handle = module.function_handle_at(def.function);
    if !module.interned_types_at(handle.return_).is_empty() {
        return Err(InvalidArguments::ReturnsValues);
    }
    let signer_params = func
        .param_tys
        .iter()
        .take_while(|&&ty| is_signer_or_signer_immut_ref(ty))
        .count();
    for &ty in &func.param_tys[signer_params..] {
        if is_signer_or_signer_immut_ref(ty) {
            return Err(InvalidArguments::SignerAfterArgument);
        }
        if !is_allowed_arg_type(ty) {
            return Err(InvalidArguments::DisallowedParameterType);
        }
    }
    Ok(signer_params)
}

/// Whether a type can be allowed as a transaction argument.
//
// TODO(completeness): support construction of public structs and enums.
fn is_allowed_arg_type(ty: InternedType) -> bool {
    match view_type(ty) {
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
        | Type::Address => true,
        Type::Vector { elem } => is_allowed_arg_type(*elem),
        Type::Nominal {
            module_id, name, ..
        } => is_allowed_framework_struct(*module_id, *name),
        Type::Signer
        | Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => false,
    }
}

/// Whether the given type is a framework struct allowed to be constructed as a transaction argument.
fn is_allowed_framework_struct(module_id: InternedModuleId, name: InternedIdentifier) -> bool {
    let module_id = view_module_id(module_id);
    matches!(
        (
            *module_id.address(),
            view_name(module_id.name()),
            view_name(name)
        ),
        (AccountAddress::ONE, "string", "String")
            | (AccountAddress::ONE, "object", "Object")
            | (AccountAddress::ONE, "option", "Option")
            | (AccountAddress::ONE, "fixed_point32", "FixedPoint32")
            | (AccountAddress::ONE, "fixed_point64", "FixedPoint64")
    )
}

// ---------------------------------------------------------------------------
// Argument placement
// ---------------------------------------------------------------------------

/// Fills the call in parameter order: the `signer_params` leading signer
/// parameters from the sender and secondary signers, everything else from the
/// transaction's BCS arguments.
fn place_user_txn_args<'a>(
    call: &mut CallBuilder<'a, '_>,
    signer_params: usize,
    sender: &'a AccountAddress,
    secondary_signers: &'a [AccountAddress],
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    // Like AptosVM, check both counts before decoding any argument: a function
    // with signer parameters requires exactly that many signers, while one
    // without ignores them.
    if args.len() != call.param_tys().len() - signer_params {
        return Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::ArgumentCountMismatch,
        ));
    }
    if signer_params > 0 && 1 + secondary_signers.len() != signer_params {
        return Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::SignerCountMismatch,
        ));
    }
    // The sender fills the first signer parameter, secondary signers the
    // rest. Placing a signer can only fail on a bug.
    for signer in std::iter::once(sender)
        .chain(secondary_signers)
        .take(signer_params)
    {
        call.signer(signer)
            .map_err(MoveExecutionFailure::RuntimeError)?;
    }
    // TODO(security, completeness): check that `String` arguments are valid
    // UTF-8 and `Object<T>` arguments point at existing objects, including
    // when nested in vectors and options.
    for arg in args {
        // Only a decode failure faults the argument's bytes; anything else
        // (a non-decodable parameter type, an exhausted heap) is the VM's.
        call.arg_bcs(arg).map_err(|err| {
            if err
                .downcast_ref::<RuntimeError>()
                .is_some_and(RuntimeError::is_bcs_decode_error)
            {
                MoveExecutionFailure::InvalidArguments(InvalidArguments::UndecodableArgument)
            } else {
                MoveExecutionFailure::RuntimeError(err)
            }
        })?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Running the payload
// ---------------------------------------------------------------------------

/// Runs the transaction's entry function on its wire-format arguments,
/// metered against the transaction's gas budget.
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
    let func =
        match resolve_function_by_name(guard, interp, address, module_name, function_name, ty_args)
        {
            Ok(func) => func,
            // A native never lowers, so it surfaces as a load failure; AptosVM
            // loads it and then refuses to run it.
            Err(err)
                if matches!(
                    err.downcast_ref::<LoaderError>(),
                    Some(LoaderError::NativeFunctionNotLoadable { .. })
                ) =>
            {
                return Err(MoveExecutionFailure::InvalidArguments(
                    InvalidArguments::NativeEntryFunction,
                ));
            },
            Err(err) => return Err(MoveExecutionFailure::RuntimeError(err)),
        };
    // TODO(completeness): AptosVM marks the session unbiasable when a friend
    // or private entry function carries the `#[randomness]` annotation.
    let module = interp
        .module_of(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let signer_params =
        check_callable_by_user_txn(func, module).map_err(MoveExecutionFailure::InvalidArguments)?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    place_user_txn_args(&mut call, signer_params, sender, secondary_signers, args)?;
    call.run().map_err(MoveExecutionFailure::RuntimeError)
}
