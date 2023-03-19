// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::io::Cursor;
use crate::{
    move_vm_ext::{MoveResolverExt, SessionExt},
    verifier::transaction_arg_validation,
};
use aptos_framework::RuntimeModuleMetadataV1;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{identifier::IdentStr, vm_status::StatusCode};
use move_vm_runtime::session::LoadedFunctionInstantiation;
use move_vm_types::gas::GasMeter;

/// Validate view function call. This checks whether the function is marked as a view
/// function, and validates the arguments.
pub(crate) fn validate_view_function<S: MoveResolverExt>(
    session: &mut SessionExt<S>,
    mut args: Vec<Vec<u8>>,
    fun_name: &IdentStr,
    fun_inst: &LoadedFunctionInstantiation,
    module_metadata: Option<&RuntimeModuleMetadataV1>,
    gas_meter: &mut impl GasMeter
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

    for (idx, ty) in fun_inst.parameters.iter().enumerate() {
        let (valid, needs_construction) = transaction_arg_validation::is_valid_txn_arg(session, ty);
        if !valid {
            return Err(PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                           .with_message("invalid view function argument".to_string()));
        }
        if needs_construction {
            let mut cursor = Cursor::new(&args[idx][..]);
            let mut new_arg = vec![];
            transaction_arg_validation::recursively_construct_arg(session, ty,&mut cursor, gas_meter, &mut new_arg)
                .map_err(|_| PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                    .with_message("invalid view function argument".to_string()))?;
            args[idx] = new_arg;
            // Check cursor has parsed everything
        }
    }

    Ok(args)
}
