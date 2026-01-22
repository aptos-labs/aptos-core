// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    move_vm_ext::{AptosMoveResolver, SessionExt},
    verifier::{transaction_arg_validation, transaction_arg_validation::get_allowed_structs},
};
use aptos_types::vm::module_metadata::RuntimeModuleMetadataV1;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{identifier::IdentStr, vm_status::StatusCode};
use move_vm_runtime::{
    module_traversal::{TraversalContext, TraversalStorage},
    LoadedFunction, Loader,
};
use move_vm_types::{
    gas::{GasMeter, UnmeteredGasMeter},
    loaded_data::runtime_types::Type,
};

/// Based on the function attributes in the module metadata, determine whether a
/// function is a view function.
pub fn determine_is_view(
    module_metadata: Option<&RuntimeModuleMetadataV1>,
    fun_name: &IdentStr,
) -> bool {
    if let Some(data) = module_metadata {
        data.fun_attributes
            .get(fun_name.as_str())
            .map(|attrs| attrs.iter().any(|attr| attr.is_view_function()))
            .unwrap_or_default()
    } else {
        false
    }
}

/// Validate view function call. This checks whether the function is marked as a view
/// function, and validates the arguments.
pub(crate) fn validate_view_function(
    session: &mut SessionExt<impl AptosMoveResolver>,
    loader: &impl Loader,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    args: Vec<Vec<u8>>,
    fun_name: &IdentStr,
    func: &LoadedFunction,
    module_metadata: Option<&RuntimeModuleMetadataV1>,
    struct_constructors_feature: bool,
) -> PartialVMResult<Vec<Vec<u8>>> {
    // Must be marked as view function.
    let is_view = determine_is_view(module_metadata, fun_name);
    if !is_view {
        return Err(
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                .with_message("function not marked as view function".to_string()),
        );
    }

    // Must return values.
    if func.return_tys().is_empty() {
        return Err(
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                .with_message("view function must return values".to_string()),
        );
    }

    let allowed_structs = get_allowed_structs(struct_constructors_feature);
    // Create a mutable pack function cache and pre-populate it by validating parameter types.
    // This avoids repeated pack function loads during argument construction, especially for
    // vectors of structs (e.g., vector<Point> with 100 elements would otherwise load the
    // pack function 100 times, charging gas each time under lazy loading).
    let mut pack_fn_cache = ahash::AHashMap::new();
    let ty_builder = &loader.runtime_environment().vm_config().ty_builder;

    // Check lazy loading once so both the pre-validation and construction branches agree.
    let is_lazy = loader.is_lazy_loading_enabled();
    if is_lazy {
        // Lazy loading: pack function loads are metered, so use the caller's gas meter.
        for ty in func.param_tys().iter() {
            let ty = ty_builder
                .create_ty_with_subst(ty, func.ty_args())
                .map_err(|e| {
                    let vm_status = e
                        .finish(move_binary_format::errors::Location::Undefined)
                        .into_vm_status();
                    PartialVMError::new(vm_status.status_code())
                })?;
            // Signer params are passed through as-is in view functions (see construct_arg),
            // so skip them here — is_valid_txn_arg returns false for Signer but that is
            // intentional only for entry functions.
            if ty == Type::Signer {
                continue;
            }
            if !transaction_arg_validation::is_valid_txn_arg(
                loader,
                gas_meter,
                traversal_context,
                &ty,
                allowed_structs,
                &mut pack_fn_cache,
            ) {
                return Err(
                    PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                        .with_message("invalid argument type for view function".to_string()),
                );
            }
        }
    } else {
        // Eager loading: argument construction uses UnmeteredGasMeter, so pre-validation
        // must also be unmetered to avoid unexpected gas charges.
        let eager_storage = TraversalStorage::new();
        let mut eager_ctx = TraversalContext::new(&eager_storage);
        for ty in func.param_tys().iter() {
            let ty = ty_builder
                .create_ty_with_subst(ty, func.ty_args())
                .map_err(|e| {
                    let vm_status = e
                        .finish(move_binary_format::errors::Location::Undefined)
                        .into_vm_status();
                    PartialVMError::new(vm_status.status_code())
                })?;
            // Signer params are passed through as-is in view functions (see construct_arg),
            // so skip them here — is_valid_txn_arg returns false for Signer but that is
            // intentional only for entry functions.
            if ty == Type::Signer {
                continue;
            }
            if !transaction_arg_validation::is_valid_txn_arg(
                loader,
                &mut UnmeteredGasMeter,
                &mut eager_ctx,
                &ty,
                allowed_structs,
                &mut pack_fn_cache,
            ) {
                return Err(
                    PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                        .with_message("invalid argument type for view function".to_string()),
                );
            }
        }
    }

    let result = if is_lazy {
        transaction_arg_validation::construct_args(
            session,
            loader,
            gas_meter,
            traversal_context,
            func.param_tys(),
            args,
            func.ty_args(),
            allowed_structs,
            true,
            &mut pack_fn_cache,
        )
    } else {
        let traversal_storage = TraversalStorage::new();
        transaction_arg_validation::construct_args(
            session,
            loader,
            // No metering with eager loading.
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            func.param_tys(),
            args,
            func.ty_args(),
            allowed_structs,
            true,
            &mut pack_fn_cache,
        )
    };
    result.map_err(|e| PartialVMError::new(e.status_code()))
}
