// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    access::ModuleAccess,
    errors::{PartialVMError, PartialVMResult},
    file_format::{SignatureToken, Visibility},
    CompiledModule,
};
use move_core_types::{
    ident_str,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::Function;

fn is_signer_or_signer_reference(token: &SignatureToken) -> bool {
    match token {
        SignatureToken::Signer => true,
        SignatureToken::Reference(inner) => matches!(&**inner, SignatureToken::Signer),
        _ => false,
    }
}

pub(crate) fn legacy_verify_module_init_function(module: &CompiledModule) -> PartialVMResult<()> {
    let init_func_name = ident_str!("init_module");
    let fdef_opt = module.function_defs().iter().enumerate().find(|(_, fdef)| {
        module.identifier_at(module.function_handle_at(fdef.function).name) == init_func_name
    });
    if fdef_opt.is_none() {
        return Ok(());
    }
    let (_idx, fdef) = fdef_opt.unwrap();

    if fdef.visibility != Visibility::Private {
        return Err(PartialVMError::new(StatusCode::VERIFICATION_ERROR)
            .with_message("'init_module' is not private".to_string()));
    }

    let fhandle = module.function_handle_at(fdef.function);
    let parameters = module.signature_at(fhandle.parameters);

    let return_ = module.signature_at(fhandle.return_);

    if !return_.0.is_empty() {
        return Err(PartialVMError::new(StatusCode::VERIFICATION_ERROR)
            .with_message("'init_module' should not return".to_string()));
    }

    let non_signer_tokens = parameters
        .0
        .iter()
        .any(|e| !is_signer_or_signer_reference(e));
    if non_signer_tokens {
        return Err(PartialVMError::new(StatusCode::VERIFICATION_ERROR)
            .with_message("'init_module' should not have no-signer arguments".to_string()));
    }
    Ok(())
}

/// Used for verifying an init_module function for module publishing. Used for 1.31 release and
/// above. The checks include:
///   1. Private visibility.
///   2. No return types, single signer (reference) input.
///   3. No type arguments.
pub(crate) fn verify_init_module_function(function: &Function) -> Result<(), VMStatus> {
    let err = |msg| Err(VMStatus::error(StatusCode::INVALID_INIT_MODULE, Some(msg)));

    if !function.is_private() {
        return err("init_module function must be private, but it is not".to_string());
    }

    if !function.return_tys().is_empty() {
        return err(format!(
            "init_module function must return 0 values, but returns {}",
            function.return_tys().len()
        ));
    }

    let param_tys = function.param_tys();
    if param_tys.len() != 1 {
        return err(format!(
            "init_module function should have a single signer or &signer parameter, \
             but has {} parameters",
            param_tys.len()
        ));
    }

    let arg_ty = &param_tys[0];
    if !arg_ty.is_signer_or_signer_ref() {
        return err(
            "init_module function expects a single signer or &signer parameter, \
             but its parameter type is different"
                .to_string(),
        );
    }

    if function.ty_params_count() != 0 {
        return err(format!(
            "init_module function expects 0 type parameters, but has {} type parameters",
            function.ty_params_count()
        ));
    }

    Ok(())
}
