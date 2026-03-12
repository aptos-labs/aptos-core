// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module validates function-level attributes that impose restrictions
//! independent of the struct or constant APIs.
//!
//! ## `#[immutable]` transitive call check
//!
//! An `#[immutable]` function guarantees stable behavior across module upgrades.
//! To uphold this guarantee, it must only call or capture (via closures):
//! - Other `#[immutable]` functions (same- or cross-module), OR
//! - Native functions (whose behavior changes, if ever, are gated by on-chain governance).
//!
//! This applies to `Call`, `CallGeneric`, `PackClosure`, and `PackClosureGeneric` instructions.
//! Cross-module callees are checked by inspecting the `FunctionHandle.attributes` field.
//! Same-module callees that have no code body are native and are always permitted.

use move_binary_format::{
    access::ModuleAccess,
    errors::{PartialVMError, PartialVMResult},
    file_format::{Bytecode, CompiledModule, FunctionAttribute, FunctionDefinition},
};
use move_core_types::vm_status::StatusCode;

/// Check that every `Call`/`CallGeneric`/`PackClosure`/`PackClosureGeneric` instruction
/// inside an `#[immutable]` function targets a callee that is itself `#[immutable]` or native.
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

    let self_handle_idx = module.self_handle_idx();

    for instr in &code.code {
        let callee_handle_idx = match instr {
            Bytecode::Call(idx) => *idx,
            Bytecode::CallGeneric(inst_idx) => module.function_instantiation_at(*inst_idx).handle,
            // Closure creation captures a function reference; the captured function must
            // also be #[immutable] or native to preserve the immutability guarantee.
            Bytecode::PackClosure(idx, _) => *idx,
            Bytecode::PackClosureGeneric(inst_idx, _) => {
                module.function_instantiation_at(*inst_idx).handle
            },
            _ => continue,
        };

        let callee_handle = module.function_handle_at(callee_handle_idx);

        // If the callee is `#[immutable]`, it satisfies the requirement.
        if callee_handle
            .attributes
            .contains(&FunctionAttribute::Immutable)
        {
            continue;
        }

        // If the callee is a same-module native function, it is also allowed.
        if callee_handle.module == self_handle_idx {
            let is_native = module
                .function_defs()
                .iter()
                .find(|def| def.function == callee_handle_idx)
                .map(|def| def.code.is_none())
                .unwrap_or(false);
            if is_native {
                continue;
            }
        }

        // Otherwise the callee violates the immutability contract.
        return Err(
            PartialVMError::new(StatusCode::IMMUTABLE_FUNCTION_CALLS_NON_IMMUTABLE).with_message(
                format!(
                    "#[immutable] function calls a callee (handle {}) that is neither \
                     #[immutable] nor native",
                    callee_handle_idx.0
                ),
            ),
        );
    }

    Ok(())
}
