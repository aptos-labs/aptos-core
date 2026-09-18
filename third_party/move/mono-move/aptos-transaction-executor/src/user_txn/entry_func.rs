// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Running an entry-function payload.

use super::args::{leading_signer_params, place_user_txn_args};
use crate::errors::{invariant_violation, InvalidArguments, MoveExecutionFailure};
use mono_move_core::{
    interner::{view_module_id, InternedIdentifier, InternedModuleId},
    types::{view_name, view_type, view_type_list, InternedType, InternedTypeList, Type},
    Interner, LayoutKind, LayoutProvider, PreparedModule, VMResult, ValueLayout,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        FunctionAttribute, FunctionDefinitionIndex, StructFieldInformation, VariantIndex,
        Visibility,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{DOLLAR_SIGN_DELIMITER, PACK},
};
use std::collections::HashMap;

/// Checks that a user transaction may call the given function based on info from its definition.
/// - It must not be a native.
/// - It must be an entry function.
/// - It must not return values.
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
    let answered = &mut HashMap::new();
    for &ty in &param_tys[num_signer_params..] {
        if !is_allowed_arg_type(guard, interp, ty, answered)
            .map_err(MoveExecutionFailure::RuntimeError)?
        {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::DisallowedParameterType,
            ));
        }
    }
    Ok(num_signer_params)
}

/// Whether a type can be allowed as a transaction argument. `answered` holds
/// the types already decided, without which a type whose fields share a type
/// is walked once per path to it rather than once.
//
// TODO(security): audit the depth of type arguments this recursion can reach
// so the check stays bounded.
fn is_allowed_arg_type<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    ty: InternedType,
    answered: &mut HashMap<InternedType, bool>,
) -> VMResult<bool> {
    if let Some(&allowed) = answered.get(&ty) {
        return Ok(allowed);
    }
    let allowed = is_allowed_arg_type_uncached(guard, interp, ty, answered)?;
    answered.insert(ty, allowed);
    Ok(allowed)
}

fn is_allowed_arg_type_uncached<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    ty: InternedType,
    answered: &mut HashMap<InternedType, bool>,
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
        Type::Vector { elem } => is_allowed_arg_type(guard, interp, *elem, answered)?,
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
                    is_allowed_arg_type(guard, interp, elem, answered)?
                },
                // Only a resource can sit under an object's address, so the
                // existence check needs a nominal type to look for.
                (true, "object", "Object") => {
                    let &[resource] = view_type_list(*ty_args) else {
                        return Ok(false);
                    };
                    matches!(view_type(resource), Type::Nominal { .. })
                },
                (true, "string", "String")
                | (true, "fixed_point32", "FixedPoint32")
                | (true, "fixed_point64", "FixedPoint64") => true,
                _ => is_allowed_nominal_type(guard, interp, ty, *module_id, *name, answered)?,
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
/// a transaction argument.
/// - Its definition must declare `copy` and not `key`.
/// - It must have public pack function: `pack$S` (if struct), or pack functions for
//    every variant (if enum).
/// - Its field types must all be allowed.
//
// TODO(perf): the answer depends only on the type, so cache it instead of
// walking the defining module's functions and fields on every transaction.
fn is_allowed_nominal_type<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    ty: InternedType,
    module_id: InternedModuleId,
    name: InternedIdentifier,
    answered: &mut HashMap<InternedType, bool>,
) -> VMResult<bool> {
    let module = &interp.load_module(module_id)?.ir().module;
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
    match &module.struct_def_at(def_idx).field_information {
        StructFieldInformation::Declared(_) => {
            if !has_pack_function(
                &format!("{PACK}{DOLLAR_SIGN_DELIMITER}{struct_name}"),
                FunctionAttribute::Pack,
            ) {
                return Ok(false);
            }
        },
        StructFieldInformation::DeclaredVariants(variants) => {
            for (tag, variant) in variants.iter().enumerate() {
                let variant_name = module.identifier_at(variant.name);
                if !has_pack_function(
                    &format!(
                        "{PACK}{DOLLAR_SIGN_DELIMITER}{struct_name}{DOLLAR_SIGN_DELIMITER}{variant_name}"
                    ),
                    FunctionAttribute::PackVariant(tag as VariantIndex),
                ) {
                    return Ok(false);
                }
            }
        },
        StructFieldInformation::Native => return Ok(false),
    }
    // Note: Lowering already published the layout of all fields, so they should be
    // readable without substitution.
    let Some(field_tys) = instantiated_field_types(guard, ty)? else {
        return Ok(false);
    };
    for field_ty in field_tys {
        if !is_allowed_arg_type(guard, interp, field_ty, answered)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The instantiated types of a struct's fields, or of every variant's fields
/// for an enum. [`None`] if a field is a reference or a function value, which
/// share one layout and so have no type of their own.
fn instantiated_field_types(
    guard: &ExecutionGuard<'_>,
    ty: InternedType,
) -> VMResult<Option<Vec<InternedType>>> {
    let layout = guard
        .layout_by_ty(ty)
        .ok_or_else(|| invariant_violation("a parameter type's layout is published"))?;
    match &layout.kind {
        LayoutKind::Struct { .. } => struct_field_types(guard, layout),
        LayoutKind::FrozenEnum { variants, .. } => {
            let mut tys = Vec::new();
            for &id in variants.iter() {
                let body = guard
                    .layout(id)
                    .ok_or_else(|| invariant_violation("a variant body's layout is published"))?;
                match struct_field_types(guard, body)? {
                    Some(variant_tys) => tys.extend(variant_tys),
                    None => return Ok(None),
                }
            }
            Ok(Some(tys))
        },
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address
        | LayoutKind::Signer
        | LayoutKind::Vector { .. }
        | LayoutKind::Ref
        | LayoutKind::Function => Err(invariant_violation(
            "a struct or enum has a struct or enum layout",
        )),
    }
}

/// The instantiated types of the fields laid out by a struct or variant body.
fn struct_field_types(
    guard: &ExecutionGuard<'_>,
    layout: &ValueLayout,
) -> VMResult<Option<Vec<InternedType>>> {
    let LayoutKind::Struct { fields } = &layout.kind else {
        return Err(invariant_violation("a struct body has a struct layout"));
    };
    fields
        .iter()
        .map(|field| {
            guard
                .layout(field.id)
                .ok_or_else(|| invariant_violation("a field's layout is published"))
                .map(|field_layout| field_layout.ty)
        })
        .collect()
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
    }
    // TODO(completeness): AptosVM marks the session unbiasable when a friend
    // or private entry function carries the `#[randomness]` annotation.
    let func = interp
        .load_function(module_id, function_name, ty_args)
        .map_err(MoveExecutionFailure::RuntimeError)?;
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
    call.run().map_err(MoveExecutionFailure::RuntimeError)
}
