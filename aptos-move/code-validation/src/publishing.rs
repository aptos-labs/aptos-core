// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::validation::{bytecode, events, natives, resource_groups};
use aptos_types::{
    on_chain_config::Features, vm::module_metadata::verify_module_metadata_for_module_publishing,
};
use move_binary_format::{
    access::ModuleAccess,
    errors::{Location, PartialVMError, VMError, VMResult},
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
use move_vm_runtime::{module_traversal::TraversalContext, ModuleStorage};
use move_vm_types::gas::GasMeter;
use std::collections::{BTreeMap, BTreeSet};

fn metadata_validation_error(msg: &str) -> VMError {
    PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
        .with_message(format!("metadata and code bundle mismatch: {}", msg))
        .finish(Location::Undefined)
}

/// Validate a publish request.
pub fn validate_publish_request(
    features: &Features,
    is_mainnet: bool,
    reject_legacy_bytecode: bool,
    module_storage: &impl ModuleStorage,
    traversal_context: &mut TraversalContext,
    gas_meter: &mut impl GasMeter,
    modules: &[CompiledModule],
    mut expected_modules: BTreeSet<String>,
    allowed_deps: Option<BTreeMap<AccountAddress, BTreeSet<String>>>,
) -> VMResult<()> {
    bytecode::reject_unstable_bytecode(is_mainnet, modules)?;
    bytecode::reject_legacy_module_bytecode(reject_legacy_bytecode, modules)?;
    natives::validate_module_natives(modules)?;

    for m in modules {
        if !expected_modules.remove(m.self_id().name().as_str()) {
            return Err(metadata_validation_error(&format!(
                "unregistered module: '{}'",
                m.self_id().name()
            )));
        }
        if let Some(allowed) = &allowed_deps {
            for dep in m.immediate_dependencies() {
                if !allowed
                    .get(dep.address())
                    .map(|modules| modules.contains("") || modules.contains(dep.name().as_str()))
                    .unwrap_or(false)
                {
                    return Err(metadata_validation_error(&format!(
                        "unregistered dependency: '{}'",
                        dep
                    )));
                }
            }
        }
        verify_module_metadata_for_module_publishing(m, features)
            .map_err(|err| metadata_validation_error(&err.to_string()))?;
    }

    resource_groups::validate_resource_groups(
        features,
        module_storage,
        traversal_context,
        gas_meter,
        modules,
    )?;
    events::validate_module_events(features, module_storage, traversal_context, modules)?;

    if !expected_modules.is_empty() {
        return Err(metadata_validation_error(
            "not all registered modules published",
        ));
    }
    Ok(())
}
