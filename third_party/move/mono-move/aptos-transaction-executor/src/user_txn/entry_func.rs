// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Running an entry-function payload.

use super::args::{leading_signer_params, place_user_txn_args};
use crate::{
    calls::resolve_function_by_name,
    errors::{invariant_violation, InvalidArguments, MoveExecutionFailure},
};
use mono_move_core::{
    interner::{view_module_id, InternedIdentifier, InternedModuleId},
    types::{view_name, view_type, view_type_list, InternedType, InternedTypeList, Type},
    FieldTypes, Function, Interner, PreparedModule, VMResult,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::LoaderError;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FunctionAttribute, StructFieldInformation, VariantIndex, Visibility},
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{DOLLAR_SIGN_DELIMITER, PACK},
};

/// Checks that `func` is allowed to be called by a user transaction, returning
/// the number of leading signer parameters.
/// - It must be an entry function.
/// - It must not return values.
/// - All signers must be in leading positions.
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
    leading_signer_params(&func.param_tys)
}

/// Checks that a transaction argument can fill every parameter type in `tys`.
fn check_param_types<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    tys: &[InternedType],
) -> Result<(), MoveExecutionFailure> {
    for &ty in tys {
        if !is_allowed_arg_type(guard, interp, ty).map_err(MoveExecutionFailure::RuntimeError)? {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::DisallowedParameterType,
            ));
        }
    }
    Ok(())
}

/// Whether a type can be allowed as a transaction argument.
fn is_allowed_arg_type<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    ty: InternedType,
) -> VMResult<bool> {
    Ok(match view_type(ty) {
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
        Type::Vector { elem } => is_allowed_arg_type(guard, interp, *elem)?,
        Type::Nominal {
            module_id,
            name,
            ty_args,
        } => {
            let module = view_module_id(*module_id);
            match (
                *module.address() == AccountAddress::ONE,
                view_name(module.name()),
                view_name(*name),
            ) {
                (true, "option", "Option") => {
                    let &[elem] = view_type_list(*ty_args) else {
                        return Ok(false);
                    };
                    is_allowed_arg_type(guard, interp, elem)?
                },
                // An `Object<T>` argument is only an address, so `T` is unrestricted.
                (true, "string", "String")
                | (true, "object", "Object")
                | (true, "fixed_point32", "FixedPoint32")
                | (true, "fixed_point64", "FixedPoint64") => true,
                _ => is_public_struct(guard, interp, *module_id, *name, *ty_args)?,
            }
        },
        Type::Signer
        | Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => false,
    })
}

/// Whether a struct or enum outside the framework whitelist can be allowed as
/// a transaction argument, loading its defining module if needed.
/// - Its definition must declare `copy` and not `key`.
/// - It must be packed by a public pack function: `pack$S` for a struct `S`,
///   `pack$S$V` for every variant `V` of an enum `S`.
/// - Its field types, instantiated with `ty_args`, must all be allowed.
fn is_public_struct<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    module_id: InternedModuleId,
    name: InternedIdentifier,
    ty_args: InternedTypeList,
) -> VMResult<bool> {
    let module = interp.load_module(module_id)?;
    let Some(handle) = module.nominal_handle(module_id, name) else {
        return Ok(false);
    };
    if !handle.abilities.has_copy() || handle.abilities.has_key() {
        return Ok(false);
    }
    let Some(def_idx) = module.interned_nominal_type_def_idx(name) else {
        return Ok(false);
    };
    let has_pack_function = |function_name: &str, attribute: FunctionAttribute| {
        module.function_defs().iter().any(|def| {
            let handle = module.function_handle_at(def.function);
            module.identifier_at(handle.name).as_str() == function_name
                && def.visibility == Visibility::Public
                && handle.attributes.contains(&attribute)
        })
    };
    let struct_name = view_name(name);
    let mut field_tys = Vec::new();
    match module.interned_field_types(name) {
        Some(FieldTypes::Struct(fields)) => {
            if !has_pack_function(
                &format!("{PACK}{DOLLAR_SIGN_DELIMITER}{struct_name}"),
                FunctionAttribute::Pack,
            ) {
                return Ok(false);
            }
            field_tys.extend_from_slice(fields);
        },
        Some(FieldTypes::Enum(variants)) => {
            let StructFieldInformation::DeclaredVariants(variant_defs) =
                &module.struct_def_at(def_idx).field_information
            else {
                return Ok(false);
            };
            for (tag, (variant, fields)) in variant_defs.iter().zip(variants).enumerate() {
                let variant_name = module.identifier_at(variant.name);
                if !has_pack_function(
                    &format!(
                        "{PACK}{DOLLAR_SIGN_DELIMITER}{struct_name}{DOLLAR_SIGN_DELIMITER}{variant_name}"
                    ),
                    FunctionAttribute::PackVariant(tag as VariantIndex),
                ) {
                    return Ok(false);
                }
                field_tys.extend_from_slice(fields);
            }
        },
        None => return Ok(false),
    }
    for field_ty in field_tys {
        let field_ty = guard
            .subst_type(field_ty, ty_args)
            .map_err(|err| invariant_violation(err.to_string()))?;
        if !is_allowed_arg_type(guard, interp, field_ty)? {
            return Ok(false);
        }
    }
    Ok(true)
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
    check_param_types(guard, interp, &func.param_tys[signer_params..])?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    place_user_txn_args(&mut call, signer_params, sender, secondary_signers, args)?;
    call.run().map_err(MoveExecutionFailure::RuntimeError)
}
