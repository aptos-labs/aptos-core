// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::SessionExt;
use aptos_framework::RuntimeModuleMetadataV1;
use move_binary_format::{
    access::ModuleAccess,
    errors::{Location, PartialVMError, VMError, VMResult},
    file_format::{
        Bytecode,
        SignatureToken::{Struct, StructInstantiation},
    },
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, language_storage::ModuleId, vm_status::StatusCode,
};
use std::collections::HashSet;

const EVENT_MODULE_NAME: &str = "event";
const EVENT_EMIT_FUNCTION_NAME: &str = "emit";

fn metadata_validation_err(msg: &str) -> Result<(), VMError> {
    Err(metadata_validation_error(msg))
}

fn metadata_validation_error(msg: &str) -> VMError {
    PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
        .with_message(format!("metadata and code bundle mismatch: {}", msg))
        .finish(Location::Undefined)
}

/// Validate resource group metadata on modules one by one:
/// * Extract the event metadata
/// * Verify all changes are compatible upgrades (existing event attributes cannot be removed)
pub(crate) fn validate_module_events(
    session: &mut SessionExt,
    modules: &[CompiledModule],
) -> VMResult<()> {
    for module in modules {
        let mut new_event_structs =
            if let Some(metadata) = aptos_framework::get_metadata_from_compiled_module(module) {
                extract_event_metadata(&metadata)?
            } else {
                HashSet::new()
            };

        // Check all the emit calls have the correct struct with event attribute.
        validate_emit_calls(&new_event_structs, module)?;

        let original_event_structs =
            extract_event_metadata_from_module(session, &module.self_id())?;

        for member in original_event_structs {
            // We don't need to re-validate new_members above.
            if !new_event_structs.remove(&member) {
                metadata_validation_err("Invalid change in event attributes")?;
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
                    if module_addr
                        != &AccountAddress::from_hex_literal("0x1").expect("valid address")
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
                                metadata_validation_err(format!("#[event] attribute is missing for {} passed to 0x1::event::emit function", struct_name).as_str())
                            } else {
                                Ok(())
                            }
                        },
                        _ => metadata_validation_err(
                            "Reference to non-struct parameter for 0x1::event::emit function",
                        ),
                    }?;
                }
            }
        }
    }
    Ok(())
}

/// Given a module id extract all event metadata
pub(crate) fn extract_event_metadata_from_module(
    session: &mut SessionExt,
    module_id: &ModuleId,
) -> VMResult<HashSet<String>> {
    let metadata = session.load_module(module_id).map(|module| {
        CompiledModule::deserialize(&module)
            .map(|module| aptos_framework::get_metadata_from_compiled_module(&module))
    });

    if let Ok(Ok(Some(metadata))) = metadata {
        extract_event_metadata(&metadata)
    } else {
        Ok(HashSet::new())
    }
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
