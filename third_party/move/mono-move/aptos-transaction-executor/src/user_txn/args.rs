// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checking a transaction payload's parameters, and placing its wire-format
//! arguments -- signer addresses and BCS blobs -- onto them. Entry functions
//! and scripts are checked alike.

use crate::errors::{invariant_violation, InvalidArguments, MoveExecutionFailure};
use mono_move_core::{
    interner::{view_module_id, InternedIdentifier},
    types::{
        is_signer_or_signer_immut_ref, view_name, view_type, view_type_list, InternedType, Type,
    },
    FieldValueLayout, Function, LayoutId, LayoutKind, LayoutProvider, PreparedModule, VMResult,
};
use mono_move_global_context::{ExecutionGuard, LoadedModule};
use mono_move_runtime::{
    CallBuilder, CompletedCall, InterpreterContext, RuntimeError, RuntimeStatus,
};
use move_binary_format::{access::ModuleAccess, file_format::FunctionDefinitionIndex};
use move_core_types::account_address::AccountAddress;
use shared_dsa::UnorderedMap;

/// Runs a user transaction's call to `func`, which `module` defines under
/// `name`: checks that a transaction may call the function, builds the call,
/// places the arguments and runs it. Entry functions and scripts share this
/// path so that their checks cannot drift apart.
pub(super) fn run_user_txn_call<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    module: &LoadedModule,
    name: InternedIdentifier,
    func: &'a Function,
    sender: &AccountAddress,
    secondary_signers: &[AccountAddress],
    args: &[Vec<u8>],
) -> Result<RuntimeStatus, MoveExecutionFailure> {
    let def_idx = module
        .function_def_idx(name)
        .ok_or_else(|| invariant_violation("a loaded function is defined by its module"))
        .map_err(MoveExecutionFailure::RuntimeError)?;
    check_no_return_values(&module.ir().module, def_idx)
        .map_err(MoveExecutionFailure::InvalidArguments)?;
    let num_signer_params = check_callable_signature(guard, interp, &func.param_tys)?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    place_user_txn_args(
        &mut call,
        num_signer_params,
        sender,
        secondary_signers,
        args,
    )?;
    call.run()
        .map(CompletedCall::into_status)
        .map_err(|err| MoveExecutionFailure::RuntimeError(err.into_error()))
}

/// Checks that the function returns no values, which no transaction payload
/// may do.
fn check_no_return_values(
    module: &PreparedModule,
    def_idx: FunctionDefinitionIndex,
) -> Result<(), InvalidArguments> {
    let def = module.function_def_at(def_idx);
    let handle = module.function_handle_at(def.function);
    if !module.interned_types_at(handle.return_).is_empty() {
        return Err(InvalidArguments::ReturnsValues);
    }
    Ok(())
}

/// Checks that a user transaction may call the given function, based on info from its signature.
/// - All signers must be in leading positions.
/// - All other parameters must be of the allowed types.
fn check_callable_signature<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    param_tys: &[InternedType],
) -> Result<usize, MoveExecutionFailure> {
    let num_signer_params =
        leading_signer_params(param_tys).map_err(MoveExecutionFailure::InvalidArguments)?;
    let cached_allowed_arg_types = &mut UnorderedMap::new();
    for &ty in &param_tys[num_signer_params..] {
        // Lowering the function published the layout of every parameter type
        // and everything reachable from it.
        let id = guard
            .layout_id(ty)
            .ok_or_else(|| invariant_violation("a parameter type's layout is published"))
            .map_err(MoveExecutionFailure::RuntimeError)?;
        if !is_allowed_arg_layout(guard, interp, id, cached_allowed_arg_types)
            .map_err(MoveExecutionFailure::RuntimeError)?
        {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::DisallowedParameterType,
            ));
        }
    }
    Ok(num_signer_params)
}

/// Whether a value with the given layout can be allowed as a transaction
/// argument. The walk follows the published layouts, one per type, so a type
/// reached along several paths is decided once.
//
// TODO(security): audit the depth of type arguments this recursion can reach
// so the check stays bounded.
fn is_allowed_arg_layout<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    id: LayoutId,
    cached_allowed_arg_types: &mut UnorderedMap<LayoutId, bool>,
) -> VMResult<bool> {
    if let Some(&allowed) = cached_allowed_arg_types.get(&id) {
        return Ok(allowed);
    }
    let allowed = is_allowed_arg_layout_uncached(guard, interp, id, cached_allowed_arg_types)?;
    cached_allowed_arg_types.insert(id, allowed);
    Ok(allowed)
}

fn is_allowed_arg_layout_uncached<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    id: LayoutId,
    cached_allowed_arg_types: &mut UnorderedMap<LayoutId, bool>,
) -> VMResult<bool> {
    let layout = guard
        .layout(id)
        .ok_or_else(|| invariant_violation("a layout id resolves to a published layout"))?;
    Ok(match &layout.kind {
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address => true,
        // Signers come from the transaction itself, and neither a reference
        // nor a function value can be built from bytes.
        LayoutKind::Signer | LayoutKind::Ref | LayoutKind::Function => false,
        LayoutKind::Vector { elem_id, .. } => {
            is_allowed_arg_layout(guard, interp, *elem_id, cached_allowed_arg_types)?
        },
        LayoutKind::Struct { fields } => {
            match is_allowed_nominal_type(interp, nominal_type_of(layout.ty)?)? {
                Some(allowed) => allowed,
                None => are_allowed_arg_fields(guard, interp, fields, cached_allowed_arg_types)?,
            }
        },
        LayoutKind::FrozenEnum { variants, .. } => {
            match is_allowed_nominal_type(interp, nominal_type_of(layout.ty)?)? {
                Some(allowed) => allowed,
                None => {
                    // An argument may carry any variant, so every variant's
                    // fields are checked.
                    for &variant in variants.iter() {
                        let body = guard.layout(variant).ok_or_else(|| {
                            invariant_violation("a variant body's layout is published")
                        })?;
                        let LayoutKind::Struct { fields } = &body.kind else {
                            return Err(invariant_violation("a variant body has a struct layout"));
                        };
                        if !are_allowed_arg_fields(guard, interp, fields, cached_allowed_arg_types)?
                        {
                            return Ok(false);
                        }
                    }
                    true
                },
            }
        },
    })
}

/// Whether every field laid out by a struct or variant body can be allowed as
/// a transaction argument.
fn are_allowed_arg_fields<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    fields: &[FieldValueLayout],
    cached_allowed_arg_types: &mut UnorderedMap<LayoutId, bool>,
) -> VMResult<bool> {
    for field in fields {
        if !is_allowed_arg_layout(guard, interp, field.id, cached_allowed_arg_types)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The type a struct or enum layout was published for.
fn nominal_type_of(ty: Option<InternedType>) -> VMResult<InternedType> {
    ty.ok_or_else(|| invariant_violation("a struct or enum layout carries its type"))
}

/// Decides whether a struct or enum can be allowed as a transaction argument
/// from its type alone, or returns [`None`] when its fields decide.
/// - The framework types AptosVM accepts are allowed; `Object<T>` only if `T`
///   is a nominal type.
/// - Any other struct or enum must declare `copy` and not `key`, and must be
///   packable from outside its module. See [`LoadedModule::has_public_pack_api`].
///   Its fields, those of every variant for an enum, then decide.
//
// TODO(perf): cache the answer per type across transactions instead of reading
// the defining module's abilities and pack API on every one. Such a cache must
// be dropped when the defining module is upgraded: an enum may gain a variant
// whose fields are not allowed.
fn is_allowed_nominal_type<'a>(
    interp: &mut InterpreterContext<'a>,
    ty: InternedType,
) -> VMResult<Option<bool>> {
    let Type::Nominal {
        module_id,
        name,
        ty_args,
    } = view_type(ty)
    else {
        return Err(invariant_violation(
            "a struct or enum layout describes a nominal type",
        ));
    };
    let module = view_module_id(*module_id);
    match (
        *module.address() == AccountAddress::ONE,
        view_name(module.name()),
        view_name(*name),
    ) {
        // An `Option<T>` is allowed exactly when `T` is, which its one
        // field of `T`s decides.
        (true, "option", "Option") => Ok(None),
        // Only a resource can sit under an object's address, so the
        // existence check needs a nominal type to look for.
        (true, "object", "Object") => {
            let &[resource] = view_type_list(*ty_args) else {
                return Ok(Some(false));
            };
            Ok(Some(matches!(view_type(resource), Type::Nominal { .. })))
        },
        (true, "string", "String")
        | (true, "fixed_point32", "FixedPoint32")
        | (true, "fixed_point64", "FixedPoint64") => Ok(Some(true)),
        _ => {
            let module = interp.load_module(*module_id)?;
            let prepared = &module.ir().module;
            let Some(handle) = prepared.nominal_handle(*module_id, *name) else {
                return Ok(Some(false));
            };
            if !handle.abilities.has_copy() || handle.abilities.has_key() {
                return Ok(Some(false));
            }
            let Some(def_idx) = prepared.interned_nominal_type_def_idx(*name) else {
                return Ok(Some(false));
            };
            if !module.has_public_pack_api(def_idx) {
                return Ok(Some(false));
            }
            Ok(None)
        },
    }
}

/// Counts the leading signer parameters, rejecting a signer that follows a
/// non-signer parameter.
fn leading_signer_params(param_tys: &[InternedType]) -> Result<usize, InvalidArguments> {
    let num_signer_params = param_tys
        .iter()
        .take_while(|&&ty| is_signer_or_signer_immut_ref(ty))
        .count();
    if param_tys[num_signer_params..]
        .iter()
        .any(|&ty| is_signer_or_signer_immut_ref(ty))
    {
        return Err(InvalidArguments::SignerAfterArgument);
    }
    Ok(num_signer_params)
}

/// Fills the call in parameter order: the `num_signer_params` leading signer
/// parameters from the sender and secondary signers, everything else from the
/// transaction's BCS arguments.
fn place_user_txn_args<'a>(
    call: &mut CallBuilder<'a, '_>,
    num_signer_params: usize,
    sender: &'a AccountAddress,
    secondary_signers: &'a [AccountAddress],
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    // A function can take either all signers or none of them.
    if args.len() != call.param_tys().len() - num_signer_params {
        return Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::ArgumentCountMismatch,
        ));
    }
    if num_signer_params > 0 && 1 + secondary_signers.len() != num_signer_params {
        return Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::SignerCountMismatch,
        ));
    }
    // The sender fills the first signer parameter, secondary signers the
    // rest. Placing a signer can only fail on a bug.
    for signer in std::iter::once(sender)
        .chain(secondary_signers)
        .take(num_signer_params)
    {
        call.signer(signer)
            .map_err(MoveExecutionFailure::RuntimeError)?;
    }
    for arg in args {
        call.arg_bcs_untrusted(arg).map_err(|err| {
            let Some(runtime_error) = err.downcast_ref::<RuntimeError>() else {
                return MoveExecutionFailure::RuntimeError(err);
            };
            // TODO(cleanup): group the BCS decoding and argument validation
            // errors in `RuntimeError` so this match can be exhaustive and a
            // new variant cannot slip through the wildcard.
            let reason = match runtime_error {
                RuntimeError::MalformedStringArgument => InvalidArguments::MalformedString,
                RuntimeError::ObjectArgumentDoesNotExist => InvalidArguments::ObjectDoesNotExist,
                RuntimeError::ObjectArgumentLacksResource => InvalidArguments::ObjectLacksResource,
                decode if decode.is_bcs_decode_error() => InvalidArguments::UndecodableArgument,
                _ => return MoveExecutionFailure::RuntimeError(err),
            };
            MoveExecutionFailure::InvalidArguments(reason)
        })?;
    }
    Ok(())
}
