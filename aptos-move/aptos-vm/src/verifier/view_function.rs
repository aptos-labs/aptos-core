// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{MoveResolverExt, SessionExt},
    verifier::transaction_arg_validation,
};
use aptos_framework::RuntimeModuleMetadataV1;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{identifier::IdentStr, vm_status::StatusCode};
use move_vm_runtime::session::LoadedFunctionInstantiation;
use move_vm_types::loaded_data::runtime_types::Type;

/// Validate view function call. This checks whether the function is marked as a view
/// function, and validates the arguments.
pub(crate) fn validate_view_function<S: MoveResolverExt>(
    session: &SessionExt<S>,
    args: Vec<Vec<u8>>,
    fun_name: &IdentStr,
    fun_inst: &LoadedFunctionInstantiation,
    module_metadata: Option<&RuntimeModuleMetadataV1>,
) -> PartialVMResult<Vec<Vec<u8>>> {
    // Must be marked as view function
    let is_view = if let Some(data) = module_metadata {
        data.fun_attributes
            .get(fun_name.as_str())
            .map(|attrs| attrs.iter().any(|attr| attr.is_view_function()))
            .unwrap_or_default()
    } else {
        false
    };
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

    // Validate arguments. We allow all what transaction allows, in addition, signers can
    // be passed. Some arguments (e.g. utf8 strings) need validation which happens here.
    let mut needs_validation = vec![];
    for (idx, ty) in fun_inst.parameters.iter().enumerate() {
        match ty {
            Type::Signer => continue,
            Type::Reference(inner_type) if matches!(&**inner_type, Type::Signer) => continue,
            _ => {
                let (valid, validation) = transaction_arg_validation::is_valid_txn_arg(session, ty);
                if !valid {
                    return Err(
                        PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                            .with_message("invalid view function argument".to_string()),
                    );
                }
                if validation {
                    needs_validation.push(idx);
                }
            },
        }
    }
    if !needs_validation.is_empty()
        && transaction_arg_validation::validate_args(session, &needs_validation, &args, fun_inst)
            .is_err()
    {
        return Err(
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                .with_message("invalid view function argument: failed validation".to_string()),
        );
    }
    Ok(args)
}
