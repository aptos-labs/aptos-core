// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Module defines validation of transaction arguments.
//!
//! TODO: we should not only validate the types but also the actual values, e.g.
//! for strings whether they consist of correct characters.

use crate::{
    aptos_vm::SerializedSigners,
    move_vm_ext::{AptosMoveResolver, SessionExt},
    VMStatus,
};
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    file_format::FunctionDefinitionIndex,
    file_format_common::read_uleb128_as_u64,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, PACK, PUBLIC_STRUCT_DELIMITER},
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_runtime::{
    execution_tracing::NoOpTraceRecorder, module_traversal::TraversalContext, LoadedFunction,
    LoadedFunctionOwner, Loader, RuntimeEnvironment,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::{StructLayout, Type, TypeParamMap},
};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashMap},
    io::{Cursor, Read},
    sync::Arc,
};

pub(crate) struct FunctionId {
    module_id: ModuleId,
    func_name: &'static IdentStr,
}

type ConstructorMap = Lazy<BTreeMap<String, FunctionId>>;
static OLD_ALLOWED_STRUCTS: ConstructorMap = Lazy::new(|| {
    [("0x1::string::String", FunctionId {
        module_id: ModuleId::new(AccountAddress::ONE, Identifier::from(ident_str!("string"))),
        func_name: ident_str!("utf8"),
    })]
    .into_iter()
    .map(|(s, validator)| (s.to_string(), validator))
    .collect()
});

static NEW_ALLOWED_STRUCTS: ConstructorMap = Lazy::new(|| {
    [
        ("0x1::string::String", FunctionId {
            module_id: ModuleId::new(AccountAddress::ONE, Identifier::from(ident_str!("string"))),
            func_name: ident_str!("utf8"),
        }),
        ("0x1::object::Object", FunctionId {
            module_id: ModuleId::new(AccountAddress::ONE, Identifier::from(ident_str!("object"))),
            func_name: ident_str!("address_to_object"),
        }),
        ("0x1::option::Option", FunctionId {
            module_id: ModuleId::new(AccountAddress::ONE, Identifier::from(ident_str!("option"))),
            func_name: ident_str!("from_vec"),
        }),
        ("0x1::fixed_point32::FixedPoint32", FunctionId {
            module_id: ModuleId::new(
                AccountAddress::ONE,
                Identifier::from(ident_str!("fixed_point32")),
            ),
            func_name: ident_str!("create_from_raw_value"),
        }),
        ("0x1::fixed_point64::FixedPoint64", FunctionId {
            module_id: ModuleId::new(
                AccountAddress::ONE,
                Identifier::from(ident_str!("fixed_point64")),
            ),
            func_name: ident_str!("create_from_raw_value"),
        }),
    ]
    .into_iter()
    .map(|(s, validator)| (s.to_string(), validator))
    .collect()
});

pub(crate) fn get_allowed_structs(
    are_struct_constructors_enabled: bool,
) -> &'static ConstructorMap {
    if are_struct_constructors_enabled {
        &NEW_ALLOWED_STRUCTS
    } else {
        &OLD_ALLOWED_STRUCTS
    }
}

/// Cache for loaded pack functions to avoid duplicate loading and gas charging.
/// Maps "module_id::function_name" -> (Arc<Module>, Arc<Function>)
type PackFunctionCache =
    HashMap<String, (Arc<move_vm_runtime::Module>, Arc<move_vm_runtime::Function>)>;

/// Creates a cache key for a pack function.
fn make_pack_fn_cache_key(module_id: &ModuleId, function_name: &str) -> String {
    format!("{}::{}", module_id.short_str_lossless(), function_name)
}

/// Constructs the pack function name for a struct: pack$<struct_name>
fn make_struct_pack_fn_name(struct_name: &Identifier) -> String {
    format!("{}{}{}", PACK, PUBLIC_STRUCT_DELIMITER, struct_name)
}

/// Constructs the pack function name for an enum variant: pack$<enum_name>$<variant_name>
fn make_variant_pack_fn_name(enum_name: &Identifier, variant_name: &Identifier) -> String {
    format!(
        "{}{}{}{}{}",
        PACK, PUBLIC_STRUCT_DELIMITER, enum_name, PUBLIC_STRUCT_DELIMITER, variant_name
    )
}

/// Validate and generate args for entry function
/// validation includes:
/// 1. return signature is empty
/// 2. number of signers is same as the number of senders
/// 3. check arg types are allowed after signers
///
/// after validation, add senders and non-signer arguments to generate the final args
pub(crate) fn validate_combine_signer_and_txn_args(
    session: &mut SessionExt<impl AptosMoveResolver>,
    loader: &impl Loader,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    serialized_signers: &SerializedSigners,
    args: Vec<Vec<u8>>,
    func: &LoadedFunction,
    are_struct_constructors_enabled: bool,
) -> Result<Vec<Vec<u8>>, VMStatus> {
    let _timer = VM_TIMER.timer_with_label("AptosVM::validate_combine_signer_and_txn_args");

    // Entry function should not return.
    if !func.return_tys().is_empty() {
        return Err(VMStatus::error(
            StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
            None,
        ));
    }
    let mut signer_param_cnt = 0;
    // find all signer params at the beginning
    for ty in func.param_tys() {
        if ty.is_signer_or_signer_ref() {
            signer_param_cnt += 1;
        }
    }

    let allowed_structs = get_allowed_structs(are_struct_constructors_enabled);
    let ty_builder = &loader.runtime_environment().vm_config().ty_builder;

    // Create pack function cache to avoid duplicate loading and gas charging
    let mut pack_fn_cache = PackFunctionCache::new();

    // Need to keep this here to ensure we return the historic correct error code for replay
    // During validation, pack functions are loaded and cached
    for ty in func.param_tys()[signer_param_cnt..].iter() {
        let subst_res = ty_builder.create_ty_with_subst(ty, func.ty_args());
        let ty = subst_res.map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
        let valid = is_valid_txn_arg(
            loader,
            gas_meter,
            traversal_context,
            &ty,
            allowed_structs,
            &mut pack_fn_cache,
        );
        if !valid {
            return Err(VMStatus::error(
                StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
                None,
            ));
        }
    }

    if (signer_param_cnt + args.len()) != func.param_tys().len() {
        return Err(VMStatus::error(
            StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH,
            None,
        ));
    }

    // If the invoked function expects one or more signers, we need to check that the number of
    // signers actually passed is matching first to maintain backward compatibility before
    // moving on to the validation of non-signer args.
    // the number of txn senders should be the same number of signers
    let sender_signers = serialized_signers.senders();
    if signer_param_cnt > 0 && sender_signers.len() != signer_param_cnt {
        return Err(VMStatus::error(
            StatusCode::NUMBER_OF_SIGNER_ARGUMENTS_MISMATCH,
            None,
        ));
    }

    // This also validates that the args are valid. If they are structs, they have to be allowed
    // and must be constructed successfully. If construction fails, this would fail with a
    // FAILED_TO_DESERIALIZE_ARGUMENT error.
    // During construction, cached pack functions are used (no duplicate loading/gas charging)
    let args = construct_args(
        session,
        loader,
        gas_meter,
        traversal_context,
        &func.param_tys()[signer_param_cnt..],
        args,
        func.ty_args(),
        allowed_structs,
        false,
        &pack_fn_cache,
    )?;

    // Combine signer and non-signer arguments.
    let combined_args = if signer_param_cnt == 0 {
        args
    } else {
        sender_signers.into_iter().chain(args).collect()
    };
    Ok(combined_args)
}

/// Returns true if the argument is valid (that is, it is a primitive type or a struct with a
/// known constructor function). Otherwise, (for structs without constructors, signers or
/// references) returns false. An error is returned in cases when a struct type is encountered and
/// its name cannot be queried for some reason.
///
/// Pack functions for public structs/enums are loaded and cached during validation.
pub(crate) fn is_valid_txn_arg<L: Loader>(
    loader: &L,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    ty: &Type,
    allowed_structs: &ConstructorMap,
    pack_fn_cache: &mut PackFunctionCache,
) -> bool {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match ty {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | I8 | I16 | I32 | I64 | I128 | I256
        | Address => true,
        Vector(inner) => is_valid_txn_arg(
            loader,
            gas_meter,
            traversal_context,
            inner,
            allowed_structs,
            pack_fn_cache,
        ),
        Struct { .. } | StructInstantiation { .. } => {
            is_allowed_struct(loader.runtime_environment(), ty, allowed_structs)
                || is_public_copy_struct(
                    loader,
                    gas_meter,
                    traversal_context,
                    ty,
                    allowed_structs,
                    pack_fn_cache,
                )
        },
        Signer | Reference(_) | MutableReference(_) | TyParam(_) | Function { .. } => false,
    }
}

/// Checks if a struct is in the allowed structs list (whitelisted structs like String, Object, etc.)
fn is_allowed_struct(
    runtime_environment: &RuntimeEnvironment,
    ty: &Type,
    allowed_structs: &ConstructorMap,
) -> bool {
    // Note: Original behavior was to return false even if the module loading fails (e.g.,
    //       if struct does not exist. This preserves it.
    runtime_environment
        .get_struct_name(ty)
        .ok()
        .flatten()
        .is_some_and(|(module_id, identifier)| {
            allowed_structs.contains_key(&format!(
                "{}::{}",
                module_id.short_str_lossless(),
                identifier
            ))
        })
}

/// Helper to recursively validate field types with optional type parameter substitution.
/// Used for both struct fields and enum variant fields.
fn validate_fields_recursively<L: Loader>(
    loader: &L,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    fields: &[(Identifier, Type)],
    ty_args: Option<&[Type]>,
    allowed_structs: &ConstructorMap,
    pack_fn_cache: &mut PackFunctionCache,
) -> bool {
    if let Some(ty_args) = ty_args {
        // Generic: substitute type parameters with actual type arguments
        let ty_builder = &loader.runtime_environment().vm_config().ty_builder;
        for (_field_name, field_ty) in fields {
            // Use ty_builder to perform type substitution (replaces TyParam with actual types)
            let substituted_ty = match ty_builder.create_ty_with_subst(field_ty, ty_args) {
                Ok(ty) => ty,
                Err(_) => return false,
            };

            if !is_valid_txn_arg(
                loader,
                gas_meter,
                traversal_context,
                &substituted_ty,
                allowed_structs,
                pack_fn_cache,
            ) {
                return false;
            }
        }
    } else {
        // Non-generic: validate field types directly
        for (_field_name, field_ty) in fields {
            if !is_valid_txn_arg(
                loader,
                gas_meter,
                traversal_context,
                field_ty,
                allowed_structs,
                pack_fn_cache,
            ) {
                return false;
            }
        }
    }
    true
}

/// Checks if a struct/enum is public (has a public pack function) and has the copy ability.
/// For structs: checks for public `pack$<struct_name>` function and caches it
/// For enums: checks for at least one public `pack$<enum_name>$<variant>` function and caches it
fn is_public_copy_struct<L: Loader>(
    loader: &L,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    ty: &Type,
    allowed_structs: &ConstructorMap,
    pack_fn_cache: &mut PackFunctionCache,
) -> bool {
    // Check if public struct/enum arguments feature is enabled
    if !loader
        .runtime_environment()
        .vm_config()
        .enable_public_struct_args
    {
        return false;
    }

    // First check if the struct has the copy ability
    let has_copy = match ty.abilities() {
        Ok(abilities) => abilities.has_copy(),
        Err(_) => return false,
    };

    if !has_copy {
        return false;
    }

    // Get the struct name and module ID
    let (module_id, struct_name) = match loader.runtime_environment().get_struct_name(ty) {
        Ok(Some((module_id, identifier))) => (module_id, identifier),
        _ => return false,
    };

    // Load the struct definition to check if it's a struct or enum
    let (struct_idx, ty_args) = match ty {
        Type::Struct { idx, .. } => (*idx, None),
        Type::StructInstantiation { idx, ty_args, .. } => (*idx, Some(ty_args.as_ref())),
        _ => return false,
    };

    let struct_type = match loader.load_struct_definition(gas_meter, traversal_context, &struct_idx)
    {
        Ok(st) => st,
        Err(_) => return false,
    };

    // Check based on whether it's a struct or enum
    match &struct_type.layout {
        StructLayout::Single(fields) => {
            // For structs, load and cache pack$<struct_name> function
            let pack_fn_name = make_struct_pack_fn_name(&struct_name);
            if load_and_cache_pack_function(
                loader,
                gas_meter,
                traversal_context,
                &module_id,
                &pack_fn_name,
                pack_fn_cache,
            )
            .is_none()
            {
                return false;
            }

            // Recursively validate all field types
            validate_fields_recursively(
                loader,
                gas_meter,
                traversal_context,
                fields,
                ty_args.map(|v| &**v),
                allowed_structs,
                pack_fn_cache,
            )
        },
        StructLayout::Variants(variants) => {
            // For enums, ALL variants must have public pack functions
            // Also validate all field types in all variants
            for (variant_name, fields) in variants {
                let pack_fn_name = make_variant_pack_fn_name(&struct_name, variant_name);
                if load_and_cache_pack_function(
                    loader,
                    gas_meter,
                    traversal_context,
                    &module_id,
                    &pack_fn_name,
                    pack_fn_cache,
                )
                .is_none()
                {
                    // Missing pack function for this variant - not public
                    return false;
                }

                // Validate all field types for this variant
                if !validate_fields_recursively(
                    loader,
                    gas_meter,
                    traversal_context,
                    fields,
                    ty_args.map(|v| &**v),
                    allowed_structs,
                    pack_fn_cache,
                ) {
                    return false;
                }
            }

            true
        },
    }
}

/// Loads a pack function and caches it, or returns the cached function if already loaded.
/// Only charges gas once per unique function.
/// Returns Some((module, function)) if the function exists and is public, None otherwise.
fn load_and_cache_pack_function<L: Loader>(
    loader: &L,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    module_id: &ModuleId,
    function_name: &str,
    cache: &mut PackFunctionCache,
) -> Option<(Arc<move_vm_runtime::Module>, Arc<move_vm_runtime::Function>)> {
    let cache_key = make_pack_fn_cache_key(module_id, function_name);

    // Check cache first - no gas charged for cache hit
    if let Some(cached) = cache.get(&cache_key) {
        return Some(cached.clone());
    }

    // Not in cache, load it (charges gas once)
    let func_name = match Identifier::new(function_name) {
        Ok(name) => name,
        Err(_) => return None,
    };

    match loader.load_function_definition(gas_meter, traversal_context, module_id, &func_name) {
        Ok((module, function)) => {
            if function.is_public() {
                // Cache the loaded module and function for later use
                cache.insert(cache_key, (module.clone(), function.clone()));
                Some((module, function))
            } else {
                None
            }
        },
        Err(_) => None,
    }
}

/// Constructs a public copy struct or enum by calling its cached pack function.
/// This is similar to validate_and_construct but for public copy structs/enums.
/// For structs: calls pack$<struct_name>
/// For enums: reads variant index from BCS, then calls pack$<enum_name>$<variant_name>
/// The pack function must have been previously loaded and cached during validation.
fn construct_public_copy_struct(
    session: &mut SessionExt<impl AptosMoveResolver>,
    loader: &impl Loader,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    expected_type: &Type,
    module_id: &ModuleId,
    struct_name: &Identifier,
    allowed_structs: &ConstructorMap,
    cursor: &mut Cursor<&[u8]>,
    initial_cursor_len: usize,
    max_invocations: &mut u64,
    pack_fn_cache: &PackFunctionCache,
) -> Result<Vec<u8>, VMStatus> {
    if *max_invocations == 0 {
        return Err(VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            None,
        ));
    }
    *max_invocations -= 1;

    // Check if public struct/enum arguments feature is enabled
    if !loader
        .runtime_environment()
        .vm_config()
        .enable_public_struct_args
    {
        return Err(VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            Some("Public struct/enum arguments feature is not enabled".to_string()),
        ));
    }

    // Load the struct definition to check if it's a struct or enum
    let struct_idx = match expected_type {
        Type::Struct { idx, .. } | Type::StructInstantiation { idx, .. } => *idx,
        _ => return Err(invalid_signature()),
    };

    let struct_type = loader
        .load_struct_definition(gas_meter, traversal_context, &struct_idx)
        .map_err(|e| {
            VMStatus::error(
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                Some(format!("Failed to load struct definition: {:?}", e)),
            )
        })?;

    // Determine pack function name based on struct vs enum
    let pack_fn_name = match &struct_type.layout {
        StructLayout::Single(_) => {
            // For structs: pack$<struct_name>
            make_struct_pack_fn_name(struct_name)
        },
        StructLayout::Variants(variants) => {
            // For enums: read variant index from BCS, then pack$<enum_name>$<variant_name>
            let variant_idx_usize = get_len(cursor)?;

            // Validate that variant index is within u16 range
            if variant_idx_usize > u16::MAX as usize {
                return Err(VMStatus::error(
                    StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                    Some(format!(
                        "Variant index {} exceeds maximum value {}",
                        variant_idx_usize,
                        u16::MAX
                    )),
                ));
            }

            // Validate that variant index is within bounds
            if variant_idx_usize >= variants.len() {
                return Err(VMStatus::error(
                    StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                    Some(format!(
                        "Invalid variant index {} for enum {} with {} variants",
                        variant_idx_usize,
                        struct_name,
                        variants.len()
                    )),
                ));
            }

            let variant_name = &variants[variant_idx_usize].0;
            make_variant_pack_fn_name(struct_name, variant_name)
        },
    };

    // Try to get pack function from cache first (for entry functions with pre-validation)
    // If not in cache, load it (for view functions without pre-validation)
    let cache_key = make_pack_fn_cache_key(module_id, &pack_fn_name);
    let (module, func_arc) = if let Some(cached) = pack_fn_cache.get(&cache_key) {
        // Cache hit - use cached module and function (no gas charge)
        cached.clone()
    } else {
        // Cache miss - load function on demand (charges gas, used by view functions)
        let pack_fn_ident = Identifier::new(pack_fn_name.clone()).map_err(|_| {
            VMStatus::error(
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                Some(format!("Invalid pack function name: {}", pack_fn_name)),
            )
        })?;
        let (module, func) = loader
            .load_function_definition(gas_meter, traversal_context, module_id, &pack_fn_ident)
            .map_err(|_| {
                VMStatus::error(
                    StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                    Some(format!("Pack function {} not found", pack_fn_name)),
                )
            })?;
        if !func.is_public() {
            return Err(VMStatus::error(
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                Some(format!("Pack function {} is not public", pack_fn_name)),
            ));
        }
        (module, func)
    };

    // Build LoadedFunction with type arguments
    let num_ty_args = func_arc.ty_param_abilities().len();
    let mut ty_args = Vec::with_capacity(num_ty_args);

    if num_ty_args > 0 {
        // Verify pack function has a return type
        // Note that bytecode verifier should already check the return value but add this for extra safety
        if func_arc.return_tys().is_empty() {
            return Err(VMStatus::error(
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                Some(format!(
                    "Pack function {} must return a value",
                    pack_fn_name
                )),
            ));
        }

        // Match type parameters from expected return type to extract ty_args
        let mut map = TypeParamMap::default();
        if !map.match_ty(&func_arc.return_tys()[0], expected_type) {
            return Err(VMStatus::error(
                StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
                Some(format!(
                    "Pack function {}::{} return type does not match expected type",
                    module_id, pack_fn_name
                )),
            ));
        }

        for i in 0..num_ty_args {
            ty_args.push(map.get_ty_param(i as u16).ok_or_else(|| {
                VMStatus::error(
                    StatusCode::INTERNAL_TYPE_ERROR,
                    Some(format!(
                        "Unable to instantiate generic pack function {}::{}",
                        module_id, pack_fn_name
                    )),
                )
            })?);
        }

        Type::verify_ty_arg_abilities(func_arc.ty_param_abilities(), &ty_args).map_err(|e| {
            e.finish(Location::Module(module_id.clone()))
                .into_vm_status()
        })?;
    }

    let ty_args_id = loader
        .runtime_environment()
        .ty_pool()
        .intern_ty_args(&ty_args);

    let function = LoadedFunction {
        owner: LoadedFunctionOwner::Module(module),
        ty_args,
        ty_args_id,
        function: func_arc,
    };

    // Construct arguments for each parameter (struct/variant fields)
    let mut args = vec![];
    let ty_builder = &loader.runtime_environment().vm_config().ty_builder;
    for param_ty in function.param_tys() {
        let mut arg = vec![];
        let arg_ty = ty_builder
            .create_ty_with_subst(param_ty, function.ty_args())
            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
        recursively_construct_arg(
            session,
            loader,
            gas_meter,
            traversal_context,
            &arg_ty,
            allowed_structs,
            cursor,
            initial_cursor_len,
            max_invocations,
            &mut arg,
            pack_fn_cache,
        )?;
        args.push(arg);
    }

    // Execute the pack function
    let serialized_result = session.execute_loaded_function(
        function,
        args,
        gas_meter,
        traversal_context,
        loader,
        &mut NoOpTraceRecorder,
    )?;

    let mut ret_vals = serialized_result.return_values;
    Ok(ret_vals
        .pop()
        .ok_or_else(|| {
            VMStatus::error(
                StatusCode::INTERNAL_TYPE_ERROR,
                Some(String::from("Pack function did not return value")),
            )
        })?
        .0)
}

// Construct arguments. Walk through the arguments and according to the signature
// construct arguments that require so.
// TODO: This needs a more solid story and a tighter integration with the VM.
pub(crate) fn construct_args(
    session: &mut SessionExt<impl AptosMoveResolver>,
    loader: &impl Loader,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    types: &[Type],
    args: Vec<Vec<u8>>,
    ty_args: &[Type],
    allowed_structs: &ConstructorMap,
    is_view: bool,
    pack_fn_cache: &PackFunctionCache,
) -> Result<Vec<Vec<u8>>, VMStatus> {
    // Perhaps in a future we should do proper gas metering here
    let mut res_args = vec![];
    if types.len() != args.len() {
        return Err(invalid_signature());
    }

    let ty_builder = &loader.runtime_environment().vm_config().ty_builder;
    for (ty, arg) in types.iter().zip(args) {
        let subst_res = ty_builder.create_ty_with_subst(ty, ty_args);
        let ty = subst_res.map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
        let arg = construct_arg(
            session,
            loader,
            gas_meter,
            traversal_context,
            &ty,
            allowed_structs,
            arg,
            is_view,
            pack_fn_cache,
        )?;
        res_args.push(arg);
    }
    Ok(res_args)
}

fn invalid_signature() -> VMStatus {
    VMStatus::error(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE, None)
}

fn construct_arg(
    session: &mut SessionExt<impl AptosMoveResolver>,
    loader: &impl Loader,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    ty: &Type,
    allowed_structs: &ConstructorMap,
    arg: Vec<u8>,
    is_view: bool,
    pack_fn_cache: &PackFunctionCache,
) -> Result<Vec<u8>, VMStatus> {
    use move_vm_types::loaded_data::runtime_types::Type::*;
    match ty {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | I8 | I16 | I32 | I64 | I128 | I256
        | Address => Ok(arg),
        Vector(_) | Struct { .. } | StructInstantiation { .. } => {
            let initial_cursor_len = arg.len();
            let mut cursor = Cursor::new(&arg[..]);
            let mut new_arg = vec![];
            let mut max_invocations = 10; // Read from config in the future
            recursively_construct_arg(
                session,
                loader,
                gas_meter,
                traversal_context,
                ty,
                allowed_structs,
                &mut cursor,
                initial_cursor_len,
                &mut max_invocations,
                &mut new_arg,
                pack_fn_cache,
            )?;
            // Check cursor has parsed everything
            // Unfortunately, is_empty is only enabled in nightly, so we check this way.
            if cursor.position() != initial_cursor_len as u64 {
                return Err(VMStatus::error(
                    StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                    Some(String::from(
                        "The serialized arguments to constructor contained extra data",
                    )),
                ));
            }
            Ok(new_arg)
        },
        Signer => {
            if is_view {
                Ok(arg)
            } else {
                Err(invalid_signature())
            }
        },
        Reference(_) | MutableReference(_) | TyParam(_) | Function { .. } => {
            Err(invalid_signature())
        },
    }
}

// A Cursor is used to recursively walk the serialized arg manually and correctly. In effect we
// are parsing the BCS serialized implicit constructor invocation tree, while serializing the
// constructed types into the output parameter arg.
pub(crate) fn recursively_construct_arg(
    session: &mut SessionExt<impl AptosMoveResolver>,
    loader: &impl Loader,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    ty: &Type,
    allowed_structs: &ConstructorMap,
    cursor: &mut Cursor<&[u8]>,
    initial_cursor_len: usize,
    max_invocations: &mut u64,
    arg: &mut Vec<u8>,
    pack_fn_cache: &PackFunctionCache,
) -> Result<(), VMStatus> {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match ty {
        Vector(inner) => {
            // get the vector length and iterate over each element
            let mut len = get_len(cursor)?;
            serialize_uleb128(len, arg);
            while len > 0 {
                recursively_construct_arg(
                    session,
                    loader,
                    gas_meter,
                    traversal_context,
                    inner,
                    allowed_structs,
                    cursor,
                    initial_cursor_len,
                    max_invocations,
                    arg,
                    pack_fn_cache,
                )?;
                len -= 1;
            }
        },
        Struct { .. } | StructInstantiation { .. } => {
            let runtime_env = loader.runtime_environment();
            let (module_id, identifier) = runtime_env
                .get_struct_name(ty)
                .map_err(|_| {
                    // Note: The original behaviour was to map all errors to an invalid signature
                    //       error, here we want to preserve it for now.
                    invalid_signature()
                })?
                .ok_or_else(invalid_signature)?;
            let full_name = format!("{}::{}", module_id.short_str_lossless(), identifier);

            if let Some(constructor) = allowed_structs.get(&full_name) {
                // Whitelisted struct - use constructor function
                // By appending the BCS to the output parameter we construct the correct BCS format
                // of the argument.
                arg.append(&mut validate_and_construct(
                    session,
                    loader,
                    gas_meter,
                    traversal_context,
                    ty,
                    constructor,
                    allowed_structs,
                    cursor,
                    initial_cursor_len,
                    max_invocations,
                    pack_fn_cache,
                )?);
            } else {
                // Must be a public copy struct (validation already checked)
                // Construct by calling the cached pack function
                arg.append(&mut construct_public_copy_struct(
                    session,
                    loader,
                    gas_meter,
                    traversal_context,
                    ty,
                    &module_id,
                    &identifier,
                    allowed_structs,
                    cursor,
                    initial_cursor_len,
                    max_invocations,
                    pack_fn_cache,
                )?);
            }
        },
        Bool | U8 | I8 => read_n_bytes(1, cursor, arg)?,
        U16 | I16 => read_n_bytes(2, cursor, arg)?,
        U32 | I32 => read_n_bytes(4, cursor, arg)?,
        U64 | I64 => read_n_bytes(8, cursor, arg)?,
        U128 | I128 => read_n_bytes(16, cursor, arg)?,
        U256 | I256 | Address => read_n_bytes(32, cursor, arg)?,
        Signer | Reference(_) | MutableReference(_) | TyParam(_) | Function { .. } => {
            return Err(invalid_signature())
        },
    };
    Ok(())
}

// A move function that constructs a type will return the BCS serialized representation of the
// constructed value. This is the correct data to pass as the argument to a function taking
// said struct as a parameter. In this function we execute the constructor constructing the
// value and returning the BCS serialized representation.
fn validate_and_construct(
    session: &mut SessionExt<impl AptosMoveResolver>,
    loader: &impl Loader,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    expected_type: &Type,
    constructor: &FunctionId,
    allowed_structs: &ConstructorMap,
    cursor: &mut Cursor<&[u8]>,
    initial_cursor_len: usize,
    max_invocations: &mut u64,
    pack_fn_cache: &PackFunctionCache,
) -> Result<Vec<u8>, VMStatus> {
    if *max_invocations == 0 {
        return Err(VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            None,
        ));
    }
    // HACK mitigation of performance attack
    // To maintain compatibility with vector<string> or so on, we need to allow unlimited strings.
    // So we do not count the string constructor against the max_invocations, instead we
    // shortcut the string case to avoid the performance attack.
    if constructor.func_name.as_str() == "utf8" {
        let constructor_error = || {
            // A slight hack, to prevent additional piping of the feature flag through all
            // function calls. We know the feature is active when more structs then just strings are
            // allowed.
            let are_struct_constructors_enabled = allowed_structs.len() > 1;
            if are_struct_constructors_enabled {
                PartialVMError::new(StatusCode::ABORTED)
                    .with_sub_status(1)
                    .at_code_offset(FunctionDefinitionIndex::new(0), 0)
                    .finish(Location::Module(constructor.module_id.clone()))
                    .into_vm_status()
            } else {
                VMStatus::error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, None)
            }
        };
        // Short cut for the utf8 constructor, which is a special case.
        let len = get_len(cursor)?;
        if cursor
            .position()
            .checked_add(len as u64)
            .is_none_or(|l| l > initial_cursor_len as u64)
        {
            // We need to make sure we do not allocate more bytes than
            // needed.
            return Err(VMStatus::error(
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                Some("String argument is too long".to_string()),
            ));
        }

        let mut arg = vec![];
        read_n_bytes(len, cursor, &mut arg)?;
        std::str::from_utf8(&arg).map_err(|_| constructor_error())?;
        return bcs::to_bytes(&arg)
            .map_err(|_| VMStatus::error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, None));
    } else {
        *max_invocations -= 1;
    }

    let function = load_constructor_function(
        loader,
        gas_meter,
        traversal_context,
        &constructor.module_id,
        constructor.func_name,
        expected_type,
    )?;
    let mut args = vec![];
    let ty_builder = &loader.runtime_environment().vm_config().ty_builder;
    for param_ty in function.param_tys() {
        let mut arg = vec![];
        let arg_ty = ty_builder
            .create_ty_with_subst(param_ty, function.ty_args())
            .unwrap();

        recursively_construct_arg(
            session,
            loader,
            gas_meter,
            traversal_context,
            &arg_ty,
            allowed_structs,
            cursor,
            initial_cursor_len,
            max_invocations,
            &mut arg,
            pack_fn_cache,
        )?;
        args.push(arg);
    }
    let serialized_result = session.execute_loaded_function(
        function,
        args,
        gas_meter,
        traversal_context,
        loader,
        // No need to record the trace for argument construction.
        &mut NoOpTraceRecorder,
    )?;
    let mut ret_vals = serialized_result.return_values;
    // We know ret_vals.len() == 1
    Ok(ret_vals
        .pop()
        .ok_or_else(|| {
            VMStatus::error(
                StatusCode::INTERNAL_TYPE_ERROR,
                Some(String::from("Constructor did not return value")),
            )
        })?
        .0)
}

// String is a vector of bytes, so both string and vector carry a length in the serialized format.
// Length of vectors in BCS uses uleb128 as a compression format.
fn get_len(cursor: &mut Cursor<&[u8]>) -> Result<usize, VMStatus> {
    match read_uleb128_as_u64(cursor) {
        Err(_) => Err(VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            None,
        )),
        Ok(len) => Ok(len as usize),
    }
}

fn serialize_uleb128(mut x: usize, dest: &mut Vec<u8>) {
    // TODO perhaps reuse the code from move_binary_format::file_format_common if it's public
    while x >= 128 {
        dest.push((x | 128) as u8);
        x >>= 7;
    }
    dest.push(x as u8);
}

fn read_n_bytes(n: usize, src: &mut Cursor<&[u8]>, dest: &mut Vec<u8>) -> Result<(), VMStatus> {
    let deserialization_error = |msg: &str| -> VMStatus {
        VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            Some(msg.to_string()),
        )
    };
    let len = dest.len();

    // It is safer to limit the length under some big (but still reasonable
    // number).
    const MAX_NUM_BYTES: usize = 1_000_000;
    if len.checked_add(n).is_none_or(|s| s > MAX_NUM_BYTES) {
        return Err(deserialization_error(&format!(
            "Couldn't read bytes: maximum limit of {} bytes exceeded",
            MAX_NUM_BYTES
        )));
    }

    // Ensure we have enough capacity for resizing.
    dest.try_reserve(len + n)
        .map_err(|e| deserialization_error(&format!("Couldn't read bytes: {}", e)))?;
    dest.resize(len + n, 0);
    src.read_exact(&mut dest[len..])
        .map_err(|_| deserialization_error("Couldn't read bytes"))
}

fn load_constructor_function(
    loader: &impl Loader,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    module_id: &ModuleId,
    function_name: &IdentStr,
    expected_return_ty: &Type,
) -> VMResult<LoadedFunction> {
    if !module_id.address().is_special() {
        let msg = format!(
            "Constructor function {}::{}::{} has a non-special address!",
            module_id.address(),
            module_id.name(),
            function_name
        );
        let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
            .with_message(msg)
            .finish(Location::Undefined);
        return Err(err);
    }

    let module_loc = || Location::Module(module_id.clone());

    let (module, function) =
        loader.load_function_definition(gas_meter, traversal_context, module_id, function_name)?;

    if function.return_tys().len() != 1 {
        // For functions that are marked constructor this should not happen.
        return Err(PartialVMError::new(StatusCode::ABORTED).finish(module_loc()));
    }

    let mut map = TypeParamMap::default();
    if !map.match_ty(&function.return_tys()[0], expected_return_ty) {
        // For functions that are marked constructor this should not happen.
        return Err(
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE).finish(module_loc()),
        );
    }

    // Construct the type arguments from the match.
    let num_ty_args = function.ty_param_abilities().len();
    let mut ty_args = Vec::with_capacity(num_ty_args);
    for i in 0..num_ty_args {
        ty_args.push(map.get_ty_param(i as u16).ok_or_else(|| {
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE).finish(module_loc())
        })?);
    }

    Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
        .map_err(|e| e.finish(module_loc()))?;
    let ty_args_id = loader
        .runtime_environment()
        .ty_pool()
        .intern_ty_args(&ty_args);

    Ok(LoadedFunction {
        owner: LoadedFunctionOwner::Module(module),
        ty_args,
        ty_args_id,
        function,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(true)]
    #[test_case(false)]
    fn test_constructor_functions_always_have_special_address(
        are_struct_constructors_enabled: bool,
    ) {
        let constructors = get_allowed_structs(are_struct_constructors_enabled);
        for function_id in constructors.values() {
            assert!(function_id.module_id.address().is_special());
        }
    }
}
