// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{MoveResolverExt, SessionExt},
    verifier::{transaction_arg_validation, transaction_arg_validation::get_allowed_structs},
};
use aptos_framework::RuntimeModuleMetadataV1;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{identifier::IdentStr, vm_status::StatusCode};
use move_vm_runtime::session::LoadedFunctionInstantiation;
use move_vm_types::loaded_data::runtime_types::Type;

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
pub(crate) fn validate_view_function<S: MoveResolverExt>(
    session: &mut SessionExt<S>,
    mut args: Vec<Vec<u8>>,
    fun_name: &IdentStr,
    fun_inst: &LoadedFunctionInstantiation,
    module_metadata: Option<&RuntimeModuleMetadataV1>,
    struct_constructors_feature: bool,
) -> PartialVMResult<Vec<Vec<u8>>> {
    // Must be marked as view function
    let is_view = determine_is_view(module_metadata, fun_name);
    if !is_view {
        return Err(
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                .with_message("function not marked as view function".to_string()),
        );
    }

    // Must return values
    if fun_inst.return_.is_empty() {
        return Err(
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                .with_message("view function must return values".to_string()),
        );
    }

    let allowed_structs = get_allowed_structs(struct_constructors_feature);
    // Validate arguments. We allow all what transaction allows, in addition, signers can
    // be passed. Some arguments (e.g. utf8 strings) need validation which happens here.
    let mut needs_construction = vec![];
    for (idx, ty) in fun_inst.parameters.iter().enumerate() {
        match ty {
            Type::Signer => continue,
            Type::Reference(inner_type) if matches!(&**inner_type, Type::Signer) => continue,
            _ => {
                let (valid, construction) =
                    transaction_arg_validation::is_valid_txn_arg(session, ty, allowed_structs);
                if !valid {
                    return Err(
                        PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                            .with_message("invalid view function argument".to_string()),
                    );
                }
                if construction {
                    needs_construction.push(idx);
                }
            },
        }
    }
    if !needs_construction.is_empty()
        && transaction_arg_validation::construct_args(
            session,
            &needs_construction,
            &mut args,
            fun_inst,
            allowed_structs,
        )
        .is_err()
    {
        return Err(
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                .with_message("invalid view function argument: failed validation".to_string()),
        );
    }

    Ok(args)
}
