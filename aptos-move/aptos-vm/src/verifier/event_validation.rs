// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    on_chain_config::Features,
    vm::module_metadata::{get_metadata_from_compiled_code, RuntimeModuleMetadataV1},
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{Location, PartialVMError, VMError, VMResult},
    file_format::{
        Bytecode, CompiledScript,
        SignatureToken::{Struct, StructInstantiation},
    },
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
use move_vm_runtime::{module_traversal::TraversalContext, ModuleStorage};
use std::collections::HashSet;

const EVENT_MODULE_NAME: &str = "event";
const EVENT_EMIT_FUNCTION_NAME: &str = "emit";

fn metadata_validation_err(msg: &str) -> Result<(), VMError> {
    Err(metadata_validation_error(msg))
}

fn metadata_validation_error(msg: &str) -> VMError {
    PartialVMError::new(StatusCode::EVENT_METADATA_VALIDATION_ERROR)
        .with_message(format!("metadata and code bundle mismatch: {}", msg))
        .finish(Location::Undefined)
}

/// Validate event metadata on modules one by one:
/// * Extract the event metadata
/// * Verify all changes are compatible upgrades (existing event attributes cannot be removed)
pub(crate) fn validate_module_events(
    _features: &Features,
    _gas_feature_version: u64,
    module_storage: &impl ModuleStorage,
    // TODO(lazy-loading): add a check that the old module has been visited.
    _traversal_context: &TraversalContext,
    new_modules: &[CompiledModule],
) -> VMResult<()> {
    for new_module in new_modules {
        let mut new_event_structs = get_metadata_from_compiled_code(new_module).map_or_else(
            || Ok(HashSet::new()),
            |metadata| extract_event_metadata(&metadata),
        )?;

        // Check all the emit calls have the correct struct with event attribute.
        validate_emit_calls(&new_event_structs, new_module)?;

        let old_module_metadata_if_exists = module_storage
            .fetch_deserialized_module(new_module.address(), new_module.name())?
            .and_then(|module| {
                // TODO(loader_v2): We can optimize this to fetch metadata directly.
                get_metadata_from_compiled_code(module.as_ref())
            });
        if let Some(metadata) = old_module_metadata_if_exists {
            let original_event_structs = extract_event_metadata(&metadata)?;
            for member in original_event_structs {
                // Fail if we see a removal of an event attribute.
                if !new_event_structs.remove(&member) {
                    metadata_validation_err("Invalid change in event attributes")?;
                }
            }
        }
    }
    Ok(())
}

/// Validate all the `0x1::event::emit` calls have the struct defined in the same module with event
/// attribute.
pub(crate) fn validate_emit_calls(
    event_structs: &HashSet<String>,
    module: &CompiledModule,
) -> VMResult<()> {
    for fun in module.function_defs() {
        if let Some(code_unit) = &fun.code {
            for bc in &code_unit.code {
                if let Bytecode::CallGeneric(index) = bc {
                    let func_instantiation = &module.function_instantiation_at(*index);
                    let func_handle = module.function_handle_at(func_instantiation.handle);
                    let module_handle = module.module_handle_at(func_handle.module);
                    let module_addr = module.address_identifier_at(module_handle.address);
                    let module_name = module.identifier_at(module_handle.name);
                    let func_name = module.identifier_at(func_handle.name);
                    if module_addr != &AccountAddress::ONE
                        || module_name.as_str() != EVENT_MODULE_NAME
                        || func_name.as_str() != EVENT_EMIT_FUNCTION_NAME
                    {
                        continue;
                    }
                    let param = module
                        .signature_at(func_instantiation.type_parameters)
                        .0
                        .first()
                        .ok_or_else(|| {
                            metadata_validation_error(
                                "Missing parameter for 0x1::event::emit function",
                            )
                        })?;
                    match param {
                        StructInstantiation(index, _) | Struct(index) => {
                            let struct_handle = &module.struct_handle_at(*index);
                            let struct_name = module.identifier_at(struct_handle.name);
                            if struct_handle.module != module.self_handle_idx() {
                                metadata_validation_err(format!("{} passed to 0x1::event::emit function is not defined in the same module", struct_name).as_str())
                            } else if !event_structs.contains(struct_name.as_str()) {
                                metadata_validation_err(format!("Missing #[event] attribute on {}. The #[event] attribute is required for all structs passed into 0x1::event::emit.", struct_name).as_str())
                            } else {
                                Ok(())
                            }
                        },
                        _ => metadata_validation_err(
                            "Passed in a non-struct parameter into 0x1::event::emit.",
                        ),
                    }?;
                }
            }
        }
    }
    Ok(())
}

/// Given a module id extract all event metadata
pub(crate) fn extract_event_metadata(
    metadata: &RuntimeModuleMetadataV1,
) -> VMResult<HashSet<String>> {
    let mut event_structs = HashSet::new();
    for (struct_, attrs) in &metadata.struct_attributes {
        for attr in attrs {
            if attr.is_event() && !event_structs.insert(struct_.clone()) {
                metadata_validation_err("Found duplicate event attribute")?;
            }
        }
    }
    Ok(event_structs)
}

pub(crate) fn verify_no_event_emission_in_compiled_script(script: &CompiledScript) -> VMResult<()> {
    for bc in &script.code().code {
        if let Bytecode::CallGeneric(index) = bc {
            let func_instantiation = &script.function_instantiation_at(*index);
            let func_handle = script.function_handle_at(func_instantiation.handle);
            let module_handle = script.module_handle_at(func_handle.module);
            let module_addr = script.address_identifier_at(module_handle.address);
            let module_name = script.identifier_at(module_handle.name);
            let func_name = script.identifier_at(func_handle.name);
            if module_addr == &AccountAddress::ONE
                && module_name.as_str() == EVENT_MODULE_NAME
                && func_name.as_str() == EVENT_EMIT_FUNCTION_NAME
            {
                return Err(PartialVMError::new(StatusCode::INVALID_OPERATION_IN_SCRIPT)
                    .finish(Location::Script));
            }
        }
    }
    Ok(())
}
