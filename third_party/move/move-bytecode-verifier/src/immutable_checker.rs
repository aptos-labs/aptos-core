// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module validates function-level attributes that impose restrictions
//! independent of the struct or constant APIs.
//!
//! ## `#[immutable]` transitive call check
//!
//! An `#[immutable]` function guarantees stable behavior across module upgrades.
//! To uphold this guarantee, every callee (same- or cross-module) reached via
//! `Call`, `CallGeneric`, `PackClosure`, or `PackClosureGeneric` must itself
//! carry the `#[immutable]` attribute.  Callees without `#[immutable]` —
//! including native functions — are rejected.

use move_binary_format::{
    access::ModuleAccess,
    errors::{PartialVMError, PartialVMResult},
    file_format::{Bytecode, CompiledModule, FunctionAttribute, FunctionDefinition},
};
use move_core_types::vm_status::StatusCode;

/// Check that every `Call`/`CallGeneric`/`PackClosure`/`PackClosureGeneric` instruction
/// inside an `#[immutable]` function targets a callee that itself carries `#[immutable]`.
pub fn check_immutable_transitive_calls(
    module: &CompiledModule,
    function_definition: &FunctionDefinition,
) -> PartialVMResult<()> {
    let handle = module.function_handle_at(function_definition.function);

    // Only applies to functions carrying the Immutable attribute.
    if !handle.attributes.contains(&FunctionAttribute::Immutable) {
        return Ok(());
    }

    // Native functions have no body; nothing to check.
    let code = match &function_definition.code {
        Some(c) => c,
        None => return Ok(()),
    };

    for instr in &code.code {
        let callee_handle_idx = match instr {
            Bytecode::Call(idx) => *idx,
            Bytecode::CallGeneric(inst_idx) => module.function_instantiation_at(*inst_idx).handle,
            // Closure creation captures a function reference; the captured function must
            // also carry #[immutable] to preserve the immutability guarantee.
            Bytecode::PackClosure(idx, _) => *idx,
            Bytecode::PackClosureGeneric(inst_idx, _) => {
                module.function_instantiation_at(*inst_idx).handle
            },
            _ => continue,
        };

        let callee_handle = module.function_handle_at(callee_handle_idx);

        // Only callees that carry `#[immutable]` are permitted.
        if callee_handle
            .attributes
            .contains(&FunctionAttribute::Immutable)
        {
            continue;
        }

        // All other callees — including native functions — violate the contract.
        let callee_module =
            module.address_identifier_at(module.module_handle_at(callee_handle.module).address);
        let callee_module_name =
            module.identifier_at(module.module_handle_at(callee_handle.module).name);
        let callee_name = module.identifier_at(callee_handle.name);
        return Err(
            PartialVMError::new(StatusCode::IMMUTABLE_FUNCTION_CALLS_NON_IMMUTABLE).with_message(
                format!(
                    "#[immutable] function calls `{}::{}::{}` which is not #[immutable]",
                    callee_module, callee_module_name, callee_name,
                ),
            ),
        );
    }

    Ok(())
}
