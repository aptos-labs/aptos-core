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
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
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
    execution_tracing::NoOpTraceRecorder, module_traversal::TraversalContext, Function,
    LoadedFunction, LoadedFunctionOwner, Loader, Module,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::{StructLayout, Type, TypeParamMap},
};
use once_cell::sync::Lazy;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    sync::Arc,
};

/// Maximum number of pack function invocations allowed during struct/enum argument construction.
/// Acts as a DoS protection against deeply nested or complex struct arguments when only
/// whitelisted structs (e.g. `String`, `Object`) are permitted.
const MAX_PACK_INVOCATIONS: u64 = 10;

/// Maximum number of pack function invocations when the `PUBLIC_STRUCT_ENUM_ARGS` feature is
/// enabled, which allows arbitrary public copy structs/enums. The higher limit accommodates
/// more complex argument structures while still bounding worst-case construction cost.
const MAX_PACK_INVOCATIONS_WITH_PUBLIC_STRUCT_ARGS: u64 = 100;

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
type PackFunctionCache = ahash::AHashMap<String, (Arc<Module>, Arc<Function>)>;

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

    // Create pack function cache shared between validation and construction.
    let mut pack_fn_cache = PackFunctionCache::new();

    // Need to keep this here to ensure we return the historic correct error code for replay.
    // Pass param_tys()[signer_param_cnt..] so that any signer in a non-leading position
    // (e.g. `(signer, u64, signer)`) is included in the slice and fails is_valid_txn_arg,
    // preserving the historical INVALID_MAIN_FUNCTION_SIGNATURE error for such cases.
    validate_non_signer_param_tys(
        loader,
        &func.param_tys()[signer_param_cnt..],
        func.ty_args(),
        allowed_structs,
    )
    .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;

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
        &mut pack_fn_cache,
    )?;

    // Combine signer and non-signer arguments.
    let combined_args = if signer_param_cnt == 0 {
        args
    } else {
        sender_signers.into_iter().chain(args).collect()
    };
    Ok(combined_args)
}

/// Returns true if the argument type is valid as a transaction argument: primitives, vectors of
/// valid types, whitelisted structs (String, Object, Option, ...), and public copy structs/enums
/// with struct APIs. Pack functions and field types are not loaded here; they are validated lazily
/// at construction time.
pub(crate) fn is_valid_txn_arg<L: Loader>(
    loader: &L,
    ty: &Type,
    allowed_structs: &ConstructorMap,
) -> bool {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match ty {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | I8 | I16 | I32 | I64 | I128 | I256
        | Address => true,
        Vector(inner) => is_valid_txn_arg(loader, inner, allowed_structs),
        Struct { .. } | StructInstantiation { .. } => {
            is_allowed_struct(loader, ty, allowed_structs) || is_public_struct_arg(loader, ty)
        },
        Signer | Reference(_) | MutableReference(_) | TyParam(_) | Function { .. } => false,
    }
}

/// Validates that all parameters in `param_tys` are valid transaction arguments.
///
/// Callers should pass `param_tys[signer_param_cnt..]` so that leading signers are
/// already excluded. Any signer type that appears in the slice (i.e., in a non-leading
/// position) fails `is_valid_txn_arg` and returns `INVALID_MAIN_FUNCTION_SIGNATURE`,
/// preserving the historical error code for functions with non-leading signers.
pub(crate) fn validate_non_signer_param_tys<L: Loader>(
    loader: &L,
    param_tys: &[Type],
    ty_args: &[Type],
    allowed_structs: &ConstructorMap,
) -> PartialVMResult<()> {
    let ty_builder = &loader.runtime_environment().vm_config().ty_builder;
    for ty in param_tys.iter() {
        let ty = ty_builder.create_ty_with_subst(ty, ty_args).map_err(|e| {
            PartialVMError::new(e.finish(Location::Undefined).into_vm_status().status_code())
        })?;
        if !is_valid_txn_arg(loader, &ty, allowed_structs) {
            return Err(PartialVMError::new(
                StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
            ));
        }
    }
    Ok(())
}

/// Checks if a struct is in the allowed structs list (whitelisted structs like String, Object,
/// Option, etc.).
///
/// Type arguments are intentionally not validated here. For example, `Option<PrivateStruct>` is
/// allowed to pass validation because `None` is a legitimate value: the only valid values the
/// caller can actually construct are those whose inner types are themselves constructable, which
/// is enforced at construction time.
fn is_allowed_struct<L: Loader>(loader: &L, ty: &Type, allowed_structs: &ConstructorMap) -> bool {
    loader
        .runtime_environment()
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

/// Checks if a struct/enum type may potentially be a valid public struct/enum argument.
/// Only the feature flag is checked here, plus an early rejection of `key` structs (resources).
/// Copy ability is NOT checked at this stage because doing so would incorrectly reject generic
/// structs like Container<NoCopyData> at the type level even when the actual value
/// (e.g. Empty variant) is safe to construct.
///
/// The definitive copy check is deferred to `construct_public_copy_struct`, which uses the
/// struct definition's *declared* copy ability (not the instantiated type's ability). This
/// ensures that:
/// - Container<NoCopyData>::Empty succeeds (Container declares copy; no inner value to check).
/// - Container<NoCopyData>::Value fails (NoCopyData field has no declared copy).
fn is_public_struct_arg<L: Loader>(loader: &L, ty: &Type) -> bool {
    if !loader
        .runtime_environment()
        .vm_config()
        .enable_public_struct_args
    {
        return false;
    }
    // Reject key structs (resources) early — they live in global storage and are not
    // valid as transaction arguments even if they also have copy.
    !ty.abilities().is_ok_and(|a| a.has_key())
}

/// Loads a pack function and caches it, or returns the cached function if already loaded.
/// Only charges gas once per unique function.
/// Returns Some((module, function)) if the function exists and is public, None otherwise.
/// Constructs a public copy struct or enum by calling its pack function.
/// For structs: calls pack$<struct_name>
/// For enums: reads variant index from BCS, then calls pack$<enum_name>$<variant_name>
/// The pack function is loaded on demand and cached for reuse within the same transaction.
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
    invocations_remaining: &mut u64,
    pack_fn_cache: &mut PackFunctionCache,
) -> Result<Vec<u8>, VMStatus> {
    if *invocations_remaining == 0 {
        return Err(VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            None,
        ));
    }
    *invocations_remaining -= 1;

    // Check if public struct/enum arguments feature is enabled.
    // If not, this struct type is not acceptable as a transaction argument.
    if !loader
        .runtime_environment()
        .vm_config()
        .enable_public_struct_args
    {
        return Err(invalid_signature());
    }

    // Load the struct definition — needed both for the copy check and to determine layout.
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

    // Require copy using the struct's *declared* abilities (not the instantiated type's).
    // This is intentional: Container<T> declares copy even when T lacks it, so
    // Container<NoCopyData>::Empty is accepted (no inner value to construct). The NoCopyData
    // field is checked when recursively constructing a Value variant. A struct whose definition
    // itself lacks copy (like NoCopyData) is always rejected here.
    if !struct_type.abilities.has_copy() {
        return Err(invalid_signature());
    }

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
            .map_err(|_| invalid_signature())?;
        if !func.is_public() || !func.is_pack_or_pack_variant() {
            return Err(invalid_signature());
        }
        pack_fn_cache.insert(cache_key, (module.clone(), func.clone()));
        (module, func)
    };

    // Build LoadedFunction with type arguments
    let num_ty_args = func_arc.ty_param_abilities().len();
    let mut ty_args = Vec::with_capacity(num_ty_args);

    if num_ty_args > 0 {
        // Verify pack function has exactly one return type
        // Note that bytecode verifier should already check the return value but add this for extra safety
        if func_arc.return_tys().len() != 1 {
            return Err(VMStatus::error(
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                Some(format!(
                    "Pack function {} must return exactly one value",
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
            invocations_remaining,
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
    pack_fn_cache: &mut PackFunctionCache,
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
    pack_fn_cache: &mut PackFunctionCache,
) -> Result<Vec<u8>, VMStatus> {
    use move_vm_types::loaded_data::runtime_types::Type::*;
    match ty {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | I8 | I16 | I32 | I64 | I128 | I256
        | Address => Ok(arg),
        Vector(_) | Struct { .. } | StructInstantiation { .. } => {
            let initial_cursor_len = arg.len();
            let mut cursor = Cursor::new(&arg[..]);
            let mut new_arg = vec![];
            // Increase invocation
            let mut invocations_remaining = if loader
                .runtime_environment()
                .vm_config()
                .enable_public_struct_args
            {
                MAX_PACK_INVOCATIONS_WITH_PUBLIC_STRUCT_ARGS
            } else {
                MAX_PACK_INVOCATIONS
            };
            recursively_construct_arg(
                session,
                loader,
                gas_meter,
                traversal_context,
                ty,
                allowed_structs,
                &mut cursor,
                initial_cursor_len,
                &mut invocations_remaining,
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
    invocations_remaining: &mut u64,
    arg: &mut Vec<u8>,
    pack_fn_cache: &mut PackFunctionCache,
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
                    invocations_remaining,
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
                    invocations_remaining,
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
                    invocations_remaining,
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
    invocations_remaining: &mut u64,
    pack_fn_cache: &mut PackFunctionCache,
) -> Result<Vec<u8>, VMStatus> {
    if *invocations_remaining == 0 {
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
        *invocations_remaining -= 1;
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
            invocations_remaining,
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
